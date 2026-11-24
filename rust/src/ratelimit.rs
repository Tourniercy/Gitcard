//! Rate-limiter `/api/data/*` — port exact de `hono-rate-limiter` 0.5.3 tel
//! qu'instancié par le legacy (`apps/api/src/routes/data.ts:20-27`) :
//! `windowMs: 60_000`, `limit: 30`, `keyGenerator` = `cf-connecting-ip`
//! sinon `getConnInfo().remote.address` (catch → `'unknown'`), store
//! `MemoryStore` (défaut), `standardHeaders: 'draft-6'` (défaut).
//!
//! Sémantique prouvée PAR EXÉCUTION du legacy (oracle
//! `tests/golden/ratelimit/`, Node 26.13.1 hors réseau) :
//! - fenêtre FIXE par clé : `resetTime = 1er hit + 60 s`
//!   (`MemoryStore.increment` : `resetClient` quand `resetTime <= now`,
//!   reset 60 → 59 à t0+1,25 s → reset total à t0+61,5 s, boot `reset_61s`) ;
//! - en-têtes draft-6 sur TOUTE réponse du chemin (y compris 404 fallback et
//!   POST non matchés — boot `paths` : `/api/data` nu EST matché par le glob
//!   `/api/data/*` du legacy), dans l'ordre d'émission hono via
//!   @hono/node-server : alphabétique (`ratelimit-limit`, `ratelimit-policy`,
//!   `ratelimit-remaining`, `ratelimit-reset`), `ratelimit-reset` ABSENT si 0
//!   (`if (resetSeconds)`) ;
//! - `Retry-After` UNIQUEMENT sur la 429 (`setRetryAfterHeader`), secondes
//!   entières `Math.ceil` bornées à 0 ;
//! - 429 hono : texte `Too many requests, please try again later.`
//!   (`text/plain; charset=UTF-8`, 42 octets) ;
//! - `remaining = max(limit - totalHits, 0)` — 404/500 COMPTENT
//!   (`skipFailedRequests`/`skipSuccessfulRequests` = false, boot
//!   `count_404` : 30 × 404 → 31e = 429) ;
//! - clés indépendantes (`cf-connecting-ip: 9.9.9.9` vs remote, boot `keys`).
//!
//! Le `clearExpired` (setInterval MemoryStore, recyclage mémoire current →
//! previous) n'a AUCUN effet observable sur hit/resetTime (get/increment
//! recomposent depuis les deux maps) : le port reproduit la sémantique
//! observable (deux maps, même logique getClient) sans timer.
//!
//! L'horloge est injectable (`now_ms`) : les tests d'oracle avancent le temps
//! virtuellement (aucune attente réelle côté Rust ; l'attente 61 s réelle
//! n'est faite qu'UNE fois côté legacy).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const WINDOW_MS: u64 = 60_000;
pub const LIMIT: u64 = 30;
/// Message 429 hono (`context.text(message)` du handler par défaut).
pub const BLOCKED_MESSAGE: &str = "Too many requests, please try again later.";

/// Entrée par clé — `Client` hono (`totalHits`, `resetTime`).
#[derive(Clone, Copy, Debug)]
struct Client {
    total_hits: u64,
    reset_time_ms: u64,
}

/// `MemoryStore` : deux maps (current / previous, recyclées par le timer
/// legacy) — la même logique `getClient` (current ?? previous ?? nouveau).
#[derive(Default)]
struct Store {
    current: HashMap<String, Client>,
    previous: HashMap<String, Client>,
}

/// Limiteur partagé par l'app (Arc derrière `AppState`).
pub struct RateLimiter {
    store: Mutex<Store>,
    now_ms: Arc<dyn Fn() -> u64 + Send + Sync>,
}

impl RateLimiter {
    /// `now_ms` : horloge injectable (prod = epoch ms réelle ; tests =
    /// cellule virtuelle — cf. `tests/p5_ratelimit.rs`).
    pub fn new(now_ms: Arc<dyn Fn() -> u64 + Send + Sync>) -> Self {
        Self {
            store: Mutex::new(Store::default()),
            now_ms,
        }
    }

    pub fn now(&self) -> u64 {
        (self.now_ms)()
    }

    /// `store.increment(key)` hono : `getClient` → reset si fenêtre échue →
    /// `totalHits++`. Retourne `(totalHits, resetTimeMs)`.
    pub fn increment(&self, key: &str) -> (u64, u64) {
        let now = self.now();
        let mut store = self.store.lock().expect("rate-limiter poison");
        let client = store
            .current
            .get(key)
            .copied()
            .or_else(|| store.previous.remove(key))
            .unwrap_or_else(|| Client {
                total_hits: 0,
                reset_time_ms: now + WINDOW_MS,
            });
        let mut client = client;
        if client.reset_time_ms <= now {
            client.total_hits = 0;
            client.reset_time_ms = now + WINDOW_MS;
        }
        client.total_hits += 1;
        store.current.insert(key.to_string(), client);
        (client.total_hits, client.reset_time_ms)
    }
}

/// `getResetSeconds` hono : `Math.ceil((resetTime - Date.now()) / 1e3)`,
/// borné à 0 (Math.ceil(-0,001) = -0 → 0). `i64::div_ceil` est instable sur
/// cette toolchain — ceil entier manuel pour diff > 0.
pub fn reset_seconds(reset_time_ms: u64, now_ms: u64) -> u64 {
    let diff = reset_time_ms as i64 - now_ms as i64;
    if diff <= 0 {
        0
    } else {
        ((diff + 999) / 1000) as u64
    }
}

/// Info d'une requête passée par le limiteur (avant decision) — calculée par
/// le middleware de `app.rs` pour construire les en-têtes draft-6.
pub struct RequestInfo {
    pub total_hits: u64,
    pub remaining: u64,
    pub reset_seconds: u64,
}

impl RateLimiter {
    /// `increment` + calcul `info` hono (`remaining = max(limit-totalHits,0)`,
    /// `getResetSeconds(info.resetTime)`).
    pub fn check(&self, key: &str) -> RequestInfo {
        let (total_hits, reset_time_ms) = self.increment(key);
        RequestInfo {
            total_hits,
            remaining: LIMIT.saturating_sub(total_hits),
            reset_seconds: reset_seconds(reset_time_ms, self.now()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn virtual_clock() -> (Arc<AtomicU64>, Arc<dyn Fn() -> u64 + Send + Sync>) {
        let cell = Arc::new(AtomicU64::new(1_000_000));
        let read = {
            let cell = cell.clone();
            Arc::new(move || cell.load(Ordering::SeqCst)) as Arc<dyn Fn() -> u64 + Send + Sync>
        };
        (cell, read)
    }

    #[test]
    fn fenetre_fixe_30_31_puis_reset_au_1er_hit_plus_60s() {
        let (cell, now) = virtual_clock();
        let rl = RateLimiter::new(now);
        for i in 1..=30 {
            let info = rl.check("1.2.3.4");
            assert_eq!(info.total_hits, i);
            assert_eq!(info.remaining, 30 - i);
            assert_eq!(info.reset_seconds, 60);
            assert!(info.total_hits <= LIMIT, "30 premiers hits non bloqués");
        }
        // 31e : totalHits > limit, remaining borné à 0
        let info = rl.check("1.2.3.4");
        assert_eq!(info.total_hits, 31);
        assert_eq!(info.remaining, 0);
        assert!(info.total_hits > LIMIT, "31e hit au-dessus de la limite");
        // clés indépendantes
        let info = rl.check("9.9.9.9");
        assert_eq!(info.total_hits, 1);
        assert_eq!(info.remaining, 29);
        // fenêtre FIXE : reset à 1er hit + 60 s (pas glissant)
        cell.store(1_000_000 + 60_000, Ordering::SeqCst);
        let info = rl.check("1.2.3.4");
        assert_eq!(info.total_hits, 1, "compteur remis à zéro après la fenêtre");
        assert_eq!(info.remaining, 29);
        assert_eq!(
            info.reset_seconds, 60,
            "nouvelle fenêtre ancrée au hit courant"
        );
    }

    #[test]
    fn reset_seconds_semantique_math_ceil_js() {
        // Math.ceil((t - now)/1000) : 59_999 ms restants → 60 ; 1 ms → 1 ;
        // 0 → 0 ; négatif (fenêtre échue avant reset) → borné à 0.
        assert_eq!(reset_seconds(60_000, 1), 60);
        assert_eq!(reset_seconds(60_000, 59_999), 1);
        assert_eq!(reset_seconds(60_000, 60_000), 0);
        assert_eq!(
            reset_seconds(60_000, 60_001),
            0,
            "Math.ceil(-0.001) = -0 → 0"
        );
        assert_eq!(reset_seconds(60_000, 61_500), 0);
    }

    #[test]
    fn previous_recycle_comme_memorystore() {
        // Un client passé en `previous` (recyclage du timer legacy) reste vu
        // par get/increment — le port déplace previous → current au premier
        // hit. Simulation : hit → avance > fenêtre → hit (reset) → la clé vit
        // dans current ; un NOUVEAU hit re-trouve le client (pas de re-init).
        let (cell, now) = virtual_clock();
        let rl = RateLimiter::new(now);
        assert_eq!(rl.check("k").total_hits, 1);
        cell.store(1_000_000 + 30_000, Ordering::SeqCst);
        assert_eq!(
            rl.check("k").total_hits,
            2,
            "même fenêtre : compteur cumulé"
        );
        cell.store(1_000_000 + 90_000, Ordering::SeqCst);
        assert_eq!(rl.check("k").total_hits, 1, "fenêtre échue : reset");
        assert_eq!(
            rl.check("k").total_hits,
            2,
            "le client survit (pas de re-création)"
        );
    }
}
