//! Cache applicatif — port de `apps/api/src/cache/index.ts`.
//!
//! L1 mémoire = [`moka`] avec TTL PAR ENTRÉE (`cacheSeconds` pour les SVG,
//! `CACHE_TTL` pour les données) et capacité NON BORNÉE — même sémantique que
//! le store par défaut de `cacheable` v2.3.4 (Keyv `KeyvCacheableMemory` :
//! Map non bornée, TTL par entrée ; vérifié en lecture seule dans
//! node_modules). L2 = [`L2Cache`], trait avec implémentation no-op
//! [`NoopL2`] en phase 4 — Redis + enveloppe Keyv = phase 4b (le brief
//! exclut Redis ici) ; le backfill L1←L2 avec le TTL RESTANT de l'enveloppe
//! Keyv sera implémenté à ce moment (le trait renvoie déjà la valeur, l4b
//! ajoutera la portée du TTL).
//!
//! Compteurs hit/miss = port de `cacheCounters` (metrics/store.ts) : incrément
//! PAR GET résolu (hit L1 OU L2 → hits, sinon misses), atomiques, exposés pour
//! la phase 5 (aucune route ne les expose — comme le legacy, où le store de
//! metrics n'est relié à aucune route).
//!
//! `flush()` : appelé au boot fire-and-forget dans le legacy (`index.ts:20-22`,
//! erreur loggée « Failed to flush cache on startup: ») — le port appelle
//! l'équivalent sans effet observable (no-op L2).

use moka::Expiry;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// L2 injectable (Redis phase 4b ; no-op en phase 4). Les futures sont
/// `+ Send` explicites : les routes axum exigent des handlers Send, et la
/// preuve générique ne passe pas par les `async fn` de trait nus (RPITIT).
pub trait L2Cache: Send + Sync + 'static {
    fn get(&self, key: &str) -> impl std::future::Future<Output = Option<String>> + Send;
    fn set(
        &self,
        key: String,
        value: String,
        ttl_secs: f64,
    ) -> impl std::future::Future<Output = ()> + Send;
    fn flush(&self) -> impl std::future::Future<Output = ()> + Send;
}

/// Implémentation no-op (phase 4 sans `REDIS_URL` — le legacy construit
/// `Cacheable` sans `secondary` dans ce cas).
pub struct NoopL2;

impl L2Cache for NoopL2 {
    async fn get(&self, _key: &str) -> Option<String> {
        None
    }
    async fn set(&self, _key: String, _value: String, _ttl_secs: f64) {}
    async fn flush(&self) {}
}

#[derive(Clone)]
struct CachedValue {
    data: String,
    ttl_secs: f64,
}

/// TTL par entrée (`Cacheable.set(key, value, ttlSeconds * 1000)`).
struct PerEntryTtl;

impl Expiry<String, CachedValue> for PerEntryTtl {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &CachedValue,
        _now: Instant,
    ) -> Option<Duration> {
        Some(duration_from_secs_f64_lossy(value.ttl_secs))
    }
}

/// `cacheSeconds: Infinity` (query `cache_seconds=Infinity`) → jamais expiré :
/// `Duration::MAX` (le Map du legacy garde l'entrée sans expiration). Valeurs
/// finies bornées à des secondes entières (l'unité du legacy).
fn duration_from_secs_f64_lossy(secs: f64) -> Duration {
    if secs.is_infinite() || secs > 2.0e53 {
        return Duration::MAX;
    }
    if !(secs.is_finite()) || secs <= 0.0 {
        return Duration::from_secs(0);
    }
    Duration::from_secs(secs as u64)
}

/// Cache 2 couches (L1 moka + L2 générique), compteurs atomiques.
pub struct AppCache<L2: L2Cache> {
    l1: moka::future::Cache<String, CachedValue>,
    l2: L2,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl<L2: L2Cache> AppCache<L2> {
    /// `createCache(redisUrl)` — capacité NON BORNÉE (parité KeyvCacheableMemory).
    pub fn new(l2: L2) -> Self {
        let l1 = moka::future::Cache::builder()
            .max_capacity(u64::MAX)
            .expire_after(PerEntryTtl)
            .build();
        Self {
            l1,
            l2,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// `cache.get(key)` : L1 puis L2 (comme `Cacheable.get`), compteur
    /// hit/miss PAR GET résolu. En phase 4 le L2 est no-op : aucun backfill
    /// observable (phase 4b : backfill avec le TTL restant de l'enveloppe Keyv).
    pub async fn get(&self, key: &str) -> Option<String> {
        let result = match self.l1.get(key).await {
            Some(v) => Some(v.data),
            None => self.l2.get(key).await,
        };
        if result.is_some() {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    /// `cache.set(key, value, ttlSeconds)` : L1 (TTL par entrée) puis L2
    /// (`nonBlocking: true` du legacy → propagation best-effort ; aucune
    /// différence observable hors panne L2).
    pub async fn set(&self, key: &str, value: String, ttl_secs: f64) {
        self.l1
            .insert(
                key.to_string(),
                CachedValue {
                    data: value.clone(),
                    ttl_secs,
                },
            )
            .await;
        self.l2.set(key.to_string(), value, ttl_secs).await;
    }

    /// `cache.clear()` (flush au boot, `index.ts:20-22`).
    pub async fn flush(&self) {
        self.l1.invalidate_all();
        self.l2.flush().await;
    }

    /// Compteurs (phase 5 : branchés sur le store de metrics, aucune route).
    pub fn counters(&self) -> (u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }
}
