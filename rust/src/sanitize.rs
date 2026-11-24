//! Port littéral de `packages/svg-renderer/src/utils/sanitize.ts` (13 lignes) :
//! échappement HTML pour interpolation dans le SVG.
//!
//! Table d'échappement exacte (ordre sans importance : remplacement en une seule
//! passe par caractères, jamais récursif) : `& < > " '`.

const ESCAPE_MAP: &[(char, &str)] = &[
    ('&', "&amp;"),
    ('<', "&lt;"),
    ('>', "&gt;"),
    ('"', "&quot;"),
    ('\'', "&#39;"),
];

/// Équivalent exact de `encodeHTML(str)` : `str.replace(/[&<>"']/g, c => MAP[c])`.
/// Une seule passe : les `&` produits (`&amp;`) ne sont PAS ré-échappés.
pub fn encode_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match ESCAPE_MAP.iter().find(|(k, _)| *k == c) {
            Some((_, repl)) => out.push_str(repl),
            None => out.push(c),
        }
    }
    out
}

/// `encodeHTML(x)` sur toute valeur (fix2 R2) : le legacy appelle `.replace(...)`
/// sur l'argument brut — `null`/`undefined` → TypeError « Cannot read properties …
/// (reading 'replace') », tout autre non-string → TypeError « … is not a function ».
/// Le Rust retourne [`crate::cards::RenderError::TypeError`] au lieu de paniquer.
pub fn encode_html_js(v: &crate::jsval::JsVal) -> Result<String, crate::cards::RenderError> {
    match v {
        crate::jsval::JsVal::Str(s) => Ok(encode_html(s)),
        other => Err(crate::cards::RenderError::TypeError(format!(
            "encodeHTML({}) : le legacy appellerait .replace sur un {}",
            other.type_name(),
            other.type_name()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_exacte_une_seule_passe() {
        assert_eq!(
            encode_html("a&b<c>d\"e'f"),
            "a&amp;b&lt;c&gt;d&quot;e&#39;f"
        );
        // pas de double échappement : la sortie n'est pas re-scannée
        assert_eq!(encode_html("&amp;"), "&amp;amp;");
        assert_eq!(encode_html(""), "");
        assert_eq!(
            encode_html("<script>&\"'"),
            "&lt;script&gt;&amp;&quot;&#39;"
        );
        // caractères non concernés inchangés (Unicode inclus)
        assert_eq!(encode_html("héllo → 🇫🇷"), "héllo → 🇫🇷");
        assert_eq!(encode_html("&#39;"), "&amp;#39;");
    }
}
