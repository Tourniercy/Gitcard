//! Routes HTTP phase 4 — port de `apps/api/src/index.ts` (+ routes/*),
//! ORACLE = legacy Node exécuté hors réseau (`tests/golden/http/base` +
//! `cors-origin`, socket brute). Tous les ordres d'en-têtes sont ceux des
//! captures (ACAO du middleware cors AVANT les en-têtes du handler, `vary`
//! APRÈS, `content-length` puis `Date` en fin — `Connection` géré par hyper).
//!
//! Correspondance legacy :
//! - `hono/cors` (index.ts:27-34) → [`cors_middleware`] : `origin` absent →
//!   `*` (ACAO sur TOUTES les réponses, même sans en-tête Origin ; jamais de
//!   `vary`) ; `CORS_ORIGIN` défini → ACAO seulement si Origin correspond,
//!   `vary: Origin` partout ; OPTIONS (toute route, même inconnue) → 204
//!   préemptif AVANT routage (ordre épinglé : [allow-headers], allow-methods,
//!   allow-origin, max-age, vary combiné « Origin, Access-Control-Request-Headers ») ;
//! - HEAD (hono-base.js:257) → dispatché sur le handler GET, corps vidé,
//!   PAS de content-length (captures head_health/head_nope) ;
//! - `notFound` (index.ts:49-51) → 404 JSON `{"error":"Not found"}` ;
//! - `onError` (index.ts:54-58) → 500 JSON `{"error":"Internal server error"}`
//!   (zod query incluse) ;
//! - health (health.ts) / themes + data (data.ts) / cartes (card-factory.ts) ;
//! - `svgResponse` (card-response.ts:4-15) : Content-Type
//!   `image/svg+xml; charset=utf-8`, Cache-Control
//!   `public, max-age=N, s-maxage=N, stale-while-revalidate=86400`,
//!   ETag = md5 hexadécimal du corps ENTRE GUILLEMETS (fort) ;
//!   ordre handler OBSERVÉ : cache-control, content-type, etag.
//!
//! ÉCART PHASE 5 : le rate-limiter `hono-rate-limiter` de `/api/data/*`
//! (data.ts:18-34) et ses en-têtes `ratelimit-*` ne sont PAS portés ici
//! (middlewares = phase 5, cf. plan des phases) ; les tests HTTP normalisent
//! ces 4 en-têtes le temps de la phase 4.
//!
//! Écart transport assumé : hyper écrit `Connection` (position transport) —
//! les tests comparent les en-têtes application et le corps à l'octet.

use crate::cache::AppCache;
use crate::github::{fetch_with_retry_shared, GitHubError, GITHUB_GRAPHQL_URL};
use crate::pat_pool::PatPool;
use crate::query_params::{parse_card_options, QueryError};
use crate::themes::THEME_NAMES;

use axum::body::Body;
use axum::extract::{Path, RawQuery, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use std::sync::{Arc, Mutex};

pub const SVG_CONTENT_TYPE: &str = "image/svg+xml; charset=utf-8";
pub const JSON_CONTENT_TYPE: &str = "application/json";
const SVG_CACHE_SECONDS_404: f64 = 300.0;
const SVG_CACHE_SECONDS_ERROR: f64 = 60.0;
const AC_MAX_AGE: &str = "86400";

/// État applicatif (cloneable : cache et pool derrière Arc).
pub struct AppState<L2: crate::cache::L2Cache> {
    pub cache: Arc<AppCache<L2>>,
    pub pool: Arc<Mutex<PatPool>>,
    pub http: reqwest::Client,
    /// URL GraphQL injectable (prod = [`GITHUB_GRAPHQL_URL`] ; tests : wiremock
    /// ou port fermé pour reproduire « fetch failed » sans réseau).
    pub github_url: String,
    /// `CACHE_TTL` (TTL de la clé `data:`).
    pub cache_ttl: f64,
    /// `CORS_ORIGIN` (absent → cors `*`).
    pub cors_origin: Option<String>,
}

impl<L2: crate::cache::L2Cache> Clone for AppState<L2> {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            pool: self.pool.clone(),
            http: self.http.clone(),
            github_url: self.github_url.clone(),
            cache_ttl: self.cache_ttl,
            cors_origin: self.cors_origin.clone(),
        }
    }
}

/// Erreur applicative → 500 JSON (équivalent `app.onError`, index.ts:54-58 ;
/// le message est logué sur stderr comme `console.error('Unhandled error:', err)`).
#[derive(Debug)]
pub struct AppError(pub String);

impl From<QueryError> for AppError {
    fn from(e: QueryError) -> Self {
        AppError(format!("Unhandled error: {e}"))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("{}", self.0);
        let mut res = json_response("{\"error\":\"Internal server error\"}".to_string());
        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        res
    }
}

// ------------------------------------------------------------ corps réponses

fn json_response(body: String) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(JSON_CONTENT_TYPE),
    );
    // ordre du handler OBSERVÉ (captures JSON) : content-type seul ;
    // content-length est posé par le middleware cors, à la position du legacy.
    (StatusCode::OK, headers, Body::from(body)).into_response()
}

fn json_response_status(body: String, status: StatusCode) -> Response {
    let mut res = json_response(body);
    *res.status_mut() = status;
    res
}

fn svg_response(svg: String, cache_seconds: f64) -> Response {
    let etag = crate::md5_hex(&svg);
    let mut headers = HeaderMap::new();
    // ordre du handler OBSERVÉ (captures SVG) : cache-control, content-type, etag
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_str(&cache_control_value(cache_seconds)).expect("cache-control ASCII"),
    );
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(SVG_CONTENT_TYPE),
    );
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(&format!("\"{etag}\"")).expect("etag ASCII"),
    );
    (StatusCode::OK, headers, Body::from(svg)).into_response()
}

/// `public, max-age=${cacheSeconds}, s-maxage=${cacheSeconds},
/// stale-while-revalidate=86400` — `cacheSeconds` est toujours FINI (l'oracle
/// params-hash.json prouve que zod v4 jette ∞ dans le catch 14400).
fn cache_control_value(cache_seconds: f64) -> String {
    let secs = crate::jsnum::js_f64_to_string(cache_seconds);
    format!("public, max-age={secs}, s-maxage={secs}, stale-while-revalidate=86400")
}

fn error_svg_response(message: &str, status: StatusCode, cache_seconds: f64) -> Response {
    let mut res = svg_response(crate::cards::error_svg(message), cache_seconds);
    *res.status_mut() = status;
    res
}

// -------------------------------------------------- middleware CORS + ordre

/// Origine autorisée (hono `findAllowOrigin`) : config absente → toujours `*`
/// (même sans en-tête Origin) ; config → uniquement si Origin correspond.
fn allow_origin_of(cors_origin: &Option<String>, req_origin: Option<&str>) -> Option<HeaderValue> {
    match cors_origin {
        None => Some(HeaderValue::from_static("*")),
        Some(cfg) => {
            if req_origin == Some(cfg.as_str()) {
                HeaderValue::from_str(cfg).ok()
            } else {
                None
            }
        }
    }
}

fn header_value_of(s: &str) -> Option<HeaderValue> {
    HeaderValue::from_str(s).ok()
}

/// CORS + en-têtes finaux dans l'ordre des captures :
/// `[ACAO] + en-têtes handler + [vary: Origin] + [content-length] + [Date]`.
/// Préempte les OPTIONS en 204 AVANT routage (toute route, comme le
/// middleware hono placé avant le routeur — même sur /nope).
async fn cors_middleware<L2: crate::cache::L2Cache>(
    State(state): State<AppState<L2>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let is_options = req.method() == Method::OPTIONS;
    let is_head = req.method() == Method::HEAD;
    let req_origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let allow_origin = allow_origin_of(&state.cors_origin, req_origin.as_deref());

    if is_options {
        let acrh = req
            .headers()
            .get("access-control-request-headers")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let mut headers = HeaderMap::new();
        if let Some(ach) = &acrh {
            // echo : requestHeaders.split(",").map(h => h.trim()).join(",")
            let joined = ach
                .split(',')
                .map(|h| h.trim())
                .collect::<Vec<_>>()
                .join(",");
            if let Some(v) = header_value_of(&joined) {
                headers.insert(HeaderName::from_static("access-control-allow-headers"), v);
            }
        }
        headers.insert(
            HeaderName::from_static("access-control-allow-methods"),
            HeaderValue::from_static("GET"),
        );
        if let Some(ao) = allow_origin {
            headers.insert(HeaderName::from_static("access-control-allow-origin"), ao);
        }
        headers.insert(
            HeaderName::from_static("access-control-max-age"),
            HeaderValue::from_static(AC_MAX_AGE),
        );
        // hono : append("Vary", "Origin") si origin != '*' ;
        // append("Vary", "Access-Control-Request-Headers") si echo ACH —
        // Headers.append combine en UNE ligne « Origin, Access-Control-Request-Headers ».
        let mut vary_parts: Vec<&str> = Vec::new();
        if state.cors_origin.is_some() {
            vary_parts.push("Origin");
        }
        if acrh.is_some() {
            vary_parts.push("Access-Control-Request-Headers");
        }
        if !vary_parts.is_empty() {
            let v = vary_parts.join(", ");
            if let Some(hv) = header_value_of(&v) {
                headers.insert(header::VARY, hv);
            }
        }
        let mut res = (StatusCode::NO_CONTENT, headers, Body::empty()).into_response();
        set_date_header(res.headers_mut());
        return res;
    }

    let (mut parts, body) = req.into_parts();
    let _ = &mut parts;
    let response = next.run(Request::from_parts(parts, body)).await;

    let is_204 = response.status() == StatusCode::NO_CONTENT;
    let (mut res_parts, res_body) = response.into_parts();
    // corps buffé (SVG/JSON petits) pour poser content-length à la position du
    // legacy — avant Date, après les en-têtes applicatifs.
    // INVARIANT (m-4 revue P4) : `to_bytes(…, 64 Mio)` ne peut échouer QUE sur
    // un corps > 64 Mio ou une erreur de lecture du flux — impossible ici :
    // tous les corps produits par l'app (SVG ≤ ~15 Ko, JSON data ≤ ~50 Ko,
    // probe corpus 512) sont très en dessous, et axum/hyper sert des corps
    // entiers en mémoire. L'err → corps vide ne peut donc JAMAIS masquer une
    // réponse réelle (choix revue : invariant commenté plutôt que 500
    // synthétique qui masquerait la vraie cause si l'invariant sautait).
    let body_bytes: Vec<u8> = match axum::body::to_bytes(res_body, 64 * 1024 * 1024).await {
        Ok(b) => b.to_vec(),
        Err(_) => Vec::new(),
    };
    let handler_headers: Vec<(HeaderName, HeaderValue)> = res_parts
        .headers
        .drain()
        .filter_map(|(k, v)| k.map(|k| (k, v)))
        .collect();

    let mut headers = HeaderMap::with_capacity(handler_headers.len() + 4);
    if let Some(ao) = allow_origin {
        headers.insert(HeaderName::from_static("access-control-allow-origin"), ao);
    }
    for (k, v) in handler_headers {
        if k == header::CONTENT_LENGTH || k == header::TRANSFER_ENCODING || k == header::VARY {
            continue; // posés plus bas (ou absents), jamais réordonnés
        }
        headers.insert(k, v);
    }
    if state.cors_origin.is_some() {
        if let Some(v) = header_value_of("Origin") {
            headers.insert(header::VARY, v);
        }
    }
    if !is_head && !is_204 {
        if let Some(v) = header_value_of(&body_bytes.len().to_string()) {
            headers.insert(header::CONTENT_LENGTH, v);
        }
    }
    set_date_header(&mut headers);
    (res_parts.status, headers, Body::from(body_bytes)).into_response()
}

// ------------------------------------------------------------ utilitaires date

pub fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// `new Date().toISOString()` (UTC, ms à 3 chiffres, `Z`).
pub fn iso8601_ms_utc(now_ms: u64) -> String {
    let ms = now_ms as i64;
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let (y, mo, d) = crate::jsdate::civil_from_days_full(days);
    format!(
        "{y:04}-{mo:02}-{d:02}T{:02}:{:02}:{:02}.{ms3:03}Z",
        rem / 3_600_000,
        (rem % 3_600_000) / 60_000,
        (rem % 60_000) / 1000,
        ms3 = rem % 1000
    )
}

const WDAY: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
const MON: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// En-tête `Date` HTTP (IMF-fixdate RFC 7231), comme le serveur Node.
fn http_date(now_ms: u64) -> String {
    let ms = now_ms as i64;
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let (y, mo, d) = crate::jsdate::civil_from_days_full(days);
    let wd = WDAY[days.rem_euclid(7) as usize];
    format!(
        "{wd}, {d:02} {} {y:04} {:02}:{:02}:{:02} GMT",
        MON[(mo as usize - 1).min(11)],
        rem / 3_600_000,
        (rem % 3_600_000) / 60_000,
        (rem % 60_000) / 1000
    )
}

fn set_date_header(headers: &mut HeaderMap) {
    if let Some(v) = header_value_of(&http_date(now_unix_ms())) {
        headers.insert(header::DATE, v);
    }
}

/// HEAD → GET (hono-base.js:257) : statut + en-têtes du GET, corps vidé,
/// content-length ABSENT (captures head_health : content-type présent,
/// pas de content-length ; cors ajoute ensuite vary/Date comme pour un GET).
async fn head_to_get_middleware(req: Request<Body>, next: Next) -> Response {
    let is_head = req.method() == Method::HEAD;
    let mut res = if is_head {
        let (mut parts, _) = req.into_parts();
        parts.method = Method::GET;
        next.run(Request::from_parts(parts, Body::empty())).await
    } else {
        next.run(req).await
    };
    if is_head {
        res.headers_mut().remove(header::CONTENT_LENGTH);
        res.headers_mut().remove(header::TRANSFER_ENCODING);
        *res.body_mut() = Body::empty();
    }
    res
}

// ------------------------------------------------------------------ routes

/// GET /health et /healthz (health.ts) — clés `status` puis `timestamp`,
/// ISO 8601 ms UTC.
async fn health() -> Response {
    let body = format!(
        "{{\"status\":\"ok\",\"timestamp\":\"{}\"}}",
        iso8601_ms_utc(now_unix_ms())
    );
    json_response(body)
}

/// GET /api/themes (data.ts:61-63) — tableau THEME_NAMES, JSON.stringify.
async fn themes() -> Response {
    let body = crate::js_json::JsJson::Arr(
        THEME_NAMES
            .iter()
            .map(|t| crate::js_json::JsJson::Str((*t).to_string()))
            .collect(),
    )
    .to_json_string();
    json_response(body)
}

/// Échec d'une route carte (card-factory.ts:70-91) — les SVG d'erreur ne
/// sont JAMAIS mis en cache (aucun cache.set sur ces chemins).
pub enum RouteFailure {
    NotFound {
        username: String,
    },
    RateLimit,
    AllPatTokens,
    /// Autre (fetch failed, GraphQL errors, rendu) → SVG générique 502.
    Generic(AppError),
}

impl From<GitHubError> for RouteFailure {
    fn from(e: GitHubError) -> Self {
        match e {
            GitHubError::NotFound { username } => RouteFailure::NotFound { username },
            GitHubError::RateLimit { .. } => RouteFailure::RateLimit,
            // card-factory.ts:81-85 : attrapé PAR MESSAGE → 429 SVG dédié
            GitHubError::AllPatTokensRateLimited => RouteFailure::AllPatTokens,
            other => RouteFailure::Generic(AppError(other.to_string())),
        }
    }
}

fn card_error_response(failure: RouteFailure, username: &str, prefix: &str) -> Response {
    match failure {
        RouteFailure::NotFound { username } => error_svg_response(
            &format!("User not found: {username}"),
            StatusCode::NOT_FOUND,
            SVG_CACHE_SECONDS_404,
        ),
        RouteFailure::RateLimit => error_svg_response(
            "GitHub API rate limit exceeded. Please try again later.",
            StatusCode::TOO_MANY_REQUESTS,
            SVG_CACHE_SECONDS_ERROR,
        ),
        RouteFailure::AllPatTokens => error_svg_response(
            "All API tokens are rate-limited. Please try again later.",
            StatusCode::TOO_MANY_REQUESTS,
            SVG_CACHE_SECONDS_ERROR,
        ),
        RouteFailure::Generic(e) => {
            eprintln!("Error generating {prefix} card for {username}: {}", e.0);
            error_svg_response(
                "An error occurred while generating the card.",
                StatusCode::BAD_GATEWAY,
                SVG_CACHE_SECONDS_ERROR,
            )
        }
    }
}

/// Données d'une route : `data:{username}` (hit → chaîne stockée telle
/// quelle) ou fetch → cache.set(CACHE_TTL) → JSON.stringify unique.
async fn data_json<L2: crate::cache::L2Cache>(
    state: &AppState<L2>,
    username: &str,
) -> Result<String, RouteFailure> {
    let key = format!("data:{username}");
    if let Some(cached) = state.cache.get(&key).await {
        // `/api/data` renvoie la chaîne stockée telle quelle (byte-identique :
        // JSON.stringify(JSON.parse(s)) est stable pour une sortie stringify).
        return Ok(cached);
    }
    let result = fetch_with_retry_shared(&state.http, &state.pool, &state.github_url, username)
        .await
        .map_err(RouteFailure::from)?;
    let stored = result.to_json_string();
    state.cache.set(&key, stored.clone(), state.cache_ttl).await;
    Ok(stored)
}

/// GET /api/data/{username} (data.ts:35-60) — l'échec du pool PAT n'y est
/// reconnu par AUCUNE branche du catch (data.ts:50-59) → 500 JSON générique
/// (revue M-B : statut legacy = 500 ; les cartes, elles, renvoient 429).
async fn data_route<L2: crate::cache::L2Cache>(
    State(state): State<AppState<L2>>,
    Path(username): Path<String>,
) -> Response {
    match data_json(&state, &username).await {
        Ok(body) => json_response(body),
        Err(RouteFailure::NotFound { username }) => json_response_status(
            format!("{{\"error\":\"User not found: {username}\"}}"),
            StatusCode::NOT_FOUND,
        ),
        Err(RouteFailure::RateLimit) => json_response_status(
            "{\"error\":\"GitHub API rate limit exceeded\"}".to_string(),
            StatusCode::TOO_MANY_REQUESTS,
        ),
        Err(RouteFailure::AllPatTokens) => json_response_status(
            "{\"error\":\"Internal server error\"}".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        Err(RouteFailure::Generic(e)) => {
            eprintln!("Error fetching data for {username}: {}", e.0);
            json_response_status(
                "{\"error\":\"Internal server error\"}".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    }
}

/// Corps commun des 4 cartes (card-factory.ts:27-94) : parseCardOptions AVANT
/// le cache (zod throw → onError 500), hash 8 hex du JSON.stringify(options),
/// clés `svg:{prefix}:{username}:{hash}` et `data:{username}` ; erreur → SVG
/// jamais caché.
async fn card_route<L2: crate::cache::L2Cache>(
    state: AppState<L2>,
    username: String,
    raw_query: Option<String>,
    prefix: &'static str,
    render: fn(
        &serde_json::Value,
        &crate::types::CardOptions,
    ) -> Result<String, crate::cards::RenderError>,
) -> Response {
    let parsed = match parse_card_options(raw_query.as_deref()) {
        Ok(p) => p,
        Err(e) => return AppError::from(e).into_response(),
    };
    let params_hash = crate::params_hash(&parsed.options_json);
    let svg_key = format!("svg:{prefix}:{username}:{params_hash}");
    if let Some(svg) = state.cache.get(&svg_key).await {
        return svg_response(svg, parsed.cache_seconds);
    }

    let data_key = format!("data:{username}");
    let render_result: Result<String, RouteFailure> = async {
        let data_str: String = if let Some(cached) = state.cache.get(&data_key).await {
            cached
        } else {
            let result =
                fetch_with_retry_shared(&state.http, &state.pool, &state.github_url, &username)
                    .await
                    .map_err(RouteFailure::from)?;
            let s = result.to_json_string();
            state.cache.set(&data_key, s.clone(), state.cache_ttl).await;
            s
        };
        let value: serde_json::Value = serde_json::from_str(&data_str).map_err(|_| {
            // divergence : le message SyntaxError V8 exact n'est pas reproduit
            // (classe conservée → 502 SVG générique, comme le catch legacy)
            RouteFailure::Generic(AppError("invalid cached JSON".to_string()))
        })?;
        // le legacy rend `data.stats` / `data.streak` / `data.languages` /
        // `data.profile` (routes/*.ts) — section du FetchResult stocké
        let section_key = if prefix == "langs" {
            "languages"
        } else {
            prefix
        };
        let section = value
            .get(section_key)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        render(&section, &parsed.options)
            .map_err(|e| RouteFailure::Generic(AppError(format!("{e}"))))
    }
    .await;

    match render_result {
        Ok(svg) => {
            state
                .cache
                .set(&svg_key, svg.clone(), parsed.cache_seconds)
                .await;
            svg_response(svg, parsed.cache_seconds)
        }
        Err(failure) => card_error_response(failure, &username, prefix),
    }
}

async fn stats_route<L2: crate::cache::L2Cache>(
    State(state): State<AppState<L2>>,
    Path(username): Path<String>,
    RawQuery(q): RawQuery,
) -> Response {
    card_route(state, username, q, "stats", |v, o| {
        crate::render_stats_card(&crate::GitHubStats::from_value(v), o)
    })
    .await
}

async fn streak_route<L2: crate::cache::L2Cache>(
    State(state): State<AppState<L2>>,
    Path(username): Path<String>,
    RawQuery(q): RawQuery,
) -> Response {
    card_route(state, username, q, "streak", |v, o| {
        crate::render_streak_card(&crate::StreakData::from_value(v), o)
    })
    .await
}

async fn langs_route<L2: crate::cache::L2Cache>(
    State(state): State<AppState<L2>>,
    Path(username): Path<String>,
    RawQuery(q): RawQuery,
) -> Response {
    card_route(state, username, q, "langs", |v, o| {
        crate::TopLangsData::from_value(v).and_then(|d| crate::render_langs_card(&d, o))
    })
    .await
}

async fn profile_route<L2: crate::cache::L2Cache>(
    State(state): State<AppState<L2>>,
    Path(username): Path<String>,
    RawQuery(q): RawQuery,
) -> Response {
    card_route(state, username, q, "profile", |v, o| {
        crate::ProfileData::from_value(v).and_then(|d| crate::render_profile_card(&d, o))
    })
    .await
}

/// 404 JSON (index.ts:49-51).
async fn not_found() -> Response {
    json_response_status(
        "{\"error\":\"Not found\"}".to_string(),
        StatusCode::NOT_FOUND,
    )
}

// Les routes hono ne matchent QUE GET (`healthRoute.get`, `createCardRoute
// .get`) : toute autre méthode tombe dans le notFound (404 JSON) — axum
// renverrait 405 avec `get()` seul. Gates `any()` + garde de méthode.
// (OPTIONS est déjà préempté en 204 par cors ; HEAD est converti en GET.)

async fn health_gate(m: Method) -> Response {
    if m == Method::GET {
        health().await
    } else {
        not_found().await
    }
}

async fn themes_gate(m: Method) -> Response {
    if m == Method::GET {
        themes().await
    } else {
        not_found().await
    }
}

async fn data_gate<L2: crate::cache::L2Cache>(
    m: Method,
    state: State<AppState<L2>>,
    username: Path<String>,
) -> Response {
    if m == Method::GET {
        data_route(state, username).await
    } else {
        not_found().await
    }
}

macro_rules! card_gate {
    ($name:ident, $inner:ident) => {
        async fn $name<L2: crate::cache::L2Cache>(
            m: Method,
            state: State<AppState<L2>>,
            username: Path<String>,
            q: RawQuery,
        ) -> Response {
            if m == Method::GET {
                $inner(state, username, q).await
            } else {
                not_found().await
            }
        }
    };
}

card_gate!(stats_gate, stats_route);
card_gate!(streak_gate, streak_route);
card_gate!(langs_gate, langs_route);
card_gate!(profile_gate, profile_route);

/// Construit le routeur complet (mêmes motifs que index.ts + routes/*) :
/// health → data → stats → streak → top-langs → profile → fallback.
pub fn build_router<L2: crate::cache::L2Cache>(state: AppState<L2>) -> Router {
    Router::new()
        .route("/health", any(health_gate))
        .route("/healthz", any(health_gate))
        .route("/api/themes", any(themes_gate))
        .route("/api/data/{username}", any(data_gate))
        .route("/stats/{username}", any(stats_gate))
        .route("/stats/{username}/streak", any(streak_gate))
        .route("/stats/{username}/top-langs", any(langs_gate))
        .route("/stats/{username}/profile", any(profile_gate))
        .fallback(not_found)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            cors_middleware::<L2>,
        ))
        .layer(axum::middleware::from_fn(head_to_get_middleware))
        .with_state(state)
}

/// État de prod : URL réelle, client reqwest par défaut (parité la plus
/// proche d'undici : aucun timeout applicatif, redirections suivies —
/// point ouvert P3 arbitré ici, cf. rapport).
pub fn prod_state<L2: crate::cache::L2Cache>(
    cache: AppCache<L2>,
    pool: PatPool,
    cache_ttl: f64,
    cors_origin: Option<String>,
) -> AppState<L2> {
    AppState {
        cache: Arc::new(cache),
        pool: Arc::new(Mutex::new(pool)),
        http: reqwest::Client::new(),
        github_url: GITHUB_GRAPHQL_URL.to_string(),
        cache_ttl,
        cors_origin,
    }
}
