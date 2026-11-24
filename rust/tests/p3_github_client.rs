//! P3 — client GitHub GraphQL contre `wiremock` lié sur 127.0.0.1 (aucun appel
//! réseau réel, valable sous `--network none`) : corps de requête attendu =
//! OCTETS capturés du legacy (`requests.json`), en-têtes (`bearer` b minuscule,
//! User-Agent, Content-Type), mapping d'erreurs 403/429 ± en-têtes, 401, 500,
//! `errors` (inconnu / autre), `data.user` nul, corps invalide, timeout, retry
//! avec le second jeton (URL injectable — jamais api.github.com).

use gitcard_p0::github::{
    fetch_github_data_at, fetch_with_retry_at, GITHUB_GRAPHQL_URL, USER_QUERY,
};
use gitcard_p0::pat_pool::PatPool;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn golden() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/golden")
}

fn fixture_json(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            golden()
                .join("graphql/fixtures")
                .join(format!("{name}.json")),
        )
        .expect("fixture illisible"),
    )
    .expect("fixture invalide")
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().build().expect("client reqwest")
}

const T1: &str = "fake-token-1";
const T2: &str = "fake-token-2";

fn ok_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(fixture_json("name-null"))
}

/// Corps attendu = corps CAPTURÉ du legacy pour la fixture name-null
/// (JSON.stringify({query, variables:{login}}), requête verbatim incluse).
#[tokio::test]
async fn corps_et_en_tetes_identiques_au_legacy() {
    let server = MockServer::start().await;
    let requests: Value = serde_json::from_str(
        &std::fs::read_to_string(golden().join("graphql/requests.json")).expect("requests"),
    )
    .expect("requests.json invalide");
    let legacy = &requests["requests"]["name-null"];
    let legacy_body = legacy["body"].as_str().expect("body");
    let legacy_url = legacy["url"].as_str().expect("url");
    assert_eq!(legacy_url, GITHUB_GRAPHQL_URL, "URL du legacy");
    // Preuve oracle du schéma d'autorisation : la capture du legacy porte
    // `bearer` MINUSCULE (prouvé aussi par comptage greps, cf. rapport).
    assert_eq!(
        legacy["headers"]["Authorization"]
            .as_str()
            .expect("Authorization capturé"),
        format!("bearer {T1}"),
        "schéma d'autorisation du legacy = `bearer <token>` (b minuscule)"
    );
    // La requête verbatim extraite par le script est incluse par le port.
    let expected_query =
        std::fs::read_to_string(golden().join("graphql/user-query.txt")).expect("user-query.txt");
    assert_eq!(USER_QUERY, expected_query, "requête verbatim incluse");

    let mock = Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(header("Authorization", format!("bearer {T1}")))
        .and(header("Content-Type", "application/json"))
        .and(header("User-Agent", "github-stats-cards"))
        .and(body_string(legacy_body.to_string()))
        .respond_with(ok_response());
    server.register(mock).await;

    // Chemin complet du legacy : <origin>/graphql (le mock exige le path,
    // l'URL réelle étant https://api.github.com/graphql).
    let result = fetch_github_data_at(
        &client(),
        &format!("{}/graphql", server.uri()),
        "anon-dev",
        T1,
    )
    .await
    .expect("doit réussir");
    // Parité bytes : le FetchResult Rust = JSON.stringify legacy (name-null).
    let expected = std::fs::read_to_string(golden().join("graphql/name-null.expected.json"))
        .expect("expected");
    assert_eq!(result.to_json_string(), expected, "FetchResult byte-parité");
}

#[tokio::test]
async fn errors_utilisateur_inconnu_notfound() {
    let server = MockServer::start().await;
    server
        .register(Mock::given(method("POST")).respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errors": [{"message": "Could not resolve to a User with the login of 'zz'."}]
            })),
        ))
        .await;
    let err = fetch_github_data_at(&client(), &server.uri(), "zz", T1)
        .await
        .expect_err("NotFound");
    assert_eq!(err.class_name(), "GitHubNotFoundError");
    assert_eq!(err.status(), Some(404));
    assert_eq!(err.to_string(), "User not found: zz");
}

#[tokio::test]
async fn errors_autres_graphql_errors_502_message() {
    let server = MockServer::start().await;
    server
        .register(Mock::given(method("POST")).respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errors": [{"message": "Something else"}, {"message": "and more"}]
            })),
        ))
        .await;
    let err = fetch_github_data_at(&client(), &server.uri(), "u", T1)
        .await
        .expect_err("Api");
    assert_eq!(err.class_name(), "GitHubApiError");
    assert_eq!(err.status(), None);
    assert_eq!(err.to_string(), "GraphQL errors: Something else, and more");
}

#[tokio::test]
async fn data_user_nul_notfound() {
    let server = MockServer::start().await;
    server
        .register(Mock::given(method("POST")).respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {"user": null}})),
        ))
        .await;
    let err = fetch_github_data_at(&client(), &server.uri(), "ghost", T1)
        .await
        .expect_err("NotFound");
    assert_eq!(err.class_name(), "GitHubNotFoundError");
}

#[tokio::test]
async fn limite_403_avec_en_tetes() {
    let server = MockServer::start().await;
    server
        .register(
            Mock::given(method("POST")).respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-ratelimit-remaining", "7")
                    .insert_header("x-ratelimit-reset", "1700000000"),
            ),
        )
        .await;
    let err = fetch_github_data_at(&client(), &server.uri(), "u", T1)
        .await
        .expect_err("RateLimit");
    assert_eq!(err.class_name(), "GitHubRateLimitError");
    assert_eq!(err.status(), Some(429));
    match &err {
        gitcard_p0::github::GitHubError::RateLimit {
            remaining,
            reset_at_ms,
        } => {
            assert_eq!(*remaining, 7.0);
            assert_eq!(*reset_at_ms, Some(1_700_000_000_000.0)); // epoch s → ms
        }
        other => panic!("attendu RateLimit, obtenu {other:?}"),
    }
    assert_eq!(err.to_string(), "GitHub API rate limit exceeded");
}

#[tokio::test]
async fn limite_429_sans_en_tetes_defauts() {
    let server = MockServer::start().await;
    server
        .register(Mock::given(method("POST")).respond_with(ResponseTemplate::new(429)))
        .await;
    let err = fetch_github_data_at(&client(), &server.uri(), "u", T1)
        .await
        .expect_err("RateLimit");
    match &err {
        gitcard_p0::github::GitHubError::RateLimit {
            remaining,
            reset_at_ms,
        } => {
            assert_eq!(*remaining, 0.0, "Number(header ?? 0)");
            assert_eq!(*reset_at_ms, None);
        }
        other => panic!("attendu RateLimit, obtenu {other:?}"),
    }
}

#[tokio::test]
async fn statut_401_message_legacy() {
    let server = MockServer::start().await;
    server
        .register(Mock::given(method("POST")).respond_with(ResponseTemplate::new(401)))
        .await;
    let err = fetch_github_data_at(&client(), &server.uri(), "u", T1)
        .await
        .expect_err("Api");
    assert_eq!(err.to_string(), "GitHub API error: 401 Unauthorized");
    assert_eq!(err.status(), Some(401));
}

#[tokio::test]
async fn statut_500_message_legacy() {
    let server = MockServer::start().await;
    server
        .register(Mock::given(method("POST")).respond_with(ResponseTemplate::new(500)))
        .await;
    let err = fetch_github_data_at(&client(), &server.uri(), "u", T1)
        .await
        .expect_err("Api");
    assert_eq!(
        err.to_string(),
        "GitHub API error: 500 Internal Server Error"
    );
}

#[tokio::test]
async fn corps_invalide_echoue() {
    let server = MockServer::start().await;
    server
        .register(
            Mock::given(method("POST")).respond_with(
                ResponseTemplate::new(200).set_body_string("<html>pas du JSON</html>"),
            ),
        )
        .await;
    let err = fetch_github_data_at(&client(), &server.uri(), "u", T1)
        .await
        .expect_err("parse doit échouer");
    assert_eq!(err.class_name(), "GitHubApiError");
    assert_eq!(err.status(), None);
}

#[tokio::test]
async fn timeout_transport_fetch_failed() {
    let server = MockServer::start().await;
    server
        .register(
            Mock::given(method("POST"))
                .respond_with(ok_response().set_delay(Duration::from_secs(5))),
        )
        .await;
    let timeout_client = reqwest::Client::builder()
        .timeout(Duration::from_millis(200))
        .build()
        .expect("client");
    let err = fetch_github_data_at(&timeout_client, &server.uri(), "u", T1)
        .await
        .expect_err("timeout");
    assert_eq!(err.to_string(), "fetch failed");
    assert_eq!(err.status(), None);
}

#[tokio::test]
async fn retry_avec_le_second_jeton_puis_succes() {
    let server = MockServer::start().await;
    let (_, now) = clock_zero();
    let mut pool = PatPool::new(vec![T1.into(), T2.into()], now).expect("pool");
    server
        .register(
            Mock::given(header("Authorization", format!("bearer {T1}")))
                .respond_with(ResponseTemplate::new(429)),
        )
        .await;
    // Second essai : le corps du legacy = {"query":<verbatim>,"variables":{"login":…}}
    // (même construction que le port ; la parité bytes est prouvée par le test
    // `corps_et_en_tetes_identiques_au_legacy` contre requests.json).
    server
        .register(
            Mock::given(header("Authorization", format!("bearer {T2}")))
                .and(body_string(format!(
                    "{{\"query\":{},\"variables\":{{\"login\":\"anon-dev\"}}}}",
                    quote_query()
                )))
                .respond_with(ok_response()),
        )
        .await;
    let result = fetch_with_retry_at(&client(), &mut pool, &server.uri(), "anon-dev")
        .await
        .expect("retry doit réussir avec T2");
    let expected = std::fs::read_to_string(golden().join("graphql/name-null.expected.json"))
        .expect("expected");
    assert_eq!(result.to_json_string(), expected);
}

#[tokio::test]
async fn retry_second_echec_se_propage() {
    let server = MockServer::start().await;
    let (_, now) = clock_zero();
    let mut pool = PatPool::new(vec![T1.into(), T2.into()], now).expect("pool");
    server
        .register(Mock::given(method("POST")).respond_with(ResponseTemplate::new(429)))
        .await;
    let err = fetch_with_retry_at(&client(), &mut pool, &server.uri(), "u")
        .await
        .expect_err("le second échec se propage");
    assert_eq!(err.class_name(), "GitHubRateLimitError");
}

#[tokio::test]
async fn limite_403_en_tetes_vides_zero_et_reset_undefined() {
    // Revue P3 M-A : `Number("")` = 0 (header présent mais VIDE) ; côté reset,
    // `"" ? … : undefined` (falsy) → undefined — le port donnait NaN/Some(NaN).
    let server = MockServer::start().await;
    server
        .register(
            Mock::given(method("POST")).respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-ratelimit-remaining", "")
                    .insert_header("x-ratelimit-reset", ""),
            ),
        )
        .await;
    let err = fetch_github_data_at(&client(), &server.uri(), "u", T1)
        .await
        .expect_err("RateLimit");
    match &err {
        gitcard_p0::github::GitHubError::RateLimit {
            remaining,
            reset_at_ms,
        } => {
            assert_eq!(*remaining, 0.0, "Number(\"\") = 0");
            assert_eq!(*reset_at_ms, None, "reset vide (falsy) → undefined");
        }
        other => panic!("attendu RateLimit, obtenu {other:?}"),
    }
}

#[tokio::test]
async fn limite_403_en_tete_non_numerique_nan() {
    // Revue P3 M-D : `Number("abc")` = NaN (Invalid Date côté reset) — branche
    // codée mais jusque-là sans test.
    let server = MockServer::start().await;
    server
        .register(
            Mock::given(method("POST")).respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-ratelimit-remaining", "abc")
                    .insert_header("x-ratelimit-reset", "abc"),
            ),
        )
        .await;
    let err = fetch_github_data_at(&client(), &server.uri(), "u", T1)
        .await
        .expect_err("RateLimit");
    match &err {
        gitcard_p0::github::GitHubError::RateLimit {
            remaining,
            reset_at_ms,
        } => {
            assert!(remaining.is_nan(), "Number(\"abc\") = NaN");
            assert!(
                reset_at_ms.is_some_and(|ms| ms.is_nan()),
                "Invalid Date → NaN"
            );
        }
        other => panic!("attendu RateLimit, obtenu {other:?}"),
    }
}

#[tokio::test]
async fn echec_pool_se_propage_en_variante_dediee() {
    // Revue P3 M-B : l'appel `getNextToken` du legacy est HORS try
    // (github.ts:168) → l'erreur du pool se propage BRUTE, pas encapsulée en
    // GitHubApiError. La variante dédiée laisse les routes de la phase 4
    // produire 429 (cartes, card-factory.ts:81-85) / 500 (/api/data,
    // data.ts:50-59) au lieu d'un 502 générique.
    let server = MockServer::start().await;
    let (_, now) = clock_zero();
    let mut pool = PatPool::new(vec![T1.into()], now).expect("pool");
    pool.mark_exhausted(T1);
    let err = fetch_with_retry_at(&client(), &mut pool, &server.uri(), "u")
        .await
        .expect_err("pool épuisé doit lever");
    assert_eq!(err.class_name(), "Error", "classe legacy = Error nu");
    assert_eq!(err.status(), None);
    assert_eq!(err.to_string(), "All PAT tokens are rate-limited");
}

fn clock_zero() -> (Arc<AtomicU64>, Arc<dyn Fn() -> u64 + Send + Sync>) {
    let cell = Arc::new(AtomicU64::new(0));
    let read = {
        let cell = cell.clone();
        Arc::new(move || cell.load(Ordering::SeqCst)) as Arc<dyn Fn() -> u64 + Send + Sync>
    };
    (cell, read)
}

/// Corps legacy attendu pour le mock du retry : {query verbatim, login}
/// sérialisé par le même échappement que le port (la parité bytes est déjà
/// prouvée par `corps_et_en_tetes_identiques_au_legacy`).
fn quote_query() -> String {
    gitcard_p0::js_json::JsJson::Str(USER_QUERY.to_string()).to_json_string()
}
