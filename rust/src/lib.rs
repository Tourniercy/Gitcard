//! P0 de la réécriture Rust de gitcard-api : primitives numériques JS, tables Intl,
//! octicons et hash de paramètres — l'outillage de parité validé à la porte G1.
//!
//! Modules :
//! - [`jsnum`] : `js_f64_to_string` / `js_to_fixed` (sémantique exacte Node 26.13.1)
//! - [`intl`]  : formateurs entiers/dates par table extraite de Node (20 locales)
//! - [`octicons`] : icônes embarquées `@primer/octicons@19.23.1`
//! - `params_hash` / `md5_hex` : ETag et hash de cache du legacy (étude §1.7)

pub mod fdlibm;
pub mod intl;
pub mod jsnum;
pub mod octicons;

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
