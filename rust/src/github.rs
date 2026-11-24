//! Client GitHub GraphQL (port de `apps/api/src/fetchers/github.ts`, phase 3).
//!
//! Vérité = legacy exécuté (oracles `scripts/gen-graphql.mjs`) :
//! - URL `https://api.github.com/graphql`, POST ;
//! - en-têtes : `Authorization: bearer <token>` (**b minuscule**, prouvé par
//!   extraction de mots du legacy — jamais la ligne imprimée), `Content-Type:
//!   application/json`, `User-Agent: github-stats-cards` ;
//! - corps : `{"query":<requête verbatim>,"variables":{"login":<username>}}`
//!   (ordre de clés du legacy `JSON.stringify`, requête extraite par script →
//!   `tests/golden/graphql/user-query.txt`, incluse ici par `include_str!`) ;
//! - mapping d'erreurs identique : 403/429 → [`GitHubError::RateLimit`]
//!   (`x-ratelimit-remaining` ?? 0, `x-ratelimit-reset` ×1000 ms), tout autre
//!   statut → [`GitHubError::Api`] « GitHub API error: <status> <statusText> »,
//!   `errors` → jointure « , » (utilisateur inconnu → NotFound), `data.user`
//!   nul/absent → NotFound ;
//! - `fetchWithRetry` : UN seul nouvel essai avec le jeton suivant si limite ;
//!   le second échec se propage ; le jeton échu est marqué.
//!
//! Agrégations : ordre d'insertion des clés = littéraux du legacy ; couleur
//! par défaut `#858585` ; pourcentage `(size/totalSize)*100` ; tri stable
//! descendant par taille puis top 5 ; séries avant/arrière du calendrier ;
//! `email: null` fixe ; `name ?? login`.

use crate::js_json::{JsJson, JsJson::*};
use crate::jsval::number_as_f64;
use serde_json::Value;

pub const GITHUB_GRAPHQL_URL: &str = "https://api.github.com/graphql";
/// Requête GraphQL VERBATIM du legacy (extraite du corps capturé par
/// `scripts/gen-graphql.mjs` — jamais retapée à la main).
pub const USER_QUERY: &str = include_str!("../../tests/golden/graphql/user-query.txt");

/// `FetchResult` : arbre ordonné prêt pour le JSON de cache (phase 4).
pub type FetchResult = JsJson;

#[derive(Debug, Clone, PartialEq)]
pub enum GitHubError {
    /// `GitHubNotFoundError` — « User not found: <username> », statut 404.
    NotFound { username: String },
    /// `GitHubRateLimitError` — « GitHub API rate limit exceeded », statut 429 ;
    /// `remaining` = `Number(header ?? 0)` (0 si absent, NaN si non numérique) ;
    /// `reset_at_ms` = `Number(x-ratelimit-reset) * 1000` si l'en-tête existe.
    RateLimit {
        remaining: f64,
        reset_at_ms: Option<f64>,
    },
    /// `GitHubApiError` — message du legacy + statut optionnel. Sert aussi aux
    /// erreurs de transport (message « fetch failed » = `TypeError` undici) et
    /// de corps non JSON (divergence notée : le message SyntaxError exact de
    /// V8 n'est pas reproduit, le TYPE d'échec l'est).
    Api {
        message: String,
        status: Option<u16>,
    },
}

impl GitHubError {
    /// Propriétés des classes legacy (name/message/status) pour les tests.
    pub fn class_name(&self) -> &'static str {
        match self {
            GitHubError::NotFound { .. } => "GitHubNotFoundError",
            GitHubError::RateLimit { .. } => "GitHubRateLimitError",
            GitHubError::Api { .. } => "GitHubApiError",
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            GitHubError::NotFound { .. } => Some(404),
            GitHubError::RateLimit { .. } => Some(429),
            GitHubError::Api { status, .. } => *status,
        }
    }
}

impl fmt::Display for GitHubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitHubError::NotFound { username } => write!(f, "User not found: {username}"),
            GitHubError::RateLimit { .. } => {
                write!(f, "GitHub API rate limit exceeded")
            }
            GitHubError::Api { message, .. } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for GitHubError {}

use std::fmt;

/// `fetchGitHubData(username, token)` : un POST, mapping d'erreurs, agrégations.
pub async fn fetch_github_data(
    http: &reqwest::Client,
    username: &str,
    token: &str,
) -> Result<FetchResult, GitHubError> {
    fetch_github_data_at(http, GITHUB_GRAPHQL_URL, username, token).await
}

/// Variante à URL injectable : les tests `wiremock` lient un serveur sur
/// 127.0.0.1 (`--network none`) sans toucher à la constante du legacy.
pub async fn fetch_github_data_at(
    http: &reqwest::Client,
    url: &str,
    username: &str,
    token: &str,
) -> Result<FetchResult, GitHubError> {
    let body = format!(
        "{{\"query\":{},\"variables\":{{\"login\":{}}}}}",
        json_escape(USER_QUERY),
        json_escape(username),
    );
    let response = http
        .post(url)
        .header("Authorization", format!("bearer {token}"))
        .header("Content-Type", "application/json")
        .header("User-Agent", "github-stats-cards")
        .body(body)
        .send()
        .await
        .map_err(|_| GitHubError::Api {
            message: "fetch failed".to_string(),
            status: None,
        })?;

    let status = response.status();
    if !status.is_success() {
        let code = status.as_u16();
        if code == 403 || code == 429 {
            let remaining = match response.headers().get("x-ratelimit-remaining") {
                Some(v) => v
                    .to_str()
                    .ok()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(f64::NAN), // Number(header) non numérique → NaN
                None => 0.0, // Number(header ?? 0)
            };
            let reset_at_ms = response.headers().get("x-ratelimit-reset").map(|v| {
                v.to_str()
                    .ok()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|epoch| epoch * 1000.0)
                    .unwrap_or(f64::NAN)
            });
            return Err(GitHubError::RateLimit {
                remaining,
                reset_at_ms,
            });
        }
        return Err(GitHubError::Api {
            message: format!(
                "GitHub API error: {} {}",
                code,
                status.canonical_reason().unwrap_or_default()
            ),
            status: Some(code),
        });
    }

    let text = response.text().await.map_err(|_| GitHubError::Api {
        message: "fetch failed".to_string(),
        status: None,
    })?;
    let json: Value = serde_json::from_str(&text).map_err(|_| GitHubError::Api {
        message: "invalid JSON body".to_string(), // divergence : message SyntaxError V8 non reproduit
        status: None,
    })?;

    if json
        .get("errors")
        .and_then(Value::as_array)
        .is_some_and(|e| !e.is_empty())
    {
        let messages: Vec<String> = json["errors"]
            .as_array()
            .expect("errors tableau")
            .iter()
            .map(|e| {
                e.get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            })
            .collect();
        let joined = messages.join(", ");
        if joined.contains("Could not resolve to a User") {
            return Err(GitHubError::NotFound {
                username: username.to_string(),
            });
        }
        return Err(GitHubError::Api {
            message: format!("GraphQL errors: {joined}"),
            status: None,
        });
    }

    let user = json.get("data").and_then(|d| d.get("user"));
    let user = match user {
        None | Some(Value::Null) => {
            return Err(GitHubError::NotFound {
                username: username.to_string(),
            })
        }
        Some(u) => u,
    };

    Ok(extract_all(user, username))
}

/// `fetchWithRetry(username, patPool)` : un jeton, UN seul nouvel essai avec le
/// jeton suivant si limite atteinte ; le second échec se propage ; le jeton
/// échu est marqué.
pub async fn fetch_with_retry(
    http: &reqwest::Client,
    pool: &mut crate::pat_pool::PatPool,
    username: &str,
) -> Result<FetchResult, GitHubError> {
    fetch_with_retry_at(http, pool, GITHUB_GRAPHQL_URL, username).await
}

/// Variante à URL injectable : les tests `wiremock` lient un serveur sur
/// 127.0.0.1 (`--network none`, aucun appel GitHub réel) pendant que l'URL du
/// legacy reste dans [`GITHUB_GRAPHQL_URL`].
pub async fn fetch_with_retry_at(
    http: &reqwest::Client,
    pool: &mut crate::pat_pool::PatPool,
    url: &str,
    username: &str,
) -> Result<FetchResult, GitHubError> {
    let token = pool.get_next_token().map_err(|e| GitHubError::Api {
        message: e.to_string(),
        status: None,
    })?;
    match fetch_github_data_at(http, url, username, &token).await {
        Ok(result) => Ok(result),
        Err(GitHubError::RateLimit { .. }) => {
            pool.mark_exhausted(&token);
            let token2 = pool.get_next_token().map_err(|e| GitHubError::Api {
                message: e.to_string(),
                status: None,
            })?;
            fetch_github_data_at(http, url, username, &token2).await
        }
        Err(err) => Err(err),
    }
}

// ---------------------------------------------------------------- agrégations

fn num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => number_as_f64(n),
        _ => f64::NAN, // `undefined`/`null` arithmétiques → NaN (compare > 0 → false)
    }
}

fn str_field(v: &Value, k: &str) -> JsJson {
    match v.get(k) {
        Some(Value::String(s)) => Str(s.clone()),
        Some(Value::Null) | None => Undefined, // `?? login` géré par l'appelant si besoin
        Some(_) => Undefined,
    }
}

fn str_or(v: &Value, k: &str, fallback: &str) -> String {
    match v.get(k) {
        Some(Value::String(s)) => s.clone(), // `?? ` : null/absent seulement
        _ => fallback.to_string(),
    }
}

/// `extractStats` — ordre de clés : username,name,totalStars,totalForks,
/// totalCommits,totalPRs,totalIssues,contributedTo.
fn extract_stats(user: &Value, username: &str) -> JsJson {
    let nodes = user["repositories"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut total_stars = 0.0;
    let mut total_forks = 0.0;
    for r in &nodes {
        total_stars += num(&r["stargazerCount"]);
        total_forks += num(&r["forkCount"]);
    }
    let total_commits = num(&user["contributionsCollection"]["totalCommitContributions"])
        + num(&user["contributionsCollection"]["restrictedContributionsCount"]);
    Obj(vec![
        ("username".into(), Str(username.to_string())),
        (
            "name".into(),
            Str(str_or(user, "name", &str_or(user, "login", username))),
        ),
        ("totalStars".into(), Num(total_stars)),
        ("totalForks".into(), Num(total_forks)),
        ("totalCommits".into(), Num(total_commits)),
        (
            "totalPRs".into(),
            Num(num(&user["pullRequests"]["totalCount"])),
        ),
        (
            "totalIssues".into(),
            Num(num(&user["issues"]["totalCount"])),
        ),
        (
            "contributedTo".into(),
            Num(num(&user["repositoriesContributedTo"]["totalCount"])),
        ),
    ])
}

/// `extractStreak` — scan avant (longest) puis arrière (current, break au 1er
/// jour à 0 APRÈS avoir compté) ; dates `''` si aucun jour.
fn extract_streak(user: &Value, username: &str) -> JsJson {
    let weeks = user["contributionsCollection"]["contributionCalendar"]["weeks"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut days: Vec<&Value> = Vec::new();
    for w in &weeks {
        if let Some(ds) = w["contributionDays"].as_array() {
            for d in ds {
                days.push(d);
            }
        }
    }
    let mut current_streak = 0.0;
    let mut longest_streak = 0.0;
    let mut temp_streak = 0.0;
    let mut current_streak_start = String::new();
    let mut current_streak_end = String::new();
    let mut longest_streak_start = String::new();
    let mut longest_streak_end = String::new();
    let mut temp_start = String::new();
    let day_date = |d: &Value| -> String {
        d.get("date")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    for d in &days {
        if num(&d["contributionCount"]) > 0.0 {
            if temp_streak == 0.0 {
                temp_start = day_date(d);
            }
            temp_streak += 1.0;
            if temp_streak > longest_streak {
                longest_streak = temp_streak;
                longest_streak_start = temp_start.clone();
                longest_streak_end = day_date(d);
            }
        } else {
            temp_streak = 0.0;
        }
    }
    // (le `currentStreak = 0` du legacy est un no-op ici : jamais écrit entre
    // l'initialisation et ce reset — clippy unused_assignments)
    for d in days.iter().rev() {
        if num(&d["contributionCount"]) > 0.0 {
            current_streak += 1.0;
            if current_streak_end.is_empty() {
                current_streak_end = day_date(d);
            }
            current_streak_start = day_date(d);
        } else if current_streak > 0.0 {
            break;
        }
    }
    Obj(vec![
        ("username".into(), Str(username.to_string())),
        (
            "totalContributions".into(),
            Num(num(
                &user["contributionsCollection"]["contributionCalendar"]["totalContributions"]
            )),
        ),
        ("currentStreak".into(), Num(current_streak)),
        ("longestStreak".into(), Num(longest_streak)),
        ("currentStreakStart".into(), Str(current_streak_start)),
        ("currentStreakEnd".into(), Str(current_streak_end)),
        ("longestStreakStart".into(), Str(longest_streak_start)),
        ("longestStreakEnd".into(), Str(longest_streak_end)),
    ])
}

/// `extractLanguages` — map d'insertion (ordre réponse GraphQL), `color ?? '#858585'`,
/// `percentage = (size/totalSize)*100` (0 si totalSize 0), tri STABLE descendant
/// par size, top 5.
fn extract_languages(user: &Value, username: &str) -> JsJson {
    let nodes = user["repositories"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    // langMap : insertion-order Map(name -> {size, color})
    let mut lang_map: Vec<(String, f64, String)> = Vec::new();
    for repo in &nodes {
        let edges = repo["languages"]["edges"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for edge in &edges {
            let name = edge["node"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let size = num(&edge["size"]);
            let color = match edge["node"]["color"] {
                Value::String(ref c) => c.clone(),
                _ => "#858585".to_string(), // `color ?? '#858585'` (null/absent)
            };
            if let Some(existing) = lang_map.iter_mut().find(|(n, _, _)| *n == name) {
                existing.1 += size;
            } else {
                lang_map.push((name, size, color));
            }
        }
    }
    let total_size: f64 = lang_map.iter().map(|(_, s, _)| *s).sum();
    let mut languages: Vec<(String, f64, String, f64)> = lang_map
        .into_iter()
        .map(|(name, size, color)| {
            let pct = if total_size > 0.0 {
                (size / total_size) * 100.0
            } else {
                0.0
            };
            (name, size, color, pct)
        })
        .collect();
    // `.toSorted((a, b) => b.size - a.size)` : tri STABLE, comparateur descendant.
    languages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    languages.truncate(5);
    Obj(vec![
        ("username".into(), Str(username.to_string())),
        (
            "languages".into(),
            Arr(languages
                .into_iter()
                .map(|(name, size, color, percentage)| {
                    Obj(vec![
                        ("name".into(), Str(name)),
                        ("size".into(), Num(size)),
                        ("color".into(), Str(color)),
                        ("percentage".into(), Num(percentage)),
                    ])
                })
                .collect()),
        ),
    ])
}

/// `extractProfile` — calendrier aplati {date, count}, `createdAt` brut,
/// `email: null` fixe, `name ?? login`.
fn extract_profile(user: &Value, username: &str) -> JsJson {
    let weeks = user["contributionsCollection"]["contributionCalendar"]["weeks"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut calendar: Vec<JsJson> = Vec::new();
    for w in &weeks {
        if let Some(days) = w["contributionDays"].as_array() {
            for d in days {
                calendar.push(Obj(vec![
                    ("date".into(), str_field(d, "date")),
                    ("count".into(), Num(num(&d["contributionCount"]))),
                ]));
            }
        }
    }
    Obj(vec![
        ("username".into(), Str(username.to_string())),
        (
            "name".into(),
            Str(str_or(user, "name", &str_or(user, "login", username))),
        ),
        (
            "contributionsThisYear".into(),
            Num(num(
                &user["contributionsCollection"]["contributionCalendar"]["totalContributions"]
            )),
        ),
        (
            "publicRepos".into(),
            Num(num(&user["repositories"]["totalCount"])),
        ),
        ("createdAt".into(), str_field(user, "createdAt")),
        ("email".into(), Null),
        ("contributionCalendar".into(), Arr(calendar)),
    ])
}

/// Les 4 agrégations, dans l'ordre du littéral `FetchResult` du legacy :
/// stats, streak, languages, profile.
pub fn extract_all(user: &Value, username: &str) -> FetchResult {
    Obj(vec![
        ("stats".into(), extract_stats(user, username)),
        ("streak".into(), extract_streak(user, username)),
        ("languages".into(), extract_languages(user, username)),
        ("profile".into(), extract_profile(user, username)),
    ])
}

/// `json_escape` : le même échappement que [`crate::js_json`].
fn json_escape(s: &str) -> String {
    JsJson::Str(s.to_string()).to_json_string()
}
