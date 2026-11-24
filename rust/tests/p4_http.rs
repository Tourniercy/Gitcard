//! P4 — parité HTTP contre les ORACLES du legacy exécuté hors réseau
//! (`tests/golden/http/`, captures socket brute par `scripts/gen-http-golden.mjs`)
//! + paramsHash, SVG d'erreur, config zod, cache L1, statut M-B au niveau route.
//!
//! Aucun appel réseau réel : le serveur du port lie 127.0.0.1:0 ; l'URL GitHub
//! pointe un port FERMÉ (127.0.0.1:1 → « fetch failed », même issue que le
//! legacy sans DNS) ou wiremock (127.0.0.1) pour les chemins de succès.
//! Normalisations assumées (documentées) : en-têtes `Date` (dynamique, format
//! vérifié) et `ratelimit-*` (phase 5, non portés à ce stade).

use gitcard_p0::app::{build_router, prod_state, AppState};
use gitcard_p0::cache::{AppCache, NoopL2};
use gitcard_p0::config::load_config;
use gitcard_p0::github::GitHubError;
use gitcard_p0::pat_pool::PatPool;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn golden_http() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/golden/http")
}

fn clock_zero() -> (Arc<AtomicU64>, Arc<dyn Fn() -> u64 + Send + Sync>) {
    let cell = Arc::new(AtomicU64::new(0));
    let read = {
        let cell = cell.clone();
        Arc::new(move || cell.load(Ordering::SeqCst)) as Arc<dyn Fn() -> u64 + Send + Sync>
    };
    (cell, read)
}

fn boot_state(cors_origin: Option<&str>) -> AppState<NoopL2> {
    let (_, now) = clock_zero();
    let pool = PatPool::new(vec!["fake-token-1".into()], now).expect("pool");
    let mut state = prod_state(
        AppCache::new(NoopL2),
        pool,
        14400.0,
        cors_origin.map(String::from),
    );
    // port FERMÉ : connect refused → « fetch failed » (comme le legacy sans DNS)
    state.github_url = "http://127.0.0.1:1/graphql".to_string();
    state
}

async fn spawn_server(state: AppState<NoopL2>) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let app = build_router(state);
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("serve");
    });
    addr
}

fn capture_raw(addr: SocketAddr, request: &str) -> String {
    let mut stream = std::net::TcpStream::connect(addr).expect("connect");
    stream.write_all(request.as_bytes()).expect("write");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).expect("read");
    String::from_utf8_lossy(&buf).into_owned()
}

struct RawResponse {
    status_line: String,
    headers: Vec<(String, String)>,
    body: String,
    date: Option<String>,
}

fn parse_response(raw: &str) -> RawResponse {
    let (head, body) = match raw.split_once("\r\n\r\n") {
        Some((h, b)) => (h, b.to_string()),
        None => (raw, String::new()),
    };
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default().to_string();
    let mut headers = Vec::new();
    let mut date = None;
    for line in lines {
        if let Some((n, v)) = line.split_once(':') {
            let name = n.trim().to_ascii_lowercase();
            let value = v.trim().to_string();
            if name == "date" {
                date = Some(value);
                continue;
            }
            headers.push((name, value));
        }
    }
    RawResponse {
        status_line,
        headers,
        body,
        date,
    }
}

/// Format IMF-fixdate du `Date` (valeur dynamique : format vérifié, pas la valeur).
fn valid_http_date(v: &str) -> bool {
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    days.iter().any(|d| v.starts_with(&format!("{d}, "))) && v.ends_with(" GMT") && v.len() == 29
}

fn normalize(headers: &[(String, String)]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(n, _)| n != "date" && !n.starts_with("ratelimit-"))
        .cloned()
        .collect()
}

/// Parité HTTP : chaque sonde des DEUX boots (CORS `*` et CORS_ORIGIN) est
/// rejouée contre le port, requête OCTET pour OCTET (Host réécrit), réponse
/// comparée : ligne de statut, liste ORDONNÉE des en-têtes (Date exclu,
/// ratelimit-* exclu phase 5), corps à l'octet — sauf le timestamp dynamique
/// de /health (format vérifié).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parite_http_oracles_deux_boots() {
    for boot in ["base", "cors-origin"] {
        let manifest: Value = serde_json::from_str(
            &std::fs::read_to_string(golden_http().join(boot).join("manifest.json"))
                .expect("manifeste"),
        )
        .expect("manifeste invalide");
        let cors_cfg = manifest["generator"]["corsOrigin"]
            .as_str()
            .map(|s| s.to_string());
        let state = boot_state(cors_cfg.as_deref());
        let addr = spawn_server(state).await;

        for probe in manifest["probes"].as_array().expect("probes") {
            let name = probe["name"].as_str().expect("name");
            let request = probe["request"].as_str().expect("request");
            let rewritten = request.replace("127.0.0.1:3000", &addr.to_string());
            // capture en thread bloquant (lecture socket) : ne gèle pas le runtime
            let raw = std::thread::scope(|s| {
                let h = s.spawn(|| capture_raw(addr, &rewritten));
                h.join().expect("capture thread")
            });
            let got = parse_response(&raw);

            assert_eq!(
                got.status_line,
                probe["statusLine"].as_str().expect("statusLine"),
                "{boot}/{name} : ligne de statut"
            );
            assert!(
                got.date.as_deref().is_some_and(valid_http_date),
                "{boot}/{name} : en-tête Date manquant ou mal formé : {:?}",
                got.date
            );
            let expected_headers: Vec<(String, String)> = probe["headers"]
                .as_array()
                .expect("headers")
                .iter()
                .map(|h| {
                    let s = h.as_str().expect("header str");
                    let (n, v) = s.split_once(':').expect("header format");
                    (n.trim().to_ascii_lowercase(), v.trim().to_string())
                })
                .filter(|(n, _)| n != "date" && !n.starts_with("ratelimit-"))
                .collect();
            let got_headers = normalize(&got.headers);
            // HEAD : hyper écrit un `content-length` (taille connue ou 0) au
            // niveau TRANSPORT là où le serveur Node n'en envoie pas — écart
            // transport assumé (documenté) : normalisé des deux côtés.
            let is_head_req = probe["method"].as_str() == Some("HEAD");
            let got_headers = if is_head_req {
                got_headers
                    .into_iter()
                    .filter(|(k, _)| k != "content-length")
                    .collect()
            } else {
                got_headers
            };
            let expected_headers = if is_head_req {
                expected_headers
                    .into_iter()
                    .filter(|(k, _)| k != "content-length")
                    .collect()
            } else {
                expected_headers
            };
            assert_eq!(
                got_headers, expected_headers,
                "{boot}/{name} : en-têtes (ordre/valeurs) — got={got_headers:?} want={expected_headers:?}"
            );

            let expected_body = std::fs::read_to_string(
                golden_http()
                    .join(boot)
                    .join(probe["bodyFile"].as_str().expect("bodyFile")),
            )
            .expect("body file");
            let path = probe["path"].as_str().expect("path");
            let is_get_health = probe["method"].as_str() == Some("GET")
                && (path.starts_with("/health") || path.starts_with("/healthz"));
            if is_get_health {
                // timestamp dynamique : statut + format ISO ms vérifiés
                let ts = got
                    .body
                    .strip_prefix("{\"status\":\"ok\",\"timestamp\":\"")
                    .and_then(|r| r.strip_suffix("\"}"))
                    .unwrap_or_else(|| {
                        panic!("{boot}/{name} : corps health inattendu : {}", got.body)
                    });
                assert_eq!(ts.len(), 24, "{boot}/{name} : timestamp ISO ms");
                assert!(
                    ts.ends_with('Z') && ts.as_bytes()[10] == b'T' && ts.as_bytes()[19] == b'.',
                    "{boot}/{name} : format ISO"
                );
                let _ = expected_body; // (la valeur du legacy n'est pas comparable)
            } else {
                assert_eq!(got.body, expected_body, "{boot}/{name} : corps (octets)");
            }
        }
    }
}

/// paramsHash : chaque jeu du golden (legacy parseCardOptions exécuté) est
/// recalculé par le port — JSON.stringify(options) à l'octet + hash 8 hex.
#[test]
fn params_hash_oracle_23_jeux() {
    let golden: Value = serde_json::from_str(
        &std::fs::read_to_string(golden_http().join("params-hash.json")).expect("params-hash"),
    )
    .expect("json invalide");
    let mut checked = 0;
    for entry in golden["params"].as_array().expect("params") {
        let label = entry["label"].as_str().expect("label");
        // requête brute reconstruite ; le cas « dupe-first-wins » rejoue la
        // CLÉ DEUX FOIS (premier gagne, comme hono `??=`)
        let query_str = if label == "dupe-first-wins" {
            "show_icons=true&show_icons=false".to_string()
        } else {
            entry["query"]
                .as_object()
                .expect("query objet")
                .iter()
                .map(|(k, v)| format!("{k}={}", v.as_str().unwrap_or_default()))
                .collect::<Vec<_>>()
                .join("&")
        };
        let parsed = gitcard_p0::query_params::parse_card_options(Some(&query_str))
            .unwrap_or_else(|e| panic!("{label} : parsing a échoué : {e:?}"));
        assert_eq!(
            parsed.options_json,
            entry["stringified"].as_str().expect("stringified"),
            "{label} : JSON.stringify(options)"
        );
        assert_eq!(
            gitcard_p0::params_hash(&parsed.options_json),
            entry["paramsHash"].as_str().expect("paramsHash"),
            "{label} : paramsHash"
        );
        checked += 1;
    }
    assert!(checked >= 12, "au moins 12 jeux exigés, trouvé {checked}");
    println!("params-hash : {checked} jeux identiques (stringified + 8 hex)");
}

/// SVG d'erreur : gabarit errorSvg du legacy à l'octet (tests/golden/errors/),
/// uid 512..516, md5 identiques.
#[test]
fn svg_erreur_oracle_octets() {
    let golden: Value = serde_json::from_str(
        &std::fs::read_to_string("../tests/golden/errors/manifest.json").expect("manifest errors"),
    )
    .expect("json invalide");
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/golden");
    for entry in golden["entries"].as_array().expect("entries") {
        let file = entry["file"].as_str().expect("file");
        let message = entry["message"].as_str().expect("message");
        let expected = std::fs::read_to_string(root.join(file)).expect("svg golden");
        let rendered = gitcard_p0::cards::error_svg(message);
        assert_eq!(rendered, expected, "{file} : SVG d'erreur à l'octet");
        assert_eq!(
            gitcard_p0::md5_hex(&rendered),
            entry["md5"].as_str().expect("md5")
        );
    }
    assert_eq!(golden["entries"].as_array().expect("entries").len(), 5);
    // uid « suivants » du corpus (0..511) : 512..516
    let uids: Vec<u64> = golden["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|e| e["uid"].as_u64().expect("uid"))
        .collect();
    assert_eq!(uids, vec![512, 513, 514, 515, 516], "uid 512.. contigus");
}

fn env_from_case(case: &Value) -> Vec<(&'static str, Option<String>)> {
    // les clés testées par les captures (ordres quelconques : la validation
    // suit l'ordre du schéma config.ts)
    let keys = [
        "PORT",
        "LOG_LEVEL",
        "REDIS_URL",
        "CACHE_TTL",
        "CORS_ORIGIN",
        "SENTRY_DSN",
        "SENTRY_TRACES_SAMPLE_RATE",
        "SENTRY_ENVIRONMENT",
        "PAT_1",
    ];
    keys.iter()
        .map(|k| {
            (
                *k,
                case["env"]
                    .get(*k)
                    .and_then(|v| v.as_str())
                    .map(String::from),
            )
        })
        .collect()
}

/// Bloc `❌ …` du stderr legacy (jusqu'au `]` fermant).
fn legacy_block(stderr_path: &std::path::Path) -> String {
    let raw = std::fs::read_to_string(stderr_path).expect("stderr golden");
    let mut lines = Vec::new();
    for line in raw.lines() {
        lines.push(line);
        if line == "]" {
            break;
        }
    }
    lines.join("\n")
}

/// Config : chaque cas des captures (19 boots legacy) → même verdict ok/crash
/// et, en échec, le BLOC `❌ Invalid environment variables: […]` IDENTIQUE.
#[test]
fn config_oracle_19_cas_zod() {
    let golden: Value = serde_json::from_str(
        &std::fs::read_to_string(golden_http().join("config").join("cases.json")).expect("cases"),
    )
    .expect("json invalide");
    let cfg_dir = golden_http().join("config");
    let mut checked = 0;
    for case in golden["results"].as_array().expect("results") {
        let name = case["name"].as_str().expect("name").to_string();
        let env = env_from_case(case);
        let result = load_config(&env);
        let expect_ok = case["ok"].as_bool().expect("ok");
        match result {
            Ok(cfg) => {
                assert!(expect_ok, "{name} : boot OK alors que le legacy crash");
                if name == "ok-default" {
                    assert_eq!(cfg.pats, vec!["fake-token-1"]);
                    assert_eq!(cfg.port, 3000.0);
                    assert_eq!(cfg.cache_ttl, 14400.0);
                    assert_eq!(cfg.cors_origin, None);
                    assert_eq!(cfg.log_level, "info");
                    assert_eq!(cfg.sentry_traces_sample_rate, 0.1);
                    assert_eq!(cfg.sentry_environment, "production");
                    assert_eq!(cfg.redis_url, None);
                    assert_eq!(cfg.sentry_dsn, None);
                }
            }
            Err(e) => {
                assert!(!expect_ok, "{name} : crash alors que le legacy boot");
                let expected = legacy_block(&cfg_dir.join(format!("{name}.stderr.txt")));
                assert_eq!(e.stderr_block(), expected, "{name} : bloc stderr zod");
            }
        }
        checked += 1;
    }
    assert!(checked >= 19, "19 cas attendus, trouvé {checked}");
    println!("config : {checked} cas zod identiques (verdicts + blocs stderr)");
}

// ----------------------------------------------------- cache + M-B par route

mod cache_suite {
    use super::*;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fixture_user() -> String {
        // FetchResult minimal — la section stats/streak/languages/profile
        // consommée par les rendus (fixture corpus 'typical' réutilisée)
        let golden = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/golden");
        std::fs::read_to_string(golden.join("graphql/fixtures/name-null.json")).expect("fixture")
    }

    fn state_with_github(url: String) -> AppState<NoopL2> {
        let (_, now) = clock_zero();
        let pool = PatPool::new(vec!["fake-token-1".into()], now).expect("pool");
        let mut state = prod_state(AppCache::new(NoopL2), pool, 14400.0, None);
        state.github_url = url;
        state
    }

    async fn spawn(state: AppState<NoopL2>) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let app = build_router(state);
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("serve");
        });
        addr
    }

    fn fetch_with_client(addr: SocketAddr, request: &str) -> RawResponse {
        let raw = std::thread::scope(|s| {
            let h = s.spawn(|| capture_raw(addr, request));
            h.join().expect("capture thread")
        });
        parse_response(&raw)
    }

    fn get(addr: SocketAddr, path: &str) -> RawResponse {
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            addr
        );
        fetch_with_client(addr, &req)
    }

    /// Clés de cache : SVG mis en cache (2e requête = hit, un seul fetch),
    /// données partagées entre carte et /api/data, compteurs hit/miss.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cache_hit_miss_compteurs_et_cles() {
        let server = MockServer::start().await;
        let n = Arc::new(AtomicU64::new(0));
        let n2 = n.clone();
        server
            .register(
                Mock::given(method("POST")).respond_with(move |_req: &wiremock::Request| {
                    n2.fetch_add(1, Ordering::SeqCst);
                    ResponseTemplate::new(200).set_body_string(fixture_user())
                }),
            )
            .await;
        let state = state_with_github(format!("{}/graphql", server.uri()));
        let addr = spawn(state.clone()).await;

        // /api/data D'ABORD : body = la chaîne FetchResult stockée
        let r0 = get(addr, "/api/data/alice");
        assert_eq!(r0.status_line, "HTTP/1.1 200 OK", "api/data succès");
        assert_eq!(n.load(Ordering::SeqCst), 1);
        let stored = r0.body.clone();

        let r1 = get(addr, "/stats/alice");
        assert_eq!(r1.status_line, "HTTP/1.1 200 OK", "carte stats succès");
        assert_eq!(
            n.load(Ordering::SeqCst),
            1,
            "un seul fetch GraphQL (data partagée)"
        );
        let etag1 = r1
            .headers
            .iter()
            .find(|(k, _)| k == "etag")
            .cloned()
            .expect("etag");
        let cc1 = r1
            .headers
            .iter()
            .find(|(k, _)| k == "cache-control")
            .cloned()
            .expect("cache-control");
        assert_eq!(
            cc1.1,
            "public, max-age=14400, s-maxage=14400, stale-while-revalidate=86400"
        );
        assert_eq!(etag1.1, format!("\"{}\"", gitcard_p0::md5_hex(&r1.body)));

        // 2e requête IDENTIQUE : hit SVG (clé svg:stats:alice:<hash>) → 0 fetch
        let r2 = get(addr, "/stats/alice");
        assert_eq!(r2.body, r1.body, "SVG identique depuis le cache");
        assert_eq!(n.load(Ordering::SeqCst), 1, "toujours un seul fetch");
        let (hits, misses) = state.cache.counters();
        assert!(hits >= 1, "hit SVG : {hits}");
        assert!(misses >= 2, "misses svg+data du 1er passage : {misses}");

        // carte streak MÊME username : hit data (clé data:alice partagée),
        // rendu différent, toujours 1 fetch
        let r3 = get(addr, "/stats/alice/streak");
        assert_eq!(r3.status_line, "HTTP/1.1 200 OK");
        assert_ne!(r3.body, r1.body, "streak ≠ stats");
        assert_eq!(
            n.load(Ordering::SeqCst),
            1,
            "data partagée : pas de refetch"
        );

        // /api/data hit : la CHAÎNE stockée est renvoyée telle quelle
        let r4 = get(addr, "/api/data/alice");
        assert_eq!(r4.status_line, "HTTP/1.1 200 OK");
        assert_eq!(r4.body, stored, "chaîne stockée renvoyée telle quelle");
        assert_eq!(n.load(Ordering::SeqCst), 1);
        let (hits, misses) = state.cache.counters();
        assert!(hits >= 3, "hits : svg(r2) + data(r3) + data(r4) : {hits}");
        assert!(misses >= 3, "misses : svg+data(r1) + svg(r3) : {misses}");
    }

    /// M-B au niveau ROUTE : pool épuisé → cartes 429 SVG dédié
    /// (card-factory.ts:81-85), /api/data → 500 JSON (data.ts:50-59).
    /// Les SVG d'erreur ne sont JAMAIS mis en cache (refetch à chaque fois).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn m_b_pool_epuise_429_svg_et_500_json_sans_cache() {
        let server = MockServer::start().await;
        server
            .register(
                Mock::given(method("POST"))
                    .respond_with(ResponseTemplate::new(429))
                    .expect(1),
            )
            .await;
        let state = state_with_github(format!("{}/graphql", server.uri()));
        let addr = spawn(state.clone()).await;

        // 1er fetch → RateLimit (T1 marqué échu) → get_next_token → pool
        // épuisé (un seul jeton) → carte : 429 + SVG dédié, max-age=60
        let r1 = get(addr, "/stats/bob");
        assert_eq!(
            r1.status_line, "HTTP/1.1 429 Too Many Requests",
            "M-B : statut carte"
        );
        let golden_429 = std::fs::read_to_string("../tests/golden/errors/429-all-pat.svg")
            .expect("svg 429-all-pat");
        assert_eq!(
            r1.body, golden_429,
            "M-B : SVG « All API tokens… » à l'octet"
        );
        let cc = r1
            .headers
            .iter()
            .find(|(k, _)| k == "cache-control")
            .cloned()
            .expect("cache-control");
        assert_eq!(
            cc.1,
            "public, max-age=60, s-maxage=60, stale-while-revalidate=86400"
        );

        // même requête : PAS de cache (erreur jamais cachée) → refetch
        let r2 = get(addr, "/stats/bob");
        assert_eq!(r2.status_line, "HTTP/1.1 429 Too Many Requests");
        assert_eq!(r2.body, golden_429);
        let (_, misses) = state.cache.counters();
        assert!(misses >= 4, "svg+data re-miss à chaque requête : {misses}");

        // /api/data : la même erreur pool → 500 JSON générique
        let r3 = get(addr, "/api/data/bob");
        assert_eq!(
            r3.status_line, "HTTP/1.1 500 Internal Server Error",
            "M-B : statut /api/data"
        );
        assert_eq!(r3.body, "{\"error\":\"Internal server error\"}");
        assert_eq!(
            r3.headers
                .iter()
                .find(|(k, _)| k == "content-type")
                .map(|(_, v)| v.as_str()),
            Some("application/json")
        );
    }

    /// 404 (Could not resolve to a User) : SVG dédié à l'octet, max-age=300,
    /// username BRUT non échappé dans le message.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn not_found_svg_octets_et_max_age_300() {
        let server = MockServer::start().await;
        server
            .register(Mock::given(method("POST")).respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "errors": [{"message": "Could not resolve to a User with the login of 'zz'."}]
                })),
            ))
            .await;
        let state = state_with_github(format!("{}/graphql", server.uri()));
        let addr = spawn(state).await;

        let r = get(addr, "/stats/a%3Cb%26c%22d%27e");
        assert_eq!(r.status_line, "HTTP/1.1 404 Not Found");
        // le username est décodé (hono c.req.param) puis injecté BRUT
        let golden_404 =
            std::fs::read_to_string("../tests/golden/errors/404-user-not-found.svg").expect("svg");
        assert_eq!(
            gitcard_p0::cards::error_svg("User not found: probe-user"),
            golden_404
        );
        assert!(
            r.body.contains("User not found: a<b&c\"d'e"),
            "username brut NON échappé : {}",
            r.body
        );
        let cc = r
            .headers
            .iter()
            .find(|(k, _)| k == "cache-control")
            .cloned()
            .expect("cache-control");
        assert_eq!(
            cc.1,
            "public, max-age=300, s-maxage=300, stale-while-revalidate=86400"
        );
    }
}

/// Le fetch failed transport (port fermé) → 502 SVG générique : le corps est
/// l'oracle 502 (épinglé aussi par la sonde HTTP `get_stats` du boot base).
#[test]
fn svg_502_generic_oracle() {
    let golden = std::fs::read_to_string("../tests/golden/errors/502-generic.svg").expect("svg");
    assert_eq!(
        gitcard_p0::cards::error_svg("An error occurred while generating the card."),
        golden
    );
}

/// Erreurs transport : GitHubError::AllPatTokensRateLimited garde le message
/// legacy (classe Error nu — test client P3 complété côté affichage Display).
#[test]
fn m_b_variante_affichage_legacy() {
    let e = GitHubError::AllPatTokensRateLimited;
    assert_eq!(e.to_string(), "All PAT tokens are rate-limited");
    assert_eq!(e.class_name(), "Error");
    assert_eq!(e.status(), None);
}
