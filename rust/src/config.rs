//! `loadConfig()` — port de `apps/api/src/config.ts` (@t3-oss/env-core +
//! zod v4), messages d'échec = ORACLE exécuté (`tests/golden/http/config/`,
//! génération `scripts/gen-http-golden.mjs` mode cfg).
//!
//! Épinglé par les captures :
//! - `emptyStringAsUndefined` : valeur `""` → undefined AVANT validation
//!   (défauts appliqués) — les cas `port-empty`, `loglevel-empty`,
//!   `cache_ttl-empty`, `redis-empty`, `cors-empty`, `sentry-rate-empty`,
//!   `sentry-env-empty` bootent ;
//! - échec → le legacy logue sur stderr le bloc
//!   `❌ Invalid environment variables: [ …issues au format inspect zod… ]`
//!   PUIS jette `Error: Invalid environment variables` (pile V8 + version Node,
//!   non reproduits — écart assumé documenté : le port imprime le bloc
//!   sémantique identique et termine avec le même code de sortie 1) ;
//! - `PAT_1` absent/vide → `Invalid input: expected string, received undefined`
//!   (le brief P3 citait « At least PAT_1 must be set » : c'est le message du
//!   POOL, jamais atteint au boot — le code capture prime, cf. rapport) ;
//! - `PORT`/`SENTRY_TRACES_SAMPLE_RATE` : `z.coerce.number()` = `Number(v)` JS
//!   (NaN → `invalid_type` avec `received: 'NaN'`) ;
//! - `REDIS_URL`/`SENTRY_DSN` : `z.string().url()` = `URL.canParse` WHATWG
//!   (zod v4 `canParseURL`, `zod/v4/core/schemas.js:214-226`) → crate `url` ;
//! - `collectPats()` : lecture DIRECTE de `process.env` (HORS zod), `PAT_1..N`
//!   séquentiels, arrêt au premier absent ou VIDE (ici : après validation, donc
//!   PAT_1 non vide garanti ; PAT_2 vide → arrêt, port de `pat_pool::collect_pats`).
//!
//! `SENTRY_TRACES_SAMPLE_RATE` / `SENTRY_ENVIRONMENT` sont validés par zod
//! (table §1.11) PUIS relus hors zod dans `telemetry.ts` avec `Number(… ?? '0.1')`
//! et `?? 'production'` — le port reproduit les DEUX lectures (les valeurs
//! zod-validées et les défauts telemetry) sans initier Sentry (phase 5).

use crate::query_params::js_number;

/// Config validée (équivalent `AppConfig` du legacy).
#[derive(Debug, Clone)]
pub struct Config {
    pub pats: Vec<String>,
    /// `Number(v)` JS — peut être non entier (zod n'exige pas un entier).
    pub port: f64,
    pub redis_url: Option<String>,
    pub cache_ttl: f64,
    pub cors_origin: Option<String>,
    pub log_level: &'static str,
    pub sentry_dsn: Option<String>,
    pub sentry_traces_sample_rate: f64,
    pub sentry_environment: String,
    /// Lecture hors zod de `telemetry.ts` (`Number(process.env.X ?? '0.1')`).
    pub sentry_traces_sample_rate_telemetry: f64,
    /// Lecture hors zod de `telemetry.ts` (`process.env.X ?? 'production'`).
    pub sentry_environment_telemetry: String,
    /// DIVERGENCE PROTECTRICE (revue P4 I-1, arbitrée par l'orchestrateur) :
    /// borne de poids du L1 en Mio (`L1_MAX_MIB`, défaut 24, `0` = non borné
    /// = parité stricte). Le legacy est NON BORNÉ (`KeyvCacheableMemory`,
    /// `_lruSize=0`, Map nue) sous un conteneur de prod 64 Mio → OOM réaliste
    /// en ~1-2 h de flood de usernames distincts (revue §9). Cette variable
    /// n'existe PAS chez le legacy : elle ne modifie le comportement QUE sous
    /// pression mémoire (évince en LRU par poids d'octets au lieu d'OOM).
    /// Lue HORS zod (aucune validation legacy à reproduire), invalide → défaut.
    pub l1_max_mib: u64,
}

/// `Number(port)` → port du LISTEN (`net.validatePort` Node, m-2 revue P4).
/// zod accepte les non-entiers (coerce) mais Node jette
/// `RangeError [ERR_SOCKET_BAD_PORT]` AU LISTEN — oracle exécuté :
/// `tests/golden/config-minors/port-3000-5.stderr.txt` (exit 1). Le port
/// reproduit l'issue : None ⇔ port non entier hors [0 ; 65536[ → stderr
/// sémantique identique (sans la pile V8) + exit 1.
pub fn port_as_u16(port: f64) -> Option<u16> {
    if port.is_finite() && (0.0..65536.0).contains(&port) && port.fract() == 0.0 {
        Some(port as u16)
    } else {
        None
    }
}

/// Un issue zod (format inspect de la capture).
#[derive(Debug, Clone, PartialEq)]
pub struct ZodIssue {
    /// Champs dans l'ordre observé : expected?, received?, origin?, code,
    /// minimum/maximum?, inclusive?, values?, format?, path, message.
    pub expected: Option<&'static str>,
    pub received: Option<&'static str>,
    pub origin: Option<&'static str>,
    pub code: &'static str,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub inclusive: Option<bool>,
    pub values: Option<&'static [&'static str]>,
    pub format: Option<&'static str>,
    pub path: &'static str,
    pub message: String,
}

impl ZodIssue {
    /// Rendu `util.inspect` zod v4 — lignes ÉPILES par les captures :
    /// ```text
    /// {
    ///     expected: 'string',
    ///     code: 'invalid_type',
    ///     path: [ 'PAT_1' ],
    ///     message: 'Invalid input: expected string, received undefined'
    /// }
    /// ```
    /// `received` n'apparaît QUE pour NaN (captures : absent sur undefined).
    fn render(&self) -> String {
        let mut lines = vec!["  {".to_string()];
        if let Some(e) = self.expected {
            lines.push(format!("    expected: '{e}',"));
        }
        if let Some(o) = self.origin {
            // captures port-0 / cache_ttl-low : origin AVANT code
            lines.push(format!("    origin: '{o}',"));
        }
        lines.push(format!("    code: '{}',", self.code));
        if self.received == Some("NaN") {
            // capture port-abc / sentry-rate-nan : received APRÈS code
            lines.push("    received: 'NaN',".to_string());
        }
        if let Some(m) = self.minimum {
            lines.push(format!("    minimum: {},", js_inspect_number(m)));
        }
        if let Some(m) = self.maximum {
            lines.push(format!("    maximum: {},", js_inspect_number(m)));
        }
        if let Some(i) = self.inclusive {
            lines.push(format!("    inclusive: {i},"));
        }
        if let Some(values) = self.values {
            let joined = values
                .iter()
                .map(|v| format!("'{v}'"))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("    values: [ {joined} ],"));
        }
        if let Some(f) = self.format {
            lines.push(format!("    format: '{f}',"));
        }
        lines.push(format!("    path: [ '{}' ],", self.path));
        lines.push(format!("    message: '{}'", escape_single(self.message())));
        lines.push("  }".to_string());
        lines.join("\n")
    }

    fn message(&self) -> String {
        match self.code {
            "invalid_type" => match self.received {
                Some("NaN") => "Invalid input: expected number, received NaN".to_string(),
                _ => format!(
                    "Invalid input: expected {}, received {}",
                    self.expected.unwrap_or("unknown"),
                    self.received.unwrap_or("undefined")
                ),
            },
            "too_small" => format!(
                "Too small: expected {} to be >={}",
                self.origin.unwrap_or("number"),
                js_inspect_number(self.minimum.unwrap_or(0.0))
            ),
            "too_big" => format!(
                "Too big: expected {} to be <={}",
                self.origin.unwrap_or("number"),
                js_inspect_number(self.maximum.unwrap_or(0.0))
            ),
            "invalid_value" => {
                let joined = self
                    .values
                    .unwrap_or(&[])
                    .iter()
                    .map(|v| format!("\"{v}\""))
                    .collect::<Vec<_>>()
                    .join("|");
                format!("Invalid option: expected one of {joined}")
            }
            "invalid_format" => "Invalid URL".to_string(),
            other => other.to_string(),
        }
    }
}

/// Bloc stderr du legacy : `❌ Invalid environment variables: [ … ]`.
pub fn render_invalid_env_block(issues: &[ZodIssue]) -> String {
    let rendered: Vec<String> = issues.iter().map(|i| i.render()).collect();
    let sep = if rendered.len() > 1 { ",\n" } else { "\n" };
    format!(
        "❌ Invalid environment variables: [\n{}\n]",
        rendered.join(sep)
    )
}

/// `util.inspect` des nombres : pas de `.0` (1 → `1`, 60 → `60`, 65535 → `65535`).
fn js_inspect_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e21 {
        format!("{}", n as i64)
    } else {
        crate::jsnum::js_f64_to_string(n)
    }
}

/// Échappement minimal pour les guillemets simples dans `inspect` (message
/// zod ne contient pas de `'` dans les captures ; garde défensive).
fn escape_single(s: String) -> String {
    s.replace('\'', "\\'")
}

/// Erreur de validation : les issues zod (ordre du schéma = ordre de déclaration
/// dans config.ts) + le bloc stderr à imprimer avant `exit(1)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigError {
    pub issues: Vec<ZodIssue>,
}

impl ConfigError {
    /// Bloc sémantique `❌ …` (la pile V8 du legacy n'est pas reproduite).
    pub fn stderr_block(&self) -> String {
        render_invalid_env_block(&self.issues)
    }
}

use std::fmt;

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid environment variables")
    }
}

impl std::error::Error for ConfigError {}

/// Source d'env injectable (les tests passent une map ; le binaire passe
/// `std::env`). `None` = variable absente.
pub trait EnvSource {
    fn get(&self, key: &str) -> Option<String>;
}

/// Adaptateur `std::env` pour le binaire.
pub struct OsEnv;

impl EnvSource for OsEnv {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

/// Vecteur de clés (les tests).
impl EnvSource for Vec<(&str, Option<String>)> {
    fn get(&self, key: &str) -> Option<String> {
        self.iter()
            .find(|(k, _)| *k == key)
            .and_then(|(_, v)| v.clone())
    }
}

/// `emptyStringAsUndefined: true` (t3-env) : `""` → undefined avant validation.
fn env_undefined_if_empty(env: &dyn EnvSource, key: &str) -> Option<String> {
    match env.get(key) {
        None => None,
        Some(v) if v.is_empty() => None,
        Some(v) => Some(v),
    }
}

fn issue_invalid_type_undefined(path: &'static str, expected: &'static str) -> ZodIssue {
    ZodIssue {
        expected: Some(expected),
        // capture no-pat1/pat1-empty : PAS de champ `received` pour undefined
        // (il n'apparaît que pour NaN — port-abc / sentry-rate-nan)
        received: None,
        origin: None,
        code: "invalid_type",
        minimum: None,
        maximum: None,
        inclusive: None,
        values: None,
        format: None,
        path,
        message: String::new(),
    }
}

/// `loadConfig(env)` — valide comme zod et construit [`Config`].
pub fn load_config(env: &dyn EnvSource) -> Result<Config, ConfigError> {
    let mut issues: Vec<ZodIssue> = Vec::new();

    // PORT: z.coerce.number().min(1).max(65535).default(3000)
    let port = match env_undefined_if_empty(env, "PORT") {
        None => 3000.0,
        Some(raw) => {
            let n = js_number(&raw);
            if n.is_nan() {
                issues.push(ZodIssue {
                    expected: Some("number"),
                    received: Some("NaN"),
                    origin: None,
                    code: "invalid_type",
                    minimum: None,
                    maximum: None,
                    inclusive: None,
                    values: None,
                    format: None,
                    path: "PORT",
                    message: String::new(),
                });
                3000.0
            } else if n < 1.0 {
                issues.push(bound_issue("PORT", "too_small", Some(1.0), None));
                3000.0
            } else if n > 65535.0 {
                issues.push(bound_issue("PORT", "too_big", None, Some(65535.0)));
                3000.0
            } else {
                n
            }
        }
    };

    // LOG_LEVEL: z.enum(['debug','info','warn','error']).default('info')
    let log_level = match env_undefined_if_empty(env, "LOG_LEVEL") {
        None => "info",
        Some(raw) => match raw.as_str() {
            "debug" => "debug",
            "info" => "info",
            "warn" => "warn",
            "error" => "error",
            _ => {
                issues.push(ZodIssue {
                    expected: None,
                    received: None,
                    origin: None,
                    code: "invalid_value",
                    minimum: None,
                    maximum: None,
                    inclusive: None,
                    values: Some(&["debug", "info", "warn", "error"]),
                    format: None,
                    path: "LOG_LEVEL",
                    message: String::new(),
                });
                "info"
            }
        },
    };

    // REDIS_URL: z.string().url().optional()
    let redis_url = url_var(env, "REDIS_URL", &mut issues);
    // CACHE_TTL: z.coerce.number().min(60).default(14400)
    let cache_ttl = match env_undefined_if_empty(env, "CACHE_TTL") {
        None => 14400.0,
        Some(raw) => {
            let n = js_number(&raw);
            if n.is_nan() {
                issues.push(ZodIssue {
                    expected: Some("number"),
                    received: Some("NaN"),
                    origin: None,
                    code: "invalid_type",
                    minimum: None,
                    maximum: None,
                    inclusive: None,
                    values: None,
                    format: None,
                    path: "CACHE_TTL",
                    message: String::new(),
                });
                14400.0
            } else if n < 60.0 {
                issues.push(bound_issue("CACHE_TTL", "too_small", Some(60.0), None));
                14400.0
            } else {
                n
            }
        }
    };
    // CORS_ORIGIN: z.string().optional()
    let cors_origin = env_undefined_if_empty(env, "CORS_ORIGIN");
    // SENTRY_DSN: z.string().url().optional()
    let sentry_dsn = url_var(env, "SENTRY_DSN", &mut issues);
    // SENTRY_TRACES_SAMPLE_RATE: z.coerce.number().min(0).max(1).default(0.1)
    let sentry_traces_sample_rate = match env_undefined_if_empty(env, "SENTRY_TRACES_SAMPLE_RATE") {
        None => 0.1,
        Some(raw) => {
            let n = js_number(&raw);
            if n.is_nan() {
                issues.push(ZodIssue {
                    expected: Some("number"),
                    received: Some("NaN"),
                    origin: None,
                    code: "invalid_type",
                    minimum: None,
                    maximum: None,
                    inclusive: None,
                    values: None,
                    format: None,
                    path: "SENTRY_TRACES_SAMPLE_RATE",
                    message: String::new(),
                });
                0.1
            } else if n < 0.0 {
                issues.push(bound_issue(
                    "SENTRY_TRACES_SAMPLE_RATE",
                    "too_small",
                    Some(0.0),
                    None,
                ));
                0.1
            } else if n > 1.0 {
                issues.push(bound_issue(
                    "SENTRY_TRACES_SAMPLE_RATE",
                    "too_big",
                    None,
                    Some(1.0),
                ));
                0.1
            } else {
                n
            }
        }
    };
    // SENTRY_ENVIRONMENT: z.string().min(1).default('production')
    let sentry_environment = match env_undefined_if_empty(env, "SENTRY_ENVIRONMENT") {
        None => "production".to_string(),
        Some(_) => env.get("SENTRY_ENVIRONMENT").unwrap_or_default(),
    };
    // PAT_1: z.string().min(1, 'At least PAT_1 must be set') — requis.
    // `""` → undefined (emptyStringAsUndefined) → invalid_type undefined :
    // le message min(1) du legacy est mort (capture `pat1-empty`).
    let pat_1 = match env_undefined_if_empty(env, "PAT_1") {
        None => {
            issues.push(issue_invalid_type_undefined("PAT_1", "string"));
            String::new()
        }
        Some(v) => v,
    };

    if !issues.is_empty() {
        return Err(ConfigError { issues });
    }

    // collectPats() : process.env direct (hors zod) — PAT_1..N séquentiels,
    // arrêt au premier absent ou vide. PAT_1 garanti non vide ici.
    let mut pats = vec![pat_1];
    let mut i = 2;
    loop {
        match env.get(&format!("PAT_{i}")) {
            Some(v) if !v.is_empty() => {
                pats.push(v);
                i += 1;
            }
            _ => break,
        }
    }

    // Lecture hors zod de telemetry.ts : `Number(process.env.X ?? '0.1')` et
    // `?? 'production'` (Number('') = 0, Number('abc') = NaN — sémantique JS).
    let sentry_traces_sample_rate_telemetry = match env.get("SENTRY_TRACES_SAMPLE_RATE") {
        None => 0.1,
        Some(raw) => js_number(&raw), // NaN possible : le legacy accepte (init Sentry)
    };
    let sentry_environment_telemetry = env
        .get("SENTRY_ENVIRONMENT")
        .unwrap_or_else(|| "production".to_string());

    // Borne protectrice L1 (I-1) — hors zod, voir le doc de `Config`.
    let l1_max_mib = env
        .get("L1_MAX_MIB")
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(24);

    Ok(Config {
        pats,
        port,
        redis_url,
        cache_ttl,
        cors_origin,
        log_level,
        sentry_dsn,
        sentry_traces_sample_rate,
        sentry_environment,
        sentry_traces_sample_rate_telemetry,
        sentry_environment_telemetry,
        l1_max_mib,
    })
}

fn bound_issue(
    path: &'static str,
    code: &'static str,
    minimum: Option<f64>,
    maximum: Option<f64>,
) -> ZodIssue {
    ZodIssue {
        expected: None,
        received: None,
        origin: Some("number"),
        code,
        minimum,
        maximum,
        inclusive: Some(true),
        values: None,
        format: None,
        path,
        message: String::new(),
    }
}

/// `z.string().url()` (zod v4) = `URL.canParse` WHATWG — crate `url`.
/// Champ optionnel : absent → None ; présent mais non URL → issue
/// `invalid_format` / `format: 'url'` / `Invalid URL` (captures redis-bad,
/// sentry-dsn-bad).
fn url_var(env: &dyn EnvSource, key: &'static str, issues: &mut Vec<ZodIssue>) -> Option<String> {
    match env_undefined_if_empty(env, key) {
        None => None,
        Some(raw) => {
            if url::Url::parse(&raw).is_ok() {
                Some(raw)
            } else {
                issues.push(ZodIssue {
                    expected: None,
                    received: None,
                    origin: None,
                    code: "invalid_format",
                    minimum: None,
                    maximum: None,
                    inclusive: None,
                    values: None,
                    format: Some("url"),
                    path: key,
                    message: String::new(),
                });
                None
            }
        }
    }
}
