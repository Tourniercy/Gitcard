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
