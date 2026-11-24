//! P0 de la réécriture Rust de gitcard-api : primitives numériques JS, tables Intl,
//! octicons et hash de paramètres — l'outillage de parité validé à la porte G1.
//!
//! Modules :
//! - [`jsnum`] : `js_f64_to_string` / `js_to_fixed` (sémantique exacte Node 26.13.1)
//! - [`jsval`] : `JsVal` — champ JSON vu par V8 (fix2 R2 : compteurs/texte)
//! - [`jsdate`] : `new Date(x)` V8 + formatage Intl 'en' (fix2 R1 : dates)
//! - [`intl`]  : formateurs entiers/dates par table extraite de Node (20 locales)
//! - [`octicons`] : icônes embarquées `@primer/octicons@19.23.1`
//! - `params_hash` / `md5_hex` : ETag et hash de cache du legacy (étude §1.7)
//! - [`sanitize`] / [`themes`] / [`types`] / [`cards`] : port P2 du renderer SVG
//!   (`packages/svg-renderer/src/`), parité à l'octet vs corpus golden (porte G2).
//!
//! Fix2 (revue P2) : les quatre `render_*_card` retournent `Result<_, RenderError>`
//! — là où le legacy lève (RangeError date invalide, TypeError encodeHTML/toFixed),
//! le Rust retourne une erreur capturable, plus aucun panic sur donnée.

pub mod app;
pub mod cache;
pub mod cards;
pub mod config;
pub mod fdlibm;
pub mod github;
pub mod intl;
pub mod js_json;
pub mod jsdate;
pub mod jsnum;
pub mod jsval;
pub mod octicons;
pub mod pat_pool;
pub mod query_params;
pub mod ratelimit;
pub mod sanitize;
pub mod sentry_gate;
pub mod themes;
pub mod types;

// Surface publique équivalente à `packages/svg-renderer/src/index.ts`.
pub use cards::base::{
    create_arc_path, create_card_wrapper, create_donut_segment, create_ring_chart, encode_html,
    format_number, octicon_svg, set_card_id_counter,
};
pub use cards::{
    render_langs_card, render_profile_card, render_stats_card, render_streak_card, RenderError,
};
pub use sanitize::encode_html as encode_html_reexport;
pub use themes::{get_theme, THEME_GROUPS, THEME_NAMES};
pub use types::{
    country_to_flag, locale_codes, supported_locales, CardOptions, ContributionDay, GitHubStats,
    LanguageData, ProfileData, StreakData, TopLangsData,
};

use md5::{Digest, Md5};

const HEX: &[u8; 16] = b"0123456789abcdef";

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[usize::from(b >> 4)] as char);
        s.push(HEX[usize::from(b & 0x0f)] as char);
    }
    s
}

/// ETag du legacy : md5 hexadécimal complet du SVG (utf8), entrelacé de guillemets côté HTTP.
pub fn md5_hex(s: &str) -> String {
    to_hex(&Md5::digest(s.as_bytes()))
}

/// Hash de cache du legacy : `md5(JSON.stringify(CardOptions)).slice(0, 8)` (étude §1.7).
/// L'ordre d'insertion des clés doit être celui de `parseCardOptions` (theme, hide,
/// showIcons, hideBorder, hideTitle, couleurs?, cacheSeconds, locale, layout?).
pub fn params_hash(options_json: &str) -> String {
    let h = Md5::digest(options_json.as_bytes());
    let full = to_hex(&h);
    full[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_de_parametres_ancres_etude_1_7() {
        // Exemple de l'étude §1.7 (requête sans query) — vérifié aussi sur Node :
        // md5('{"theme":"default",…,"locale":"en"}').slice(0,8) = "289b4511"
        let options = r#"{"theme":"default","hide":[],"showIcons":true,"hideBorder":false,"hideTitle":false,"cacheSeconds":14400,"locale":"en"}"#;
        assert_eq!(params_hash(options), "289b4511");
    }

    #[test]
    fn md5_etag() {
        assert_eq!(md5_hex("hello"), "5d41402abc4b2a76b9719d911017c592");
    }
}
