//! Sentry/GlitchTip serveur — port de `apps/api/src/telemetry.ts` +
//! `index.ts:54-58`, initialisation **soft-fail** : active seulement si
//! `SENTRY_DSN` est défini, sinon désactivée ; DSN mal formé → avertissement
//! sans crash (la façade Rust `sentry::init` PANIQUE sur un DSN invalide —
//! le parse est donc fait AVANT).
//!
//! Pile transport : crate `sentry` SANS les features par défaut — le
//! transport par défaut de la façade 0.49 est `reqwest`+`native-tls`
//! (OpenSSL, incompilable sur musl/alpine) → transport **ureq + rustls/ring**,
//! la MÊME pile TLS que le `reqwest` du projet (prouvé côté weight :
//! `cargo tree -e features --target x86_64-unknown-linux-musl`, 0 arête
//! native-tls/openssl/aws-lc — rapports rust-weight-fix5 §I1).
//!
//! Ce qui remonte, à l'image du legacy `@sentry/node ^10.47.0` :
//! - panics (intégration `panic`, chaîne vers le hook par défaut) ;
//! - erreurs de fetch GitHub non-404/429 (github.ts:153-160
//!   `captureException` : transport, statut HTTP, JSON, GraphQL errors hors
//!   NotFound — tags `username` + extra `operation: 'fetchGitHubData'`) ;
//! - toute réponse 5xx — middleware maison (`app.rs`
//!   `sentry_server_error_mw`, « [web] METHOD /path -> code », méthode +
//!   chemin SANS query) : sentry-tower 0.49 ne capture PAS les réponses 5xx
//!   (il ne pose que le statut de la span) ;
//! - transactions/spans HTTP échantillonnées à `tracesSampleRate`
//!   (`SentryHttpLayer::enable_transaction`, équivalent de l'instrumentation
//!   http.server du SDK JS) + contexte requête (en-têtes sensibles filtrés
//!   par sentry-tower — vérifié dans sa source : Authorization/cookie/Cookie
//!   retirés, `testé` en p5_sentry.rs).
//!
//! DIVERGENCES ASSUMÉES (rapport P5 §Sentry) :
//! - `Sentry.metrics.*` du legacy : port dans le commit métriques (l'API
//!   existe dans la crate 0.49, feature `metrics` — voir sujet C) ;
//! - `Sentry.init` s'exécute à l'IMPORT du legacy (avant la validation zod) ;
//!   le port initialise après `load_config` — non observable : le crash zod
//!   reste byte-exact, et l'init JS avec DSN invalide n'émet rien d'observable ;
//! - la garde client vit toute la vie du process (drop = flush, comme la
//!   fin de process legacy).

use sentry::types::Dsn;

/// État du gate Sentry serveur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SentryGate {
    pub enabled: bool,
}

/// Client Sentry + garde — la garde DOIT vivre pendant toute la vie du
/// processus (`main.rs` la conserve dans une liaison du runtime) : le drop
/// vide la file d'envoi et ferme le transport, comme la fin de process legacy.
pub struct SentryInit {
    pub gate: SentryGate,
    /// `None` = gate désactivée (aucun client global posé).
    _guard: Option<sentry::ClientInitGuard>,
}

impl SentryGate {
    /// `if (dsn) { Sentry.init({dsn, tracesSampleRate, environment}) }`
    /// (telemetry.ts). Soft-fail : DSN PRÉSENT mais mal formé → warn, boot
    /// poursuivi ; DSN vide → silencieux (comme le legacy sans SENTRY_DSN).
    pub fn init(dsn: &str, environment: &str, traces_sample_rate: f64) -> SentryInit {
        let off = SentryInit {
            gate: SentryGate { enabled: false },
            _guard: None,
        };
        match client_options(dsn, environment, traces_sample_rate) {
            None => {
                if !dsn.is_empty() {
                    eprintln!("warn: sentry-sdk non démarré : DSN invalide");
                }
                off
            }
            Some(opts) => {
                let guard = sentry::init(opts);
                SentryInit {
                    gate: SentryGate { enabled: true },
                    _guard: Some(guard),
                }
            }
        }
    }
}

/// Options Sentry réelles du gate — `None` si désactivé (DSN absent ou mal
/// formé). Fonction pure, sans effet de bord : utilisée par `init` (prod) et
/// par les tests hermétiques (`sentry::test` : transport en mémoire, jamais
/// le transport ureq).
pub fn client_options(
    dsn: &str,
    environment: &str,
    traces_sample_rate: f64,
) -> Option<sentry::ClientOptions> {
    if dsn.is_empty() {
        return None;
    }
    // Soft-fail : DSN mal formé → None (`Dsn::parse` valide la forme complète,
    // plus strict que `URL.canParse` du zod — un DSN zod-valide peut être
    // rejeté ici : warn + aucun client, jamais de crash au boot).
    dsn.parse::<Dsn>().ok()?;
    // `tracesSampleRate` : sémantique `Number()` du legacy déjà portée par
    // config.rs (`sentry_traces_sample_rate_telemetry`) — absent → 0.1,
    // présent vide → 0, non numérique → NaN. Le SDK JS invalide une valeur
    // NaN (tracing désactivé, aucune transaction échantillonnée) ; la crate
    // Rust, elle, PANIQUE sur NaN hors [0.0, 1.0] (clientoptions.rs:402,
    // constaté par exécution) → NaN est mappé à 0.0, comportement OBSERVABLE
    // identique (aucune transaction). Inatteignable en serving : zod rejette
    // `SENTRY_TRACES_SAMPLE_RATE=abc` avant (crash zod byte-exact).
    let rate = if traces_sample_rate.is_nan() {
        0.0
    } else {
        traces_sample_rate as f32
    };
    let opts = sentry::ClientOptions::new()
        .dsn(dsn)
        .environment(environment.to_string())
        .traces_sample_rate(rate);
    Some(opts)
}

/// Équivalent `captureException` du legacy pour les erreurs MÉTIER du port
/// (les erreurs du port sont des messages typés, pas des `std::error::Error`
/// portant une pile V8) : `capture_message` niveau Error + tags. Sites :
/// fetch GitHub non-404/429 (github.rs, github.ts:153-160) ; no-op sans
/// client (DSN absent).
pub fn capture_exception_message(message: &str, tags: &[(&str, &str)]) {
    sentry::with_scope(
        |scope| {
            for (k, v) in tags {
                scope.set_tag(k, *v);
            }
        },
        || {
            sentry::capture_message(message, sentry::Level::Error);
        },
    );
}

/// Capture d'une réponse 5xx (middleware `app.rs`, équivalent des erreurs
/// non rattrapées du legacy) : méthode + chemin SANS query (aucun secret ni
/// login dans le message).
pub fn capture_error(what: &str) {
    sentry::capture_message(what, sentry::Level::Error);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsn_absent_desactive_dsn_malforme_softfail() {
        let off = SentryGate::init("", "production", 0.1);
        assert!(!off.gate.enabled);
        let bad = SentryGate::init("pas-un-dsn", "production", 0.1);
        assert!(!bad.gate.enabled); // soft-fail : pas de panic, boot poursuivi
        let on = SentryGate::init("https://leurre@sentry.example/1", "production", 0.1);
        assert!(on.gate.enabled);
    }

    #[test]
    fn client_options_none_quand_dsn_absent_ou_invalide() {
        assert!(client_options("", "production", 0.1).is_none());
        assert!(client_options("pas-un-dsn", "production", 0.1).is_none());
        assert!(client_options("https://incomplet", "production", 0.1).is_none());
        let opts =
            client_options("https://k@sentry.invalid/1", "staging", 1.0).expect("dsn valide");
        assert_eq!(opts.environment.as_deref(), Some("staging"));
        assert!(opts.dsn.is_some());
    }

    #[test]
    fn traces_sample_rate_semantique_number_legacy() {
        // `Number()` : absent → 0.1 (config.rs, sémantique telemetry) ;
        // vide → 0 ; bornes zod 0..1. Le taux n'est PAS introspectable
        // (`ClientOptions` non_exhaustif, aucun getter 0.49 — cf. Journal
        // weight fix5) : il est prouvé COMPORTEMENTALEMENT par (b)/(c) de
        // tests/p5_sentry.rs (1.0 → transaction émise ; 0.0 → aucune).
        let full = client_options("https://k@sentry.invalid/1", "production", 1.0).unwrap();
        assert_eq!(full.environment.as_deref(), Some("production"));
        let zero = client_options("https://k@sentry.invalid/1", "production", 0.0).unwrap();
        assert_eq!(zero.environment.as_deref(), Some("production"));
        assert!(
            client_options("https://k@sentry.invalid/1", "production", f64::NAN).is_some(),
            "NaN passe la construction (zod le rejetterait avant en serving)"
        );
    }
}
