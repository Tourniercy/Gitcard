//! Cartes du renderer (port de `packages/svg-renderer/src/cards/`).
//!
//! Fix2 R1/R2 : les quatre `render_*_card` retournent `Result<String, RenderError>`
//! — là où le legacy LÈVE (RangeError « Invalid time value » sur date invalide,
//! TypeError sur `encodeHTML(null)` / `percentage.toFixed`), le Rust retourne une
//! erreur que la couche HTTP traduira en 500 (P6). Plus aucun `panic!` sur donnée.

pub mod base;
pub mod langs;
pub mod profile;
pub mod stats;
pub mod streak;

pub use langs::render_langs_card;
pub use profile::render_profile_card;
pub use stats::render_stats_card;
pub use streak::render_streak_card;

use std::fmt;

/// `errorSvg(message, width = 495, height = 120)` — gabarit VERBATIM de
/// `apps/api/src/routes/card-response.ts:17-25` (oracle : tests/golden/errors/,
/// générés par le legacy). Message NON échappé (`username` brut, écart assumé
/// « ne pas réparer l'injection » — étude §2.9), `.trim()` final, `width/2`
/// → `247.5` et `height/2` → `60` littéraux. Ne consomme PAS le compteur d'uid
/// (pas de `createCardWrapper`).
pub fn error_svg(message: &str) -> String {
    const WIDTH: f64 = 495.0;
    const HEIGHT: f64 = 120.0;
    let body = format!(
        "\n<svg width=\"{WIDTH}\" height=\"{HEIGHT}\" viewBox=\"0 0 {WIDTH} {HEIGHT}\" xmlns=\"http://www.w3.org/2000/svg\">\n  <rect width=\"{WIDTH}\" height=\"{HEIGHT}\" rx=\"8\" fill=\"#fef2f2\" stroke=\"#fca5a5\" />\n  <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" dominant-baseline=\"central\"\n        font-family=\"'Segoe UI', system-ui, sans-serif\" font-size=\"14\" fill=\"#dc2626\">\n    {message}\n  </text>\n</svg>",
        crate::jsnum::js_f64_to_string(WIDTH / 2.0),
        crate::jsnum::js_f64_to_string(HEIGHT / 2.0),
    );
    body.trim().to_string()
}

/// Équivalent capturable des exceptions JS du renderer legacy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// `RangeError` JS — ex. `Intl.format(Invalid Date)` → « Invalid time value ».
    RangeError(String),
    /// `TypeError` JS — ex. `null.replace(...)` / `("42").toFixed(1)`.
    TypeError(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::RangeError(m) => write!(f, "RangeError: {m}"),
            RenderError::TypeError(m) => write!(f, "TypeError: {m}"),
        }
    }
}

impl std::error::Error for RenderError {}
