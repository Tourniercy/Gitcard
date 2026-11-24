//! P5 — Sentry réel, tests hermétiques (AUCUN réseau) : transport de test en
//! mémoire `sentry::test::with_captured_envelopes_options(f, options)`, hub
//! process-wide → les blocs de capture sont sérialisés par un verrou global,
//! et le runtime tokio est current_thread DANS le bloc capturé (jamais
//! `#[tokio::test]` + `Runtime::new` : « cannot start a runtime from within
//! a runtime »). Helpers = pattern prouvé côté weight (fix5 I1).
//!
//! Ce qui est prouvé :
//! (a) capture au site github.ts (transport 500 → message + tags username/operation) ;
//! (b) requête 5xx via le ROUTEUR RÉEL → événement « [web] GET /path -> 500 » sans query ;
//! (c) transaction échantillonnée à 1.0 (op http.server, statut 200) ;
//! (d) DSN absent → aucun client (options = None ; PAS via le helper de test
//!     qui force un TEST_DSN quand dsn est None) ;
//! (e) AUCUNE enveloppe ne contient le PAT leurre / Authorization.

use gitcard_p0::app::{build_router, prod_state, AppState};
use gitcard_p0::cache::{AppCache, NoopL2};
use gitcard_p0::pat_pool::PatPool;
use gitcard_p0::sentry_gate::client_options;
use sentry::protocol::EnvelopeItem;
use sentry::test::with_captured_envelopes_options;
use serde_json::Value;
use std::sync::{Arc, Mutex, MutexGuard};

/// Verrou global : `with_captured_envelopes_options` lie le hub du process —
/// les tests parallèles se voleraient les enveloppes sans sérialisation.
static SENTRY_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Garde RAII du verrou : libère même en cas de panic (unwind).
struct TestGuard(#[allow(dead_code)] MutexGuard<'static, ()>);
fn test_guard() -> TestGuard {
    TestGuard(SENTRY_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner()))
}
impl Drop for TestGuard {
    fn drop(&mut self) {}
}

/// Payloads `event` extraits des enveloppes : (niveau, JSON sérialisé).
fn event_payloads(envelopes: &[sentry::Envelope]) -> Vec<(sentry::Level, Value)> {
    envelopes
        .iter()
        .flat_map(|e| e.items())
        .filter_map(|item| match item {
            EnvelopeItem::Event(ev) => Some((ev.level, serde_json::to_value(ev).ok()?)),
            _ => None,
        })
        .collect()
}

/// Garde (e) : aucune enveloppe ne contient un jeton (leurre des tests) ni
/// d'en-tête d'autorisation — SentryHttpLayer scrubbe les en-têtes sensibles.
fn assert_no_secret(envelopes: &[sentry::Envelope], contexte: &str) {
    let all: Vec<String> = envelopes
        .iter()
        .flat_map(|e| e.items())
        .filter_map(|item| match item {
            EnvelopeItem::Event(ev) => serde_json::to_string(ev).ok(),
            EnvelopeItem::Transaction(trx) => serde_json::to_string(trx).ok(),
            _ => None,
        })
        .collect();
    for blob in &all {
        for secret in ["fake-token-1", "bearer ", "Authorization", "authorization"] {
            assert!(
                !blob.contains(secret),
                "{contexte} : jeton/en-tête sensible dans l'enveloppe : {secret}"
            );
        }
    }
}

const TEST_DSN: &str = "https://leurre@test.example/1";

fn real_now() -> Arc<dyn Fn() -> u64 + Send + Sync> {
    Arc::new(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    })
}

/// (a) — le site github.ts capture : HTTP 500 → Api « GitHub API error: 500 »
/// → 1 event Error avec tags username/operation.
#[test]
fn a_site_github_capture_une_erreur_avec_tags() {
    let _g = test_guard();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let envelopes = with_captured_envelopes_options(
        || {
            rt.block_on(async {
                use wiremock::matchers::method;
                use wiremock::{Mock, MockServer, ResponseTemplate};
                let server = MockServer::start().await;
                server
                    .register(Mock::given(method("POST")).respond_with(ResponseTemplate::new(500)))
                    .await;
                let http = reqwest::Client::new();
                let err = gitcard_p0::github::fetch_github_data_at(
                    &http,
                    &format!("{}/graphql", server.uri()),
                    "leurre-user",
                    "fake-token-1",
                )
                .await
                .expect_err("500 → Api");
                assert!(matches!(err, gitcard_p0::github::GitHubError::Api { .. }));
            });
        },
        client_options(TEST_DSN, "production", 0.0).expect("options valides"),
    );
    let events = event_payloads(&envelopes);
    assert_eq!(events.len(), 1, "exactement 1 event : {events:?}");
    assert_eq!(events[0].0, sentry::Level::Error, "niveau error");
    assert_eq!(
        events[0].1["message"], "GitHub API error: 500 Internal Server Error",
        "message capture au site github.ts"
    );
    assert_eq!(
        events[0].1["tags"]["username"], "leurre-user",
        "tag username"
    );
    assert_eq!(
        events[0].1["tags"]["operation"], "fetchGitHubData",
        "tag operation"
    );
    assert_no_secret(&envelopes, "(a)");
}

/// (b)+(c) — requête 5xx via le ROUTEUR RÉEL → événement « [web] … » ;
/// /healthz à sample 1.0 → transaction op http.server statut 200.
#[test]
fn b_5xx_routeur_reel_et_transaction_echantillonnee() {
    let _g = test_guard();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let envelopes = with_captured_envelopes_options(
        || {
            rt.block_on(async {
                use tower::ServiceExt; // oneshot
                use wiremock::matchers::method;
                use wiremock::{Mock, MockServer, ResponseTemplate};

                // 500 provoqué par le STUB (HTTP 500 → Api → 500 JSON data.ts)
                let server = MockServer::start().await;
                server
                    .register(Mock::given(method("POST")).respond_with(ResponseTemplate::new(500)))
                    .await;
                let pool = PatPool::new(vec!["fake-token-1".into()], real_now()).expect("pool");
                let mut state: AppState<NoopL2> =
                    prod_state(AppCache::new(NoopL2), pool, 14400.0, None);
                state.github_url = format!("{}/graphql", server.uri());
                let app = build_router(state);

                let res = app
                    .clone()
                    .oneshot(
                        axum::http::Request::builder()
                            .uri("/api/data/leurre-user")
                            .body(axum::body::Body::empty())
                            .expect("requête"),
                    )
                    .await
                    .expect("réponse");
                assert_eq!(res.status().as_u16(), 500, "5xx provoqué");

                // transaction : /healthz à sample 1.0
                let res2 = app
                    .oneshot(
                        axum::http::Request::builder()
                            .uri("/healthz")
                            .body(axum::body::Body::empty())
                            .expect("requête"),
                    )
                    .await
                    .expect("réponse");
                assert_eq!(res2.status().as_u16(), 200);
            });
        },
        client_options(TEST_DSN, "production", 1.0).expect("options valides"),
    );
    let events = event_payloads(&envelopes);
    let web: Vec<_> = events
        .iter()
        .filter(|(_, p)| p["message"].as_str().unwrap_or("").contains("[web]"))
        .collect();
    assert_eq!(web.len(), 1, "exactement 1 event [web] : {events:?}");
    assert_eq!(
        web[0].1["message"], "[web] GET /api/data/leurre-user -> 500",
        "méthode + chemin SANS query"
    );
    assert!(
        events
            .iter()
            .all(|(_, p)| !p["message"].as_str().unwrap_or("").contains('?')),
        "aucune query dans l'événement"
    );

    // (c) : une transaction par requête (sample 1.0), op http.server
    let mut transactions = Vec::new();
    for env in &envelopes {
        for item in env.items() {
            if let EnvelopeItem::Transaction(trx) = item {
                transactions.push(serde_json::to_value(trx).expect("trx sérialisable"));
            }
        }
    }
    let health: Vec<_> = transactions
        .iter()
        .filter(|t| t["transaction"] == "GET /healthz")
        .collect();
    assert_eq!(health.len(), 1, "transaction /healthz : {transactions:?}");
    assert_eq!(health[0]["contexts"]["trace"]["op"], "http.server");
    assert_eq!(
        health[0]["contexts"]["trace"]["data"]["http.response.status_code"], 200,
        "statut de réponse attaché à la span"
    );
    assert_no_secret(&envelopes, "(b)");
}

/// (d) — DSN absent : `client_options` = None, init sans effet ; le helper
/// de test (qui force un TEST_DSN quand dsn est None) n'est PAS utilisé ici.
#[test]
fn d_dsn_absent_aucun_client() {
    assert!(client_options("", "production", 0.1).is_none(), "DSN vide");
    assert!(
        client_options("pas-un-dsn", "production", 0.1).is_none(),
        "DSN mal formé"
    );
    let init = gitcard_p0::sentry_gate::SentryGate::init("", "production", 0.1);
    assert!(!init.gate.enabled, "gate désactivée, boot poursuivi");
    let bad = gitcard_p0::sentry_gate::SentryGate::init("https://x", "production", 0.1);
    assert!(
        !bad.gate.enabled,
        "DSN URL-valide mais non-Sentry : soft-fail"
    );
}

/// (e) — options reproduisent telemetry.ts : environment legacy (`??`),
/// dsn posé. Le TAUX lui-même est prouvé COMPORTEMENTALEMENT ((b) : sample
/// 1.0 → transaction émise ; pas de getter 0.49 sur `traces_sample_rate`,
/// `ClientOptions` non_exhaustif — cf. Journal weight fix5).
#[test]
fn e_options_reproduisent_telemetry_ts() {
    let opts = client_options("https://k@sentry.invalid/1", "ma-prod", 0.25).expect("options");
    assert_eq!(opts.environment.as_deref(), Some("ma-prod"));
    assert!(opts.dsn.is_some(), "dsn posé");
    // sémantique Number() : vide → 0 (taux nul = aucune transaction) et
    // absent → 0.1 — comportements éprouvés côté config.rs (telemetry).
    assert!(client_options("https://k@sentry.invalid/1", "p", 0.0).is_some());
    assert!(client_options("https://k@sentry.invalid/1", "p", 0.1).is_some());
    assert!(client_options("", "production", 0.1).is_none(), "DSN vide");
}

/// (f) — métriques : L'API Metrics du SDK Rust 0.49 (feature `metrics`) est
/// réelle ; les métriques sont batchées (flush à 100 items — batcher.rs
/// MAX_ITEMS) : une rafale de 100 `metric_count` force le flush → 1
/// enveloppe `Metrics` avec les noms/types/attributs du legacy.
#[test]
fn f_metriques_compteurs_flushees_par_rafale_100() {
    let _g = test_guard();
    let envelopes = with_captured_envelopes_options(
        || {
            // 50 hits + 50 cards (attributes card_type) = 100 items → flush
            for _ in 0..50 {
                gitcard_p0::sentry_gate::metric_count("cache_hits_total", &[]);
            }
            for _ in 0..50 {
                gitcard_p0::sentry_gate::metric_count(
                    "cards_generated_total",
                    &[("card_type", "stats")],
                );
            }
        },
        client_options(TEST_DSN, "production", 0.0).expect("options valides"),
    );
    let mut all: Vec<Value> = Vec::new();
    for env in &envelopes {
        for item in env.items() {
            // les métriques voyagent dans un ItemContainer (batch du client)
            if let EnvelopeItem::ItemContainer(sentry::protocol::ItemContainer::Metrics(m)) = item {
                for x in m {
                    all.push(serde_json::to_value(x).expect("metric sérialisable"));
                }
            }
        }
    }
    assert!(
        !all.is_empty(),
        "au moins une enveloppe Metrics après flush : {envelopes:?}"
    );
    let hits = all
        .iter()
        .filter(|m| m["name"] == "cache_hits_total")
        .count();
    let cards = all
        .iter()
        .filter(|m| m["name"] == "cards_generated_total")
        .count();
    assert_eq!(hits, 50, "50 compteurs cache_hits_total : {hits}");
    assert_eq!(cards, 50, "50 compteurs cards_generated_total : {cards}");
    assert!(
        all.iter().all(|m| m["type"] == "counter"),
        "type counter : {all:?}"
    );
    let card_attr = all
        .iter()
        .find(|m| m["name"] == "cards_generated_total")
        .expect("card metric");
    assert_eq!(
        card_attr["attributes"]["card_type"]["value"], "stats",
        "attribut card_type du legacy (LogAttribute {{type, value}})"
    );
    assert_no_secret(&envelopes, "(f)");
}

/// (g) — distribution avec unité milliseconde (github_api_duration).
#[test]
fn g_distribution_avec_unite_milliseconde() {
    let _g = test_guard();
    let envelopes = with_captured_envelopes_options(
        || {
            for _ in 0..100 {
                gitcard_p0::sentry_gate::metric_distribution_ms("github_api_duration", 12.5);
            }
        },
        client_options(TEST_DSN, "production", 0.0).expect("options valides"),
    );
    let mut found = 0;
    for env in &envelopes {
        for item in env.items() {
            if let EnvelopeItem::ItemContainer(sentry::protocol::ItemContainer::Metrics(metrics)) =
                item
            {
                for x in metrics {
                    let m = serde_json::to_value(x).expect("metric sérialisable");
                    if m["name"] == "github_api_duration" {
                        found += 1;
                        assert_eq!(m["type"], "distribution");
                        assert_eq!(m["unit"], "millisecond");
                    }
                }
            }
        }
    }
    assert_eq!(found, 100, "100 distributions flushees : {found}");
    assert_no_secret(&envelopes, "(g)");
}
