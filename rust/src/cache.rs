//! Cache applicatif — port de `apps/api/src/cache/index.ts` (phase 4b).
//!
//! L1 mémoire = [`moka`] avec TTL PAR ENTRÉE (`cacheSeconds` pour les SVG,
//! `CACHE_TTL` pour les données) et capacité NON BORNÉE — même sémantique que
//! le store par défaut de `cacheable` v2.3.4 (Keyv `KeyvCacheableMemory` :
//! Map non bornée, TTL par entrée ; vérifié en lecture seule dans
//! node_modules).
//!
//! L2 = [`RedisL2`] (REDIS_URL présent) ou [`NoopL2`] (absent — phase 4).
//! ORACLE `tests/golden/redis/` (legacy Node 26.13.1 exécuté, faux serveur
//! RESP) — tout est constaté, pas deviné :
//! - le legacy choisit `cacheable` avec `secondary = new KeyvRedis(url)` :
//!   L1 mémoire RESTE DEVANT Redis (`l1_plus_l2`) — preuve : 2 requêtes
//!   identiques → 2 GET Redis puis 0 (segments a_req1*/a_req2_all) ;
//! - clés = `keyv::keyv:<clé logique>` (namespace KeyvRedis `::` + préfixe
//!   Keyv `keyv:`) — ex. `keyv::keyv:svg:stats:probe-user:289b4511` ;
//! - enveloppe = `{"value":"…","expires":<epoch ms>}` (sérialisation
//!   @keyv/serialize : chaîne commençant par `:` → préfixe `:` supplémentaire ;
//!   TTL nul → `expires` omise) ;
//! - écriture = `SET <clé> <enveloppe> PX <ttl ms>` (PX, pas SETEX/PSETEX) ;
//! - `flush()` au boot = `SCAN 0 MATCH keyv::* COUNT 1000 TYPE string` puis
//!   `UNLINK <clés…>` par lot (JAMAIS FLUSHDB/FLUSHALL ; segments a_boot,
//!   e_boot_flush) ;
//! - entrée expirée (expires < Date.now()) → `GET` puis `UNLINK` puis miss
//!   (keyv getRaw ; segments c_*) ;
//! - hit L2 → la CHAÎNE stockée est servie telle quelle, et le backfill L1
//!   porte le TTL RESTANT (cacheable `getTtlFromExpires` = expires − now) ;
//! - Redis indisponible (serveur qui accepte puis ferme) : le boot passe
//!   (flush fire-and-forget), les requêtes répondent en MISS silencieux
//!   (erreur L2 avalée par le catch de `Cacheable.getRaw`, cf.
//!   `redis-unavailable.json` : /stats → 200 en 225 ms) — jamais de crash,
//!   jamais de hang observable.
//!
//! Compteurs hit/miss = port de `cacheCounters` (metrics/store.ts) : incrément
//! PAR GET résolu (hit L1 OU L2 → hits, sinon misses), atomiques, exposés pour
//! la phase 5 (aucune route ne les expose — comme le legacy, où le store de
//! metrics n'est relié à aucune route).
//!
//! `set` L2 non bloquant : `Cacheable.set` fait `Promise.race` entre L1 et L2
//! (`nonBlocking: true`) → l'écriture Redis est détachée (spawn), l'erreur L2
//! est avalée ; `flush()` au boot reste fire-and-forget (index.ts:20-22, le
//! spawn est dans `main.rs`).

use moka::Expiry;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Résultat d'un GET L2 : la valeur stockée + le TTL restant (secondes) déduit
/// de `expires` (None = entrée sans expiration).
pub struct L2Hit {
    pub value: String,
    pub ttl_secs: Option<f64>,
}

/// L2 injectable (Redis phase 4b ; no-op sans `REDIS_URL`). Les futures sont
/// `+ Send` explicites : les routes axum exigent des handlers Send, et la
/// preuve générique ne passe pas par les `async fn` de trait nus (RPITIT).
pub trait L2Cache: Send + Sync + 'static {
    fn get(&self, key: &str) -> impl std::future::Future<Output = Option<L2Hit>> + Send;
    fn set(
        &self,
        key: String,
        value: String,
        ttl_secs: f64,
    ) -> impl std::future::Future<Output = ()> + Send;
    fn flush(&self) -> impl std::future::Future<Output = ()> + Send;
}

/// Implémentation no-op (sans `REDIS_URL` — le legacy construit `Cacheable`
/// sans `secondary` dans ce cas : aucune connexion, aucun comportement L2).
pub struct NoopL2;

impl L2Cache for NoopL2 {
    async fn get(&self, _key: &str) -> Option<L2Hit> {
        None
    }
    async fn set(&self, _key: String, _value: String, _ttl_secs: f64) {}
    async fn flush(&self) {}
}

/// `Date.now()` (ms epoch).
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Enveloppe Keyv (@keyv/serialize v1.1.1 `_serialize`) — byte-exact oracle.
// ---------------------------------------------------------------------------

/// Clé Redis réelle : `keyv::keyv:<clé logique>` (oracle manifest.json §prefix).
fn redis_key(key: &str) -> String {
    format!("keyv::keyv:{key}")
}

/// `{"value":<chaîne>,"expires":<epoch ms>}` — `expires` omise si None (TTL
/// nul) ; chaîne commençant par `:` → préfixe `:` supplémentaire (règle
/// escapeColonStrings de @keyv/serialize, désérialisation retire UN `:`).
fn serialize_envelope(value: &str, expires_ms: Option<i64>) -> String {
    let mut out = String::from("{\"value\":");
    crate::js_json::write_json_string(&mut out, &escape_colon_string(value));
    if let Some(exp) = expires_ms {
        out.push_str(&format!(",\"expires\":{exp}"));
    }
    out.push('}');
    out
}

fn escape_colon_string(value: &str) -> String {
    if value.starts_with(':') {
        format!(":{value}")
    } else {
        value.to_string()
    }
}

fn unescape_colon_string(value: &str) -> String {
    value
        .strip_prefix(':')
        .map(String::from)
        .unwrap_or_else(|| value.to_string())
}

// ---------------------------------------------------------------------------
// RedisL2
// ---------------------------------------------------------------------------

/// L2 Redis (`REDIS_URL` présent) — commandes byte-exact de l'oracle :
/// GET / SET…PX / UNLINK / SCAN…MATCH…COUNT…TYPE. Connexion multiplexée
/// paresseuse (la première commande connecte, comme `getClient()` du legacy) ;
/// erreur de connexion/commande → miss silencieux (l'erreur est avalée par le
/// catch de `Cacheable.getRaw` côté legacy) avec une REPRISE à la commande
/// suivante (le legacy rejoue les commandes après reconnexion) ; jamais de
/// panic, jamais de blocage prolongé (ECONNREFUSED/ECONNRESET échouent vite).
pub struct RedisL2 {
    client: redis::Client,
    conn: tokio::sync::Mutex<Option<redis::aio::MultiplexedConnection>>,
    /// Namespace+préfixe constatés (`keyv::keyv:`) — champ documentaire, la
    /// fabrique [`redis_key`] reste la source.
    pub key_prefix: &'static str,
}

impl RedisL2 {
    /// `new KeyvRedis(redisUrl)` — n'OUVRE aucune connexion (le legacy se
    /// contente de créer le client ; la connexion part à la première commande).
    /// Erreur d'URL → None (le legacy crasherait au boot : le caller décide).
    pub fn new(url: &str) -> Option<Self> {
        match redis::Client::open(url) {
            Ok(client) => Some(Self {
                client,
                conn: tokio::sync::Mutex::new(None),
                key_prefix: "keyv::keyv:",
            }),
            Err(_) => None,
        }
    }

    /// Connexion partagée : existante sinon tentative (connecte À LA PREMIÈRE
    /// commande, comme `getClient()`). Échec → None (miss silencieux).
    async fn conn(&self) -> Option<redis::aio::MultiplexedConnection> {
        let mut slot = self.conn.lock().await;
        if let Some(c) = slot.as_ref() {
            return Some(c.clone());
        }
        match self.client.get_multiplexed_tokio_connection().await {
            Ok(c) => {
                *slot = Some(c.clone());
                Some(c)
            }
            Err(_) => None,
        }
    }

    /// Exécute `cmd` avec reprise : erreur → connexion périmée → UNE reprise
    /// sur connexion fraîche (le legacy rejoue les commandes après
    /// reconnexion node-redis) ; encore en erreur → None (miss silencieux,
    /// l'erreur est avalée par le catch de `Cacheable.getRaw`).
    async fn query<T: redis::FromRedisValue>(&self, build: impl Fn() -> redis::Cmd) -> Option<T> {
        let mut conn = self.conn().await?;
        match build().query_async::<T>(&mut conn).await {
            Ok(v) => Some(v),
            Err(_) => {
                *self.conn.lock().await = None;
                let mut fresh = self.conn().await?;
                build().query_async::<T>(&mut fresh).await.ok()
            }
        }
    }
}

impl L2Cache for RedisL2 {
    async fn get(&self, key: &str) -> Option<L2Hit> {
        let full = redis_key(key);
        let raw: Option<String> = self
            .query(|| {
                let mut cmd = redis::cmd("GET");
                cmd.arg(&full);
                cmd
            })
            .await
            .flatten();
        let raw = raw?;
        let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let value = parsed.get("value")?.as_str()?;
        let expires = parsed.get("expires").and_then(|e| e.as_i64());
        if let Some(exp) = expires {
            if exp < now_ms() {
                // entrée expirée : `UNLINK` puis miss (oracle segments c_*)
                let _: Option<i64> = self
                    .query(|| {
                        let mut cmd = redis::cmd("UNLINK");
                        cmd.arg(&full);
                        cmd
                    })
                    .await;
                return None;
            }
        }
        let ttl_secs = expires.map(|exp| (exp - now_ms()) as f64 / 1000.0);
        Some(L2Hit {
            value: unescape_colon_string(value),
            ttl_secs,
        })
    }

    async fn set(&self, key: String, value: String, ttl_secs: f64) {
        let full = redis_key(&key);
        let ttl_ms = ttl_secs * 1000.0;
        // keyv : `ttl === 0` → TTL retiré (SET sans PX, `expires` omise).
        let no_ttl = ttl_ms <= 0.0;
        let expires_ms = if no_ttl {
            None
        } else {
            Some(now_ms().saturating_add(ttl_ms as i64))
        };
        let envelope = serialize_envelope(&value, expires_ms);
        let _: Option<String> = self
            .query(|| {
                let mut cmd = redis::cmd("SET");
                cmd.arg(&full).arg(&envelope);
                if !no_ttl {
                    cmd.arg("PX").arg(ttl_ms as i64);
                }
                cmd
            })
            .await;
    }

    /// `cache.clear()` → KeyvRedis.clear() : boucle SCAN (MATCH `keyv::*`,
    /// COUNT 1000, TYPE string) + UNLINK par lot (oracle a_boot / e_boot_flush).
    async fn flush(&self) {
        let mut cursor = String::from("0");
        loop {
            let part: Option<(String, Vec<String>)> = self
                .query(|| {
                    let mut cmd = redis::cmd("SCAN");
                    cmd.arg(&cursor)
                        .arg("MATCH")
                        .arg("keyv::*")
                        .arg("COUNT")
                        .arg(1000)
                        .arg("TYPE")
                        .arg("string");
                    cmd
                })
                .await;
            let Some((next, keys)) = part else {
                return;
            };
            if !keys.is_empty() {
                let keys_for_cmd = keys.clone();
                let _: Option<i64> = self
                    .query(|| {
                        let mut cmd = redis::cmd("UNLINK");
                        for k in &keys_for_cmd {
                            cmd.arg(k);
                        }
                        cmd
                    })
                    .await;
            }
            cursor = next;
            if cursor == "0" {
                return;
            }
        }
    }
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
    l2: Arc<L2>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl<L2: L2Cache> AppCache<L2> {
    /// `createCache(redisUrl)` — capacité NON BORNÉE (parité KeyvCacheableMemory).
    pub fn new(l2: L2) -> Self {
        Self::with_l1_bound(l2, 0)
    }

    /// Borne protectrice du L1 par POIDS D'OCTETS (revue P4 I-1, divergence
    /// assumée — voir `Config::l1_max_mib`) : `max_bytes > 0` → `moka`
    /// weigher (poids = octets de la clé + de la valeur) + capacité
    /// `max_bytes` → éviction LRU au lieu d'OOM ; `0` → non borné (parité
    /// stricte avec le Map du legacy). Le TTL par entrée est conservé.
    pub fn with_l1_bound(l2: L2, max_bytes: u64) -> Self {
        let l1 = if max_bytes == 0 {
            moka::future::Cache::builder()
                .max_capacity(u64::MAX)
                .expire_after(PerEntryTtl)
                .build()
        } else {
            moka::future::Cache::builder()
                .weigher(|key: &String, value: &CachedValue| (key.len() + value.data.len()) as u32)
                .max_capacity(max_bytes)
                .expire_after(PerEntryTtl)
                .build()
        };
        Self {
            l1,
            l2: Arc::new(l2),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// `cache.get(key)` : L1 puis L2 (comme `Cacheable.get`), compteur
    /// hit/miss PAR GET résolu. Hit L2 → backfill L1 avec le TTL RESTANT
    /// (cacheable `processSecondaryForGetRawNonBlocking` → `primary.set(key,
    /// value, getTtlFromExpires(expires))`) — oracle b_req1 : la requête
    /// suivante fait 0 commande Redis.
    pub async fn get(&self, key: &str) -> Option<String> {
        let result = match self.l1.get(key).await {
            Some(v) => Some(v.data),
            None => match self.l2.get(key).await {
                Some(hit) => {
                    self.l1
                        .insert(
                            key.to_string(),
                            CachedValue {
                                data: hit.value.clone(),
                                ttl_secs: hit.ttl_secs.unwrap_or(f64::INFINITY),
                            },
                        )
                        .await;
                    Some(hit.value)
                }
                None => None,
            },
        };
        if result.is_some() {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    /// `cache.set(key, value, ttlSeconds)` : L1 (TTL par entrée) puis L2 —
    /// `nonBlocking: true` (Promise.race) → l'écriture L2 est DÉTACHÉE : elle
    /// ne bloque jamais la réponse (même Redis indisponible) et son erreur est
    /// avalée (emit 'error' du legacy).
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
        let l2 = Arc::clone(&self.l2);
        let key = key.to_string();
        tokio::spawn(async move { l2.set(key, value, ttl_secs).await });
    }

    /// `cache.clear()` (flush au boot, `index.ts:20-22`) — le spawn est dans
    /// `main.rs` (fire-and-forget côté legacy).
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

    /// Force le traitement des tâches internes moka (évictons en attente) —
    /// utilisé par les tests de borne et la mesure RSS (future moka → await).
    pub async fn run_pending_tasks(&self) {
        self.l1.run_pending_tasks().await;
    }

    /// Nombre d'entrées vivantes du L1 (après `run_pending_tasks`).
    pub fn entry_count(&self) -> u64 {
        self.l1.entry_count()
    }
}
