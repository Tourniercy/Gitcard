//! `parseCardOptions(query)` — port de `apps/api/src/utils/query-params.ts`
//! (zod v4), oracle = legacy exécuté (`tests/golden/http/params-hash.json`).
//!
//! Sémantique épinglée :
//! - le record vu par le schéma = Hono `c.req.query()` : clés répétées →
//!   **premier gagne** (`results[name] ??= value`, hono `utils/url.js` ; épinglé
//!   aussi côté HTTP par les sondes `get_stats_q_dupe1/2`) ;
//! - décodage hono `_decodeURI` : `+` → espace TOUJOURS, puis `%XX` si la
//!   valeur en contient (malformé → valeur brute, comme `tryDecode`) ;
//! - `show_icons`/`hide_border`/`hide_title` : enum `['true','false','1','0','']`
//!   → booléen ; toute autre valeur → ZodError → la route renvoie 500 JSON ;
//! - `cache_seconds` : `z.coerce.number().catch(14400).transform(max(v,1800))`
//!   — `Number("")` = 0 → 1800, `Number("abc")` = NaN → catch 14400,
//!   `"Infinity"` → Infinity → catch 14400 AUSSI (l'oracle `cache-inf` et le
//!   code legacy tranchent : `catch` englobe TOUTE valeur invalide, Infinity
//!   compris → `max-age=14400`, hash identique à no-query `289b4511` —
//!   le commentaire précédent (« max-age=Infinity reproduit, étude §1.3 »)
//!   était PÉRIMÉ, m-1 revue P4) ;
//! - `locale` ∉ `LOCALE_CODES` → `'en'` ; clés inconnues ignorées (zod strip) ;
//! - l'objet `CardOptions` construit a TOUJOURS l'ordre de clés `theme, hide,
//!   showIcons, hideBorder, hideTitle, bgColor?, titleColor?, textColor?,
//!   iconColor?, borderColor?, cacheSeconds, locale, layout?` — les `undefined`
//!   sont omis par `JSON.stringify` (objet hashé par `paramsHash`).

use crate::js_json::JsJson;
use crate::types::{locale_codes, CardOptions};

/// Échec de parsing (le legacy jette `ZodError`, attrapé par `app.onError`
/// → 500 JSON `{"error":"Internal server error"}`).
#[derive(Debug, Clone, PartialEq)]
pub struct QueryError {
    pub path: &'static str,
    pub code: &'static str,
    /// Message zod v4 (invalid_value : `Invalid option: expected one of …`,
    /// format épinglé par la capture config LOG_LEVEL).
    pub message: String,
}

use std::fmt;

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// Résultat de `parseCardOptions` : les options typées (rendu), la
/// sérialisation JSON.stringify EXACTE (hash de cache) et `cacheSeconds` en
/// f64 (les en-têtes Cache-Control : `Infinity` doit rester `Infinity`).
pub struct ParsedOptions {
    pub options: CardOptions,
    pub options_json: String,
    pub cache_seconds: f64,
}

/// Record post-Hono : paires (clé, valeur) décodées, PREMIER gagne.
pub fn hono_query_record(raw_query: &str) -> Vec<(String, String)> {
    let mut record: Vec<(String, String)> = Vec::new();
    for pair in raw_query.split('&') {
        let (k_raw, v_raw) = match pair.find('=') {
            Some(i) => (&pair[..i], &pair[i + 1..]),
            None => (pair, ""),
        };
        let k = decode_hono(k_raw);
        // hono ignore les paires à nom vide (`if (name === "") continue`)
        if k.is_empty() {
            continue;
        }
        let v = decode_hono(v_raw);
        if !record.iter().any(|(ek, _)| *ek == k) {
            record.push((k, v)); // `results[name] ??= value` : la première reste
        }
    }
    record
}

/// `_decodeURI` de hono : `+` → espace TOUJOURS, puis `%XX` si présent.
/// `tryDecode` avale les erreurs (`decodeURIComponent` jette URIError sur
/// % malformé ou suite UTF-8 invalide → chaîne BRUTE rendue).
fn decode_hono(s: &str) -> String {
    let s = if s.contains('+') {
        s.replace('+', " ")
    } else {
        s.to_string()
    };
    if !s.contains('%') {
        return s;
    }
    let bytes = s.as_bytes();
    // validation préalable : tout % doit être suivi de 2 hex
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let ok = i + 2 < bytes.len()
                && (bytes[i + 1] as char).is_ascii_hexdigit()
                && (bytes[i + 2] as char).is_ascii_hexdigit();
            if !ok {
                return s; // URIError → brut
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    // décodage complet
    let mut out = Vec::with_capacity(bytes.len());
    i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hi = (bytes[i + 1] as char).to_digit(16).unwrap() as u16;
            let lo = (bytes[i + 2] as char).to_digit(16).unwrap() as u16;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    match String::from_utf8(out) {
        Ok(decoded) => decoded,
        Err(_) => s, // suite UTF-8 invalide → URIError → brut
    }
}

/// `Number(v)` JS pour `z.coerce.number()` : trim, `""` → 0, `Infinity`
/// signé, radix `0x/0o/0b`, sinon parsing décimal ; tout le reste → NaN.
pub fn js_number(v: &str) -> f64 {
    let t = v.trim_matches(|c: char| c.is_whitespace());
    if t.is_empty() {
        return 0.0;
    }
    let (neg, rest) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    if rest == "Infinity" {
        return if neg {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let mag = if let Some(hex) = rest.strip_prefix("0x").or(rest.strip_prefix("0X")) {
        return radix_number(neg, hex, 16);
    } else if let Some(oct) = rest.strip_prefix("0o").or(rest.strip_prefix("0O")) {
        return radix_number(neg, oct, 8);
    } else if let Some(bin) = rest.strip_prefix("0b").or(rest.strip_prefix("0B")) {
        return radix_number(neg, bin, 2);
    } else {
        rest
    };
    // Rust accepte "inf"/"NaN" que `Number` refuse → écartés d'abord (NaN).
    let lower = mag.to_ascii_lowercase();
    if lower.contains("inf") || lower.contains("nan") {
        return f64::NAN;
    }
    let parsed: f64 = mag.parse().unwrap_or(f64::NAN);
    if parsed == 0.0 && neg && !parsed.is_sign_negative() {
        return -0.0;
    }
    parsed
}

fn radix_number(neg: bool, digits: &str, radix: u32) -> f64 {
    match u64::from_str_radix(digits, radix) {
        Ok(n) => {
            let v = n as f64;
            if neg {
                -v
            } else {
                v
            }
        }
        Err(_) => f64::NAN,
    }
}

/// enum `['true','false','1','0','']` → booléen (`v === 'true' || v === '1'`).
fn boolean_string(v: &str, path: &'static str) -> Result<bool, QueryError> {
    match v {
        "true" | "1" => Ok(true),
        "false" | "0" | "" => Ok(false),
        _ => Err(QueryError {
            path,
            code: "invalid_value",
            message: "Invalid option: expected one of \"true\"|\"false\"|\"1\"|\"0\"|\"\"".into(),
        }),
    }
}

/// `parseCardOptions(raw_query)` — `raw_query` = query string BRUTE (sans `?`),
/// `None` pour l'absence totale de query.
pub fn parse_card_options(raw_query: Option<&str>) -> Result<ParsedOptions, QueryError> {
    let record = raw_query.map(hono_query_record).unwrap_or_default();
    let get = |k: &str| record.iter().find(|(ek, _)| ek == k).map(|(_, v)| v);

    // theme: z.string().default('default')
    let theme = get("theme")
        .cloned()
        .unwrap_or_else(|| "default".to_string());
    // hide: z.string().default('').transform(v => v ? v.split(',').map(trim) : [])
    let hide: Vec<String> = match get("hide") {
        None => Vec::new(),
        Some(v) if v.is_empty() => Vec::new(),
        Some(v) => v.split(',').map(|s| s.trim().to_string()).collect(),
    };
    // show_icons: enum → bool, default(true)
    let show_icons = match get("show_icons") {
        None => true,
        Some(v) => boolean_string(v, "show_icons")?,
    };
    // hide_border / hide_title: enum → bool, default(false)
    let hide_border = match get("hide_border") {
        None => false,
        Some(v) => boolean_string(v, "hide_border")?,
    };
    let hide_title = match get("hide_title") {
        None => false,
        Some(v) => boolean_string(v, "hide_title")?,
    };
    // couleurs : z.string().optional() (toute chaîne, y compris vide)
    let opt_str = |k: &str| get(k).cloned();
    let bg_color = opt_str("bg_color");
    let title_color = opt_str("title_color");
    let text_color = opt_str("text_color");
    let icon_color = opt_str("icon_color");
    let border_color = opt_str("border_color");
    // cache_seconds: z.coerce.number().catch(14400).transform(max(v,1800)).default(14400)
    // ORACLE (params-hash.json, cas cache-inf) : zod v4 z.number() refuse ±∞
    // (l'étude §1.3 annonçait max-age=Infinity — FAUX, le code capture prime)
    // → Number('Infinity') = ∞ → catch(14400). NaN → catch. "" → 0 → 1800.
    let cache_seconds = match get("cache_seconds") {
        None => 14400.0,
        Some(v) => {
            let n = js_number(v);
            if n.is_nan() || !n.is_finite() {
                14400.0
            } else if n < 1800.0 {
                1800.0
            } else {
                n
            }
        }
    };
    // locale: z.string().default('en').transform(in LOCALE_CODES ? v : 'en')
    let locale = match get("locale") {
        None => "en".to_string(),
        Some(v) => {
            if locale_codes().contains(&v.as_str()) {
                v.clone()
            } else {
                "en".to_string()
            }
        }
    };
    let layout = opt_str("layout");

    let options = CardOptions {
        theme: theme.clone(),
        hide: hide.clone(),
        show_icons,
        hide_border,
        hide_title,
        bg_color: bg_color.clone(),
        title_color: title_color.clone(),
        text_color: text_color.clone(),
        icon_color: icon_color.clone(),
        border_color: border_color.clone(),
        cache_seconds: cache_seconds as i64, // rendu : jamais dans les octets SVG (manifeste)
        locale: locale.clone(),
        layout: layout.clone(),
    };
    let options_json = options_json_string(&options, cache_seconds);
    Ok(ParsedOptions {
        options,
        options_json,
        cache_seconds,
    })
}

/// `JSON.stringify(options)` — ordre d'insertion du littéral de
/// `parseCardOptions`, clés `undefined` omises (`js_json`), nombres via
/// `js_f64_to_string` (`Infinity` → `Infinity`). `cache_seconds_f64` prime sur
/// le champ i64 (le cast saturant ne doit JAMAIS entrer dans le hash).
pub fn options_json_string(o: &CardOptions, cache_seconds_f64: f64) -> String {
    JsJson::Obj(vec![
        ("theme".into(), JsJson::Str(o.theme.clone())),
        (
            "hide".into(),
            JsJson::Arr(o.hide.iter().map(|h| JsJson::Str(h.clone())).collect()),
        ),
        ("showIcons".into(), JsJson::Bool(o.show_icons)),
        ("hideBorder".into(), JsJson::Bool(o.hide_border)),
        ("hideTitle".into(), JsJson::Bool(o.hide_title)),
        ("bgColor".into(), opt_json(&o.bg_color)),
        ("titleColor".into(), opt_json(&o.title_color)),
        ("textColor".into(), opt_json(&o.text_color)),
        ("iconColor".into(), opt_json(&o.icon_color)),
        ("borderColor".into(), opt_json(&o.border_color)),
        ("cacheSeconds".into(), JsJson::Num(cache_seconds_f64)),
        ("locale".into(), JsJson::Str(o.locale.clone())),
        ("layout".into(), opt_json(&o.layout)),
    ])
    .to_json_string()
}

fn opt_json(v: &Option<String>) -> JsJson {
    match v {
        Some(s) => JsJson::Str(s.clone()),
        None => JsJson::Undefined, // omis par to_json_string (comme JSON.stringify)
    }
}
