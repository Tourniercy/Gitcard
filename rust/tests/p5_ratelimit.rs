//! P5 — parité rate-limiter contre l'ORACLE du legacy exécuté hors réseau
//! (`tests/golden/ratelimit/`, captures socket brute par
//! `scripts/gen-ratelimit-golden.mjs` : hono-rate-limiter 0.5.3 réel).
//!
//! Chaque boot est rejoué contre le ROUTEUR RÉEL du port (serveur
//! 127.0.0.1:0 avec `ConnectInfo` comme la prod) : requête OCTET pour OCTET
//! (Host réécrit), réponse comparée — ligne de statut, liste ORDONNÉE des
//! en-têtes (Date exclu : dynamique), corps à l'octet quand l'oracle le
//! stocke. GitHub stubbé par wiremock (127.0.0.1, `--network none`).
//!
//! AUCUNE attente réelle : l'horloge du limiteur est une cellule virtuelle
//! épinglée au boot (les 6 boots legacy tournent en < 1 s → tous leurs
//! `ratelimit-reset` sont 60) ; le boot `reset_61s` avance la cellule
//! exactement comme le parent legacy a attendu (1 250 ms puis 60 250 ms).
//! La preuve de la fenêtre réelle (61 s) reste côté legacy — une seule fois.

use gitcard_p0::app::{build_router, prod_state, AppState};
use gitcard_p0::cache::{AppCache, NoopL2};
use gitcard_p0::pat_pool::PatPool;
use gitcard_p0::ratelimit::RateLimiter;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn golden_rl() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/golden/ratelimit")
}

/// Horloge réelle (epoch ms) pour le pool PAT (bookkeeping, non épingle).
fn real_now() -> Arc<dyn Fn() -> u64 + Send + Sync> {
    Arc::new(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    })
}

/// Cellule virtuelle épinglée au boot (limiteur).
fn pinned_clock() -> (Arc<AtomicU64>, Arc<dyn Fn() -> u64 + Send + Sync>) {
    let cell = Arc::new(AtomicU64::new(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    ));
    let read = {
        let cell = cell.clone();
        Arc::new(move || cell.load(Ordering::SeqCst)) as Arc<dyn Fn() -> u64 + Send + Sync>
    };
    (cell, read)
}

fn fixture_body() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/golden/graphql/fixtures/name-null.json"),
    )
    .expect("fixture graphql")
}

/// Stub GitHub (équivalent process du stub `globalThis.fetch` du legacy) :
/// `ok` → fixture ; `by-user` → 404 GraphQL / 500 pour les users désignés.
async fn spawn_github_stub(stub: &Value) -> String {
    use wiremock::matchers::{body_partial_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let fixture = ResponseTemplate::new(200).set_body_string(fixture_body());
    let mode = stub["mode"].as_str().unwrap_or("ok");
    if mode == "fail" {
        // stub throw → transport en échec pour TOUT user (500 générique)
        server
            .register(Mock::given(method("POST")).respond_with(ResponseTemplate::new(500)))
            .await;
        return format!("{}/graphql", server.uri());
    }
    if mode == "by-user" {
        let not_found_user = stub["notFoundUser"].as_str().expect("notFoundUser");
        server
            .register(
                Mock::given(method("POST"))
                    .and(body_partial_json(json!({
                        "variables": {"login": not_found_user}
                    })))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                        "errors": [{
                            "message":
                                format!("Could not resolve to a User with the login of '{not_found_user}'.")
                        }]
                    }))),
            )
            .await;
        let fail_user = stub["failUser"].as_str().expect("failUser");
        server
            .register(
                Mock::given(method("POST"))
                    .and(body_partial_json(json!({
                        "variables": {"login": fail_user}
                    })))
                    // `fetch failed` legacy et 500 GitHub mènent au MÊME catch
                    // générique de data.ts → 500 JSON (observable identique).
                    .respond_with(ResponseTemplate::new(500)),
            )
            .await;
    }
    server
        .register(Mock::given(method("POST")).respond_with(fixture))
        .await;
    format!("{}/graphql", server.uri())
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

fn parse_response(raw: &str) -> (String, Vec<(String, String)>, String) {
    let (head, body) = match raw.split_once("\r\n\r\n") {
        Some((h, b)) => (h, b.to_string()),
        None => (raw, String::new()),
    };
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default().to_string();
    let mut headers = Vec::new();
    for line in lines {
        if let Some((n, v)) = line.split_once(':') {
            let name = n.trim().to_ascii_lowercase();
            headers.push((name, v.trim().to_string()));
        }
    }
    (status_line, headers, body)
}

/// Rejeu des 6 boots de l'oracle : chaque sonde est comparée OCTET pour
/// OCTET (hors Date) — c'est le test qui échoue sans le rate-limiter
/// (la 31e requête répondrait 200 sans en-têtes au lieu du 429 oracle).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rate_limiter_oracle_boots_octets() {
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(golden_rl().join("manifest.json")).expect("manifeste"),
    )
    .expect("manifeste invalide");

    for boot in manifest["boots"].as_array().expect("boots") {
        let name = boot["name"].as_str().expect("boot name").to_string();
        let (cell, limiter_clock) = pinned_clock();
        let pool = PatPool::new(vec!["fake-token-1".into()], real_now()).expect("pool");
        let mut state = prod_state(AppCache::new(NoopL2), pool, 14400.0, None);
        state.github_url = spawn_github_stub(&boot["stub"]).await;
        state.limiter = Arc::new(RateLimiter::new(limiter_clock));
        let addr = spawn_server(state).await;

        for probe in boot["probes"].as_array().expect("probes") {
            let pname = probe["name"].as_str().expect("name");
            // avance l'horloge virtuelle comme le parent legacy a attendu
            if let Some(delay) = probe["delayMs"].as_u64() {
                cell.fetch_add(delay, Ordering::SeqCst);
            }
            let request = probe["request"].as_str().expect("request");
            let rewritten = request.replace("127.0.0.1:3000", &addr.to_string());
            let raw = std::thread::scope(|s| {
                let h = s.spawn(|| capture_raw(addr, &rewritten));
                h.join().expect("capture thread")
            });
            let (status_line, headers, body) = parse_response(&raw);

            assert_eq!(
                status_line,
                probe["statusLine"].as_str().expect("statusLine"),
                "{name}/{pname} : ligne de statut"
            );

            let expected: Vec<(String, String)> = probe["headers"]
                .as_array()
                .expect("headers")
                .iter()
                .map(|h| {
                    let s = h.as_str().expect("header str");
                    let (n, v) = s.split_once(':').expect("header format");
                    (n.trim().to_ascii_lowercase(), v.trim().to_string())
                })
                .filter(|(n, _)| n != "date")
                .collect();
            let got: Vec<(String, String)> =
                headers.into_iter().filter(|(n, _)| n != "date").collect();
            assert_eq!(
                got, expected,
                "{name}/{pname} : en-têtes (ordre/valeurs) — got={got:?} want={expected:?}"
            );

            if let Some(body_file) = probe["bodyFile"].as_str() {
                let expected_body =
                    std::fs::read_to_string(golden_rl().join(body_file)).expect("body oracle");
                assert_eq!(body, expected_body, "{name}/{pname} : corps (octets)");
            }
        }
    }
}

/// Le chemin nu `/api/data` et les suffixes profonds sont bien comptés (glob
/// hono `/api/data/*`, oracle boot `paths`) : chaque requête décrémente
/// `remaining` même quand le fallback notFound répond 404.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn glob_compte_les_chemins_non_routes() {
    let (_, limiter_clock) = pinned_clock();
    let pool = PatPool::new(vec!["fake-token-1".into()], real_now()).expect("pool");
    let mut state = prod_state(AppCache::new(NoopL2), pool, 14400.0, None);
    state.github_url = "http://127.0.0.1:1/graphql".to_string();
    state.limiter = Arc::new(RateLimiter::new(limiter_clock));
    let addr = spawn_server(state).await;

    let get = |addr: SocketAddr, path: &str| {
        let req = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        let raw = std::thread::scope(|s| {
            let h = s.spawn(|| capture_raw(addr, &req));
            h.join().expect("capture")
        });
        parse_response(&raw)
    };

    let (st1, h1, _) = get(addr, "/api/data");
    let (st2, h2, _) = get(addr, "/api/data/");
    let (st3, h3, _) = get(addr, "/api/data/deep/a/b");
    for (st, h) in [(st1, &h1), (st2, &h2), (st3, &h3)] {
        assert_eq!(st, "HTTP/1.1 404 Not Found", "fallback notFound");
        let rem = h
            .iter()
            .find(|(n, _)| n == "ratelimit-remaining")
            .expect("remaining");
        assert!(rem.1.parse::<u64>().is_ok(), "remaining présent : {rem:?}");
    }
    let r1 = h1.iter().find(|(n, _)| n == "ratelimit-remaining").unwrap();
    let r3 = h3.iter().find(|(n, _)| n == "ratelimit-remaining").unwrap();
    assert_eq!(
        r1.1.parse::<u64>().unwrap() - r3.1.parse::<u64>().unwrap(),
        2,
        "deux requêtes comptées sur le chemin nu / profond"
    );
    // hors glob : aucun en-tête ratelimit
    let (_, h4, _) = get(addr, "/api/themes");
    assert!(
        h4.iter().all(|(n, _)| !n.starts_with("ratelimit-")),
        "aucun ratelimit-* sur /api/themes"
    );
}
