//! Tests P4b — sémantique Redis L2 vs ORACLE `tests/golden/redis/` (legacy
//! Node 26.13.1 exécuté derrière un faux serveur RESP, `--network none`).
//!
//! Un faux serveur RESP (tokio TcpListener 127.0.0.1, même RESP2 que
//! gen-redis-golden.mjs) enregistre les commandes du PORT byte-exact
//! (hexadécimal, normalisation unique `"expires":<digits>` → `"expires":N`)
//! et les compare aux segments du legacy. Les commandes `CLIENT SETINFO …`
//! du boot (transport node-redis, absentes du client Rust) sont filtrées des
//! deux côtés — documenté dans manifest.json §normalization.
//!
//! Ce qui est prouvé ici : préfixe `keyv::keyv:`, enveloppe
//! `{"value","expires"}` (epoch ms, règle `:`), `SET … PX <ms>`,
//! flush boot = SCAN+UNLINK (jamais FLUSHDB), expiré = GET→UNLINK→miss,
//! hit L2 = chaîne telle quelle + backfill L1 (TTL restant), L1 DEVANT Redis,
//! Redis down = miss silencieux sans hang, SVG d'erreur jamais en L2,
//! et le mode sans `REDIS_URL` (NoopL2) sans aucune connexion.

use gitcard_p0::app::{build_router, prod_state};
use gitcard_p0::cache::{AppCache, NoopL2, RedisL2};
use gitcard_p0::pat_pool::PatPool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// ----------------------------- goldens ---------------------------------------
const A_BOOT: &str = include_str!("../../tests/golden/redis/a_boot.hexlog");
const A_REQ1_GET_SVG: &str = include_str!("../../tests/golden/redis/a_req1_get_svg.hexlog");
const A_REQ1_SET_DATA: &str = include_str!("../../tests/golden/redis/a_req1_set_data.hexlog");
const A_REQ1_SET_SVG: &str = include_str!("../../tests/golden/redis/a_req1_set_svg.hexlog");
const A_REQ2_ALL: &str = include_str!("../../tests/golden/redis/a_req2_all.hexlog");
const E_BOOT_FLUSH: &str = include_str!("../../tests/golden/redis/e_boot_flush.hexlog");
const B_REQ1_GET_SVG: &str = include_str!("../../tests/golden/redis/b_req1_get_svg.hexlog");
const B_REQ2_ALL: &str = include_str!("../../tests/golden/redis/b_req2_all.hexlog");
const C_GET_SVG_EXPIRED: &str = include_str!("../../tests/golden/redis/c_get_svg_expired.hexlog");
const C_UNLINK_SVG: &str = include_str!("../../tests/golden/redis/c_unlink_svg.hexlog");

fn golden_lines(raw: &str) -> Vec<String> {
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

/// Retire les commandes `CLIENT` (transport : SETINFO LIB-VER/LIB-NAME —
/// node-redis côté legacy, redis-rs côté port) des DEUX côtés.
fn golden_cache_lines(raw: &str) -> Vec<String> {
    golden_lines(raw)
        .into_iter()
        .filter(|l| name_of(l) != "CLIENT")
        .collect()
}

fn observed_cache_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|l| name_of(l) != "CLIENT")
        .cloned()
        .collect()
}

// --------------------------- helpers RESP ------------------------------------
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

/// Nom de commande d'une ligne hexlog (1er argument RESP).
fn name_of(hexline: &str) -> String {
    parse_resp_args(&unhex(hexline))[0].clone()
}

/// Tous les arguments bulk d'une commande RESP encodée en hexlog.
fn parse_resp_args(bytes: &[u8]) -> Vec<String> {
    assert_eq!(bytes[0], b'*', "RESP array attendu");
    let mut i = 1;
    let nl = find_crlf(bytes, i);
    let count: usize = std::str::from_utf8(&bytes[i..nl])
        .expect("utf8")
        .parse()
        .expect("count");
    i = nl + 2;
    let mut args = Vec::new();
    for _ in 0..count {
        assert_eq!(bytes[i], b'$', "bulk attendu");
        let nl2 = find_crlf(bytes, i + 1);
        let len: usize = std::str::from_utf8(&bytes[i + 1..nl2])
            .expect("utf8")
            .parse()
            .expect("len");
        let start = nl2 + 2;
        args.push(String::from_utf8(bytes[start..start + len].to_vec()).expect("utf8"));
        i = start + len + 2;
    }
    args
}

fn find_crlf(bytes: &[u8], from: usize) -> usize {
    bytes[from..]
        .windows(2)
        .position(|w| w == b"\r\n")
        .map(|p| p + from)
        .expect("CRLF")
}

/// Normalisation UNIQUE (comme gen-redis-golden.mjs) : chaque CHIFFRE de
/// `"expires":<epoch>` → 0 (longueur CONSERVÉE → les hexlog restent des
/// commandes RESP bien formées, re-parsables et stables entre régénérations).
fn normalize(bytes: &[u8]) -> Vec<u8> {
    const MARK: &[u8] = b"\"expires\":";
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(MARK) {
            out.extend_from_slice(MARK);
            i += MARK.len();
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                out.push(b'0');
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// --------------------------- faux serveur RESP -------------------------------
#[derive(Clone)]
struct FakeServer {
    addr: std::net::SocketAddr,
    records: Arc<Mutex<Vec<String>>>, // hex normalisés, dans l'ordre d'arrivée
    store: Arc<Mutex<Vec<(String, String)>>>,
}

impl FakeServer {
    async fn spawn() -> Self {
        Self::spawn_inner(false).await
    }
    /// `dead` : accepte puis ferme IMMÉDIATEMENT chaque connexion (scénario D
    /// de l'oracle — Redis indisponible).
    async fn spawn_dead() -> Self {
        Self::spawn_inner(true).await
    }
    async fn spawn_inner(dead: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let records = Arc::new(Mutex::new(Vec::new()));
        let store = Arc::new(Mutex::new(Vec::new()));
        {
            let records = Arc::clone(&records);
            let store = Arc::clone(&store);
            tokio::spawn(async move {
                loop {
                    let (sock, _) = match listener.accept().await {
                        Ok(x) => x,
                        Err(_) => return,
                    };
                    if dead {
                        drop(sock);
                        continue;
                    }
                    serve_conn(sock, Arc::clone(&records), Arc::clone(&store)).await;
                }
            });
        }
        Self {
            addr,
            records,
            store,
        }
    }
    fn url(&self) -> String {
        format!("redis://{}", self.addr)
    }
    fn record_hexes(&self) -> Vec<String> {
        self.records.lock().expect("lock").clone()
    }
    fn record_range_hexes(&self, from: usize) -> Vec<String> {
        self.records
            .lock()
            .expect("lock")
            .iter()
            .skip(from)
            .cloned()
            .collect()
    }
    fn seed(&self, key: &str, value: &str) {
        self.store
            .lock()
            .expect("lock")
            .push((key.to_string(), value.to_string()));
    }
    fn store_get(&self, key: &str) -> Option<String> {
        self.store
            .lock()
            .expect("lock")
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }
    fn count_sets(&self) -> usize {
        self.records
            .lock()
            .expect("lock")
            .iter()
            .filter(|l| name_of(l) == "SET")
            .count()
    }
}

async fn serve_conn(
    mut sock: TcpStream,
    records: Arc<Mutex<Vec<String>>>,
    store: Arc<Mutex<Vec<(String, String)>>>,
) {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        while let Some(parsed) = parse_command(&buf) {
            let reply = handle(&parsed.name, &parsed.args, &parsed.raw, &store, &records);
            let consumed = parsed.consumed;
            if sock.write_all(&reply).await.is_err() {
                return;
            }
            buf.drain(..consumed);
        }
        match sock.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
}

struct Parsed {
    name: String,
    args: Vec<String>,
    raw: Vec<u8>,
    consumed: usize,
}

fn parse_command(buf: &[u8]) -> Option<Parsed> {
    if buf.is_empty() || buf[0] != b'*' {
        return None;
    }
    let mut i = 1;
    let nl = find_crlf(buf, i);
    let count: usize = std::str::from_utf8(&buf[i..nl]).ok()?.parse().ok()?;
    i = nl + 2;
    let mut marks = Vec::new();
    for _ in 0..count {
        if buf.get(i) != Some(&b'$') {
            return None;
        }
        let nl2 = find_crlf(buf, i + 1);
        let len: usize = std::str::from_utf8(&buf[i + 1..nl2]).ok()?.parse().ok()?;
        let start = nl2 + 2;
        let end = start + len + 2;
        if buf.len() < end {
            return None;
        }
        marks.push((start, start + len));
        i = end;
    }
    let args: Vec<String> = marks
        .iter()
        .map(|(s, e)| String::from_utf8(buf[*s..*e].to_vec()).expect("utf8"))
        .collect();
    let name = args[0].to_ascii_uppercase();
    let args = args[1..].to_vec();
    Some(Parsed {
        name,
        args,
        raw: buf[..i].to_vec(),
        consumed: i,
    })
}

fn bulk(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = format!("${}\r\n", b.len()).into_bytes();
    out.extend_from_slice(b);
    out.extend_from_slice(b"\r\n");
    out
}

fn handle(
    name: &str,
    args: &[String],
    raw: &[u8],
    store: &Arc<Mutex<Vec<(String, String)>>>,
    records: &Arc<Mutex<Vec<String>>>,
) -> Vec<u8> {
    records.lock().expect("lock").push(hex(&normalize(raw)));
    let mut reply = Vec::new();
    match name {
        "GET" => {
            let s = store.lock().expect("lock");
            match s.iter().find(|(k, _)| *k == args[0]) {
                Some((_, v)) => reply.extend_from_slice(&bulk(v)),
                None => reply.extend_from_slice(b"$-1\r\n"),
            }
        }
        "SET" => {
            store
                .lock()
                .expect("lock")
                .push((args[0].clone(), args[1].clone()));
            reply.extend_from_slice(b"+OK\r\n");
        }
        "UNLINK" | "DEL" => {
            let mut s = store.lock().expect("lock");
            let mut n = 0;
            for k in args {
                if let Some(pos) = s.iter().position(|(key, _)| key == k) {
                    s.remove(pos);
                    n += 1;
                }
            }
            reply.extend_from_slice(format!(":{n}\r\n").as_bytes());
        }
        "SCAN" => {
            // args : cursor [MATCH p] [COUNT n] [TYPE string] — préfixe only
            let cursor: usize = args[0].parse().expect("cursor");
            let mut match_prefix = String::new();
            let mut count = 10usize;
            let mut j = 1;
            while j + 1 < args.len() {
                match args[j].to_ascii_uppercase().as_str() {
                    "MATCH" => {
                        match_prefix = args[j + 1].trim_end_matches('*').to_string();
                        j += 2;
                    }
                    "COUNT" => {
                        count = args[j + 1].parse().expect("count");
                        j += 2;
                    }
                    _ => j += 2,
                }
            }
            let keys: Vec<String> = store
                .lock()
                .expect("lock")
                .iter()
                .filter(|(k, _)| k.starts_with(&match_prefix))
                .map(|(k, _)| k.clone())
                .collect();
            let start = cursor.min(keys.len());
            let end = (start + count).min(keys.len());
            let batch = keys[start..end].to_vec();
            let next = if end >= keys.len() {
                "0".to_string()
            } else {
                end.to_string()
            };
            reply.extend_from_slice(b"*2\r\n");
            reply.extend_from_slice(&bulk(&next));
            reply.extend_from_slice(format!("*{}\r\n", batch.len()).as_bytes());
            for k in batch {
                reply.extend_from_slice(&bulk(&k));
            }
        }
        _ => reply.extend_from_slice(b"+OK\r\n"),
    }
    reply
}

// ------------------------------ utilitaires ----------------------------------
async fn sleep_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

/// Valeur (désérialisée) de l'enveloppe Keyv masquée. NB : l'horodatage masqué
/// `"expires":000…0` n'est PAS du JSON valide (zéros non significatifs) → on
/// isole la chaîne `"value":<json-string>` par scanning d'échappements, puis
/// serde_json la décode seule.
fn envelope_value_str(env: &str) -> String {
    const P: &str = "{\"value\":";
    let start = env.find(P).expect("clé value") + P.len();
    let bytes = env.as_bytes();
    assert_eq!(bytes[start], b'"', "chaîne JSON attendue");
    let mut i = start + 1;
    let end = loop {
        match bytes.get(i) {
            Some(b'\\') => i += 2,
            Some(b'"') => break i + 1,
            Some(_) => i += 1,
            None => panic!("fermeture de chaîne"),
        }
    };
    let mut value: String = serde_json::from_str(&env[start..end]).expect("string json");
    if let Some(stripped) = value.strip_prefix("::") {
        value = format!(":{stripped}");
    }
    value
}

/// Extrait la valeur de l'enveloppe d'un SET golden : hexlog → args →
/// `{"value":"…","expires":…}` → (clé complète, value avec règle `:` retirée).
/// NB : parse_resp_args renvoie le NOM de commande en [0].
fn envelope_value_of_set(hexline: &str) -> (String, String) {
    let args = parse_resp_args(&unhex(hexline));
    (args[1].clone(), envelope_value_str(&args[2]))
}

fn clock_now() -> Arc<dyn Fn() -> u64 + Send + Sync> {
    Arc::new(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    })
}

fn assert_eq_lines(actual: &[String], expected: &[String], label: &str) {
    assert_eq!(
        actual, expected,
        "{label} : attendu (oracle) = {expected:?}, observé (port) = {actual:?}"
    );
}

// --------------------------------- tests -------------------------------------
#[tokio::test]
async fn oracle_a_boot_flush_scan_puis_get_miss_set_px_et_hit_l1() {
    let srv = FakeServer::spawn().await;
    let l2 = RedisL2::new(&srv.url()).expect("client");
    let cache = AppCache::new(l2);
    let mut idx;

    // flush au boot : SCAN (jamais FLUSHDB/FLUSHALL) — segment a_boot
    //     = [CLIENT×2 (filtrés des deux côtés) + SCAN]
    cache.flush().await;
    assert_eq_lines(
        &observed_cache_lines(&srv.record_hexes()),
        &golden_cache_lines(A_BOOT),
        "boot",
    );
    idx = srv.record_hexes().len();

    // (2) premier GET manquant : GET keyv::keyv:svg:stats:probe-user:289b4511
    let svg_key = "svg:stats:probe-user:289b4511";
    let miss = cache.get(svg_key).await;
    assert!(miss.is_none(), "miss attendu (store vide)");
    assert_eq_lines(
        &observed_cache_lines(&srv.record_range_hexes(idx)),
        &golden_lines(A_REQ1_GET_SVG),
        "get_miss_svg",
    );
    assert_eq!(cache.counters(), (0, 1));
    idx = srv.record_hexes().len();

    // (3) SET data puis SET svg — enveloppe byte-exact (normalisation expires),
    //     avec LES MÊMES valeurs que le legacy (reprises des segments golden).
    let (data_key_full, data_value) = envelope_value_of_set(&golden_lines(A_REQ1_SET_DATA)[0]);
    let logical_data = data_key_full
        .strip_prefix("keyv::keyv:")
        .expect("préfixe oracle");
    cache.set(logical_data, data_value, 14400.0).await;
    sleep_ms(200).await; // set L2 non bloquant (Promise.race → spawn)
    assert_eq_lines(
        &observed_cache_lines(&srv.record_range_hexes(idx)),
        &golden_lines(A_REQ1_SET_DATA),
        "set_data",
    );
    idx = srv.record_hexes().len();

    let (_, svg_value) = envelope_value_of_set(&golden_lines(A_REQ1_SET_SVG)[0]);
    cache.set(svg_key, svg_value.clone(), 14400.0).await;
    sleep_ms(200).await;
    assert_eq_lines(
        &observed_cache_lines(&srv.record_range_hexes(idx)),
        &golden_lines(A_REQ1_SET_SVG),
        "set_svg",
    );
    idx = srv.record_hexes().len();

    // (4) requête IDENTIQUE → hit L1 → ZÉRO commande Redis (segment
    //     a_req2_all = vide) = preuve que le legacy garde le L1 DEVANT Redis.
    let hit = cache.get(svg_key).await;
    assert_eq!(hit.as_deref(), Some(svg_value.as_str()));
    assert_eq_lines(
        &observed_cache_lines(&srv.record_range_hexes(idx)),
        &golden_lines(A_REQ2_ALL),
        "req2_hit_l1",
    );
    assert_eq!(cache.counters(), (1, 1));
}

#[tokio::test]
async fn oracle_e_flush_avec_cles_scan_puis_unlink_batch() {
    let srv = FakeServer::spawn().await;
    srv.seed(
        "keyv::keyv:svg:stats:olduser:12345678",
        "{\"value\":\"<svg-old/>\",\"expires\":0}",
    );
    srv.seed(
        "keyv::keyv:data:olduser",
        "{\"value\":\"{\\\"stats\\\":1}\",\"expires\":0}",
    );
    let l2 = RedisL2::new(&srv.url()).expect("client");
    let cache = AppCache::new(l2);
    cache.flush().await;
    // SCAN (MATCH keyv::* COUNT 1000 TYPE string) puis UNLINK <k1> <k2>
    assert_eq_lines(
        &observed_cache_lines(&srv.record_hexes()),
        &golden_cache_lines(E_BOOT_FLUSH),
        "boot_flush_keys",
    );
    assert!(srv
        .store_get("keyv::keyv:svg:stats:olduser:12345678")
        .is_none());
    assert!(srv.store_get("keyv::keyv:data:olduser").is_none());
}

#[tokio::test]
async fn oracle_b_hit_l2_chaine_telle_quelle_et_backfill_l1_ttl_restant() {
    let srv = FakeServer::spawn().await;
    let l2 = RedisL2::new(&srv.url()).expect("client");
    let cache = AppCache::new(l2);
    // boot flush (store vide) — le segment b_boot équivaut à a_boot
    cache.flush().await;
    assert_eq_lines(
        &observed_cache_lines(&srv.record_hexes()),
        &golden_cache_lines(A_BOOT),
        "b_boot",
    );
    let idx = srv.record_hexes().len();

    // seed APRÈS le boot (comme l'oracle : écriture d'une autre instance)
    let (full_key, svg_value) = envelope_value_of_set(&golden_lines(A_REQ1_SET_SVG)[0]);
    let expires = now_ms() + 14_400_000;
    let envelope = format!(
        "{{\"value\":{},\"expires\":{expires}}}",
        serde_json::to_string(&svg_value).expect("json")
    );
    srv.seed(&full_key, &envelope);

    // hit L2 : 1 GET, la CHAÎNE stockée est renvoyée telle quelle
    let hit = cache
        .get("svg:stats:probe-user:289b4511")
        .await
        .expect("hit");
    assert_eq!(hit, svg_value, "chaîne stockée servie telle quelle");
    assert_eq_lines(
        &srv.record_range_hexes(idx),
        &golden_lines(B_REQ1_GET_SVG),
        "b_req1_get_svg",
    );
    assert_eq!(cache.counters(), (1, 0));

    // requête suivante → hit L1 (backfill) → ZÉRO commande (b_req2_all vide)
    let idx = srv.record_hexes().len();
    let hit2 = cache
        .get("svg:stats:probe-user:289b4511")
        .await
        .expect("hit L1");
    assert_eq!(hit2, svg_value);
    assert_eq_lines(
        &observed_cache_lines(&srv.record_range_hexes(idx)),
        &golden_lines(B_REQ2_ALL),
        "b_req2_hit_l1",
    );
    assert_eq!(cache.counters(), (2, 0));
}

/// Backfill L1 : le TTL du hit L2 est le TTL RESTANT (getTtlFromExpires =
/// expires − now), pas le TTL complet — preuve comportementale : une entrée
/// seedée avec expires = now+1500 ms remonte du L2, alimente le L1 qui
/// expire ~1,5 s plus tard → le GET suivant repasse par Redis (2e GET).
#[tokio::test]
async fn backfill_l1_ttl_restant_court_repart_vers_redis_apres_expiration() {
    let srv = FakeServer::spawn().await;
    let l2 = RedisL2::new(&srv.url()).expect("client");
    let cache = AppCache::new(l2);
    cache.flush().await;
    srv.seed(
        "keyv::keyv:svg:stats:shortttl:00000001",
        &format!(
            "{{\"value\":\"{}\",\"expires\":{}}}",
            "<svg-short/>",
            now_ms() + 1500
        ),
    );

    // hit L2 (1er GET) → backfill L1 avec ~1,5 s
    let hit = cache.get("svg:stats:shortttl:00000001").await.expect("hit");
    assert_eq!(hit, "<svg-short/>");
    let after_first = srv.record_hexes().len();

    // immédiatement : hit L1 → 0 commande
    let hit2 = cache
        .get("svg:stats:shortttl:00000001")
        .await
        .expect("hit L1");
    assert_eq!(hit2, "<svg-short/>");
    assert_eq!(
        srv.record_hexes().len(),
        after_first,
        "hit L1 : aucune commande"
    );
    sleep_ms(1800).await;

    // après expiration du L1 : nouveau GET Redis (le backfill a bien porté le
    // TTL RESTANT, sinon le L1 servirait encore ~14400 s) ; l'enveloppe du
    // store est elle-même périmée → GET + UNLINK (oracle segment c_*)
    let miss3 = cache.get("svg:stats:shortttl:00000001").await;
    assert!(
        miss3.is_none(),
        "enveloppe expirée (seed 1,5 s) → miss + UNLINK"
    );
    let names: Vec<String> = srv
        .record_range_hexes(after_first)
        .iter()
        .map(|l| name_of(l))
        .collect();
    assert_eq!(
        names,
        vec!["GET".to_string(), "UNLINK".to_string()],
        "L1 expiré → re-GET Redis puis UNLINK (enveloppe devenue périmée)"
    );
    assert_eq!(cache.counters(), (2, 1));
}

/// TTL restant du L2Hit déduit de `expires` (au niveau RedisL2 via le trait).
#[tokio::test]
async fn l2_hit_ttl_restant_deduit_de_expires() {
    use gitcard_p0::cache::L2Cache;
    let srv = FakeServer::spawn().await;
    let l2 = RedisL2::new(&srv.url()).expect("client");
    srv.seed(
        "keyv::keyv:data:ttlsense",
        &format!("{{\"value\":\"x\",\"expires\":{}}}", now_ms() + 10_000),
    );
    let hit = L2Cache::get(&l2, "data:ttlsense").await.expect("hit");
    assert_eq!(hit.value, "x");
    let ttl = hit.ttl_secs.expect("ttl restant");
    assert!(
        (8.5..=10.0).contains(&ttl),
        "TTL restant ∈ [8,5 ; 10] s, reçu {ttl}"
    );
}

#[tokio::test]
async fn oracle_c_entree_expiree_get_puis_unlink_puis_miss() {
    let srv = FakeServer::spawn().await;
    let l2 = RedisL2::new(&srv.url()).expect("client");
    let cache = AppCache::new(l2);
    cache.flush().await;
    let (full_key, svg_value) = envelope_value_of_set(&golden_lines(A_REQ1_SET_SVG)[0]);
    // seed APRÈS le boot : enveloppe EXPIRÉE (expires < Date.now())
    srv.seed(
        &full_key,
        &format!(
            "{{\"value\":{},\"expires\":{}}}",
            serde_json::to_string(&svg_value).expect("json"),
            now_ms() - 1000
        ),
    );
    let idx = srv.record_hexes().len();

    let miss = cache.get("svg:stats:probe-user:289b4511").await;
    assert!(miss.is_none(), "expiré → miss");
    let mut expected = golden_cache_lines(C_GET_SVG_EXPIRED);
    expected.extend(golden_cache_lines(C_UNLINK_SVG));
    assert_eq_lines(
        &observed_cache_lines(&srv.record_range_hexes(idx)),
        &expected,
        "expired_get_unlink",
    );
    // l'entrée expirée a été retirée du store par le UNLINK
    assert!(srv.store_get(&full_key).is_none());
    assert_eq!(cache.counters(), (0, 1));
}

#[tokio::test]
async fn redis_down_miss_silencieux_boot_ok_jamais_de_hang() {
    // scénario D de l'oracle : serveur qui ACCEPTE puis FERME chaque connexion
    let srv = FakeServer::spawn_dead().await;
    let l2 = RedisL2::new(&srv.url()).expect("client");
    let cache = AppCache::new(l2);
    // flush (boot fire-and-forget) ne plante pas et ne bloque pas
    let t0 = Instant::now();
    cache.flush().await;
    assert!(t0.elapsed() < Duration::from_secs(3), "flush down rapide");
    // GET → miss (erreur avalée), jamais de hang observable
    let t0 = Instant::now();
    let v = cache.get("svg:stats:probe-user:289b4511").await;
    assert!(v.is_none(), "Redis down → miss");
    assert!(
        t0.elapsed() < Duration::from_secs(3),
        "get down rapide (pas de hang)"
    );
    // SET (spawn, non bloquant) ne plante pas : côté legacy l'écriture part
    // vers le L1 (le SET Redis échoue silencieusement) → le GET suivant est
    // un HIT L1 avec ZÉRO commande Redis (parité `cacheable` nonBlocking).
    cache
        .set("data:probe-user", "{\"stats\":1}".into(), 14400.0)
        .await;
    sleep_ms(250).await;
    let idx = srv.record_hexes().len();
    let v = cache.get("data:probe-user").await;
    assert_eq!(
        v.as_deref(),
        Some("{\"stats\":1}"),
        "hit L1 (le legacy fait pareil)"
    );
    assert_eq!(
        srv.record_hexes().len(),
        idx,
        "hit L1 : aucune commande Redis"
    );
    assert_eq!(cache.counters(), (1, 1));
    assert!(
        observed_cache_lines(&srv.record_hexes()).is_empty(),
        "aucune commande CACHE persistée côté serveur fermé"
    );
}

#[tokio::test]
async fn sans_redis_url_noop_l2_aucune_connexion() {
    // témoin : un serveur qui compte ses connexions
    let victim = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let accepts = Arc::new(AtomicUsize::new(0));
    {
        let accepts = Arc::clone(&accepts);
        tokio::spawn(async move {
            loop {
                if let Ok((sock, _)) = victim.accept().await {
                    accepts.fetch_add(1, Ordering::Relaxed);
                    drop(sock);
                }
            }
        });
    }

    let cache = AppCache::new(NoopL2);
    assert!(cache.get("svg:stats:probe-user:289b4511").await.is_none());
    cache
        .set("data:probe-user", "{\"stats\":1}".into(), 14400.0)
        .await;
    cache.flush().await;
    assert_eq!(cache.counters(), (0, 1));
    sleep_ms(300).await;
    assert_eq!(
        accepts.load(Ordering::Relaxed),
        0,
        "sans REDIS_URL : AUCUNE connexion (comportement P4 inchangé)"
    );
}

#[tokio::test]
async fn enveloppe_colon_prefix_rule_et_ttl_zero() {
    let srv = FakeServer::spawn().await;
    let l2 = RedisL2::new(&srv.url()).expect("client");
    let cache = AppCache::new(l2);
    // valeur commençant par `:` → préfixe `:` supplémentaire
    // (@keyv/serialize escapeColonStrings), SET sans PX si TTL nul.
    cache.set("data:colon", ":special".into(), 14400.0).await;
    cache.set("data:nottl", "sans-ttl".into(), 0.0).await;
    sleep_ms(250).await;
    let hexes = observed_cache_lines(&srv.record_hexes());
    assert_eq!(hexes.len(), 2, "2 SET (transport CLIENT filtré)");
    let args1 = parse_resp_args(&unhex(&hexes[0]));
    assert_eq!(args1[0], "SET");
    assert_eq!(args1[1], "keyv::keyv:data:colon");
    assert!(
        args1[2].starts_with("{\"value\":\"::special\""),
        "échappement `:` : {}",
        args1[2]
    );
    assert_eq!(args1.len(), 5, "SET … PX <ms>");
    let args2 = parse_resp_args(&unhex(&hexes[1]));
    assert_eq!(args2[1], "keyv::keyv:data:nottl");
    assert_eq!(
        args2[2], "{\"value\":\"sans-ttl\"}",
        "ttl 0 → `expires` omise"
    );
    assert_eq!(
        args2.len(),
        3,
        "ttl 0 → SET sans PX (règle keyv `ttl === 0`)"
    );
    // roundtrip : hit L2 → désérialisation retire UN `:`
    let hit = cache.get("data:colon").await.expect("hit");
    assert_eq!(hit, ":special");
}

#[tokio::test]
async fn route_erreur_svg_jamais_en_l2_et_succes_ecrit_en_l2() {
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // (d) SVG d'erreur jamais écrit en L2 : GitHub renvoie « Could not resolve
    // to a User » → 404 SVG → aucun SET Redis.
    let srv = FakeServer::spawn().await;
    let gh = MockServer::start().await;
    let not_found = serde_json::json!({
        "errors": [{"message": "Could not resolve to a User with the login 'ghost-user'"}]
    });
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_json(not_found))
        .mount(&gh)
        .await;
    let pool = PatPool::new(vec!["fake-token-1".into()], clock_now()).expect("pool");
    let mut state = prod_state(
        AppCache::new(RedisL2::new(&srv.url()).expect("client")),
        pool,
        14400.0,
        None,
    );
    state.github_url = gh.uri();
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let app = build_router(state);
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .expect("serve");
    });

    let raw = http_get(addr, "/stats/ghost-user").await;
    assert!(
        raw.starts_with("HTTP/1.1 404"),
        "404 : {}",
        &raw[..raw.len().min(120)]
    );
    assert!(raw.contains("User not found: ghost-user"), "SVG d'erreur");
    sleep_ms(250).await;
    assert_eq!(srv.count_sets(), 0, "AUCUN SET sur le chemin d'erreur");
    // seuls des GET miss ont été émis (transport CLIENT filtré ; clé svg, clé data)
    assert!(observed_cache_lines(&srv.record_hexes())
        .iter()
        .all(|l| name_of(l) == "GET"));

    // chemin de SUCCÈS : fixture GraphQL → 2 GET miss + SET data + SET svg ;
    // les octets du SVG servi = la chaîne stockée en enveloppe L2.
    let srv2 = FakeServer::spawn().await;
    let gh2 = MockServer::start().await;
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/golden/graphql/fixtures/name-null.json");
    let fixture_body =
        std::fs::read_to_string(&fixture).expect("fixture graphql name-null (oracle corpus)");
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_body))
        .mount(&gh2)
        .await;
    let pool2 = PatPool::new(vec!["fake-token-1".into()], clock_now()).expect("pool");
    let mut state2 = prod_state(
        AppCache::new(RedisL2::new(&srv2.url()).expect("client")),
        pool2,
        14400.0,
        None,
    );
    state2.github_url = gh2.uri();
    let listener2 = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr2 = listener2.local_addr().expect("addr");
    let app2 = build_router(state2);
    tokio::spawn(async move {
        axum::serve(
            listener2,
            app2.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .expect("serve");
    });

    let raw2 = http_get(addr2, "/stats/probe-user").await;
    assert!(
        raw2.starts_with("HTTP/1.1 200"),
        "200 : {}",
        &raw2[..raw2.len().min(120)]
    );
    sleep_ms(250).await;
    let hexes = observed_cache_lines(&srv2.record_hexes());
    let names: Vec<String> = hexes.iter().map(|l| name_of(l)).collect();
    assert_eq!(
        names,
        vec![
            "GET".to_string(),
            "GET".to_string(),
            "SET".to_string(),
            "SET".to_string()
        ],
        "GET svg + GET data + SET data + SET svg (comme l'oracle a_req1)"
    );
    // enveloppe du SET svg : la valeur = les octets du SVG servi
    let svg_args = parse_resp_args(&unhex(&hexes[3]));
    let served = raw2.split("\r\n\r\n").nth(1).expect("corps");
    assert_eq!(
        envelope_value_str(&svg_args[2]),
        served,
        "chaîne servie = chaîne stockée"
    );
    assert_eq!(svg_args[1], "keyv::keyv:svg:stats:probe-user:289b4511");
    assert_eq!(svg_args[3], "PX");
    assert_eq!(svg_args[4], "14400000");
}

/// GET HTTP/1.1 brut (Connection: close) — réponse complète en String.
async fn http_get(addr: std::net::SocketAddr, path: &str) -> String {
    let mut sock = TcpStream::connect(addr).await.expect("connect");
    sock.write_all(
        format!("GET {path} HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .await
    .expect("write");
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf).await.expect("read");
    String::from_utf8_lossy(&buf).to_string()
}
