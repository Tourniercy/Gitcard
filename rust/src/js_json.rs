//! `JSON.stringify` côté Rust (fix3/P3) : arbre de valeurs ordonné + sérialiseur
//! à l'octet près — ordre d'insertion des clés (le legacy construit ses objets
//! par littéraux, l'ordre = ordre du littéral), nombres via [`js_f64_to_string`]
//! (même algorithme que `Number::toString`, donc que `JSON.stringify`), NaN/±∞ →
//! `null`, `-0` → `0`, chaînes échappées comme V8 (`"` `\` \b \f \n \r \t et
//! `\u00xx` minuscules sous 0x20 — `<>&'` restent littéraux).
//!
//! [`JsJson::Undefined`] n'existe pas en JSON : une clé portant `Undefined` est
//! **omise** de l'objet (`JSON.stringify({a: undefined})` = `{}`) ; dans un
//! tableau elle devient `null`.

use crate::jsnum::js_f64_to_string;
use crate::jsval::number_as_f64;
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum JsJson {
    Undefined,
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<JsJson>),
    Obj(Vec<(String, JsJson)>),
}

impl JsJson {
    /// `JSON.stringify(v)`.
    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            JsJson::Undefined => out.push_str("null"), // tableau uniquement (omis dans Obj)
            JsJson::Null => out.push_str("null"),
            JsJson::Bool(true) => out.push_str("true"),
            JsJson::Bool(false) => out.push_str("false"),
            JsJson::Num(x) => {
                if x.is_nan() || x.is_infinite() {
                    out.push_str("null");
                } else {
                    out.push_str(&js_f64_to_string(*x));
                }
            }
            JsJson::Str(s) => write_json_string(out, s),
            JsJson::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            JsJson::Obj(entries) => {
                out.push('{');
                let mut first = true;
                for (k, v) in entries {
                    if matches!(v, JsJson::Undefined) {
                        continue; // clé omise, comme JSON.stringify
                    }
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    write_json_string(out, k);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    /// Conversion vers `serde_json::Value` (rendu SVG : `from_value` des cartes).
    /// Les nombres non finis, non représentables en `Number`, deviennent `Null`.
    pub fn to_value(&self) -> Value {
        match self {
            JsJson::Undefined | JsJson::Null => Value::Null,
            JsJson::Bool(b) => Value::Bool(*b),
            JsJson::Num(x) => serde_json::Number::from_f64(*x)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            JsJson::Str(s) => Value::String(s.clone()),
            JsJson::Arr(items) => Value::Array(items.iter().map(JsJson::to_value).collect()),
            JsJson::Obj(entries) => Value::Object(
                entries
                    .iter()
                    .filter(|(_, v)| !matches!(v, JsJson::Undefined))
                    .map(|(k, v)| (k.clone(), v.to_value()))
                    .collect(),
            ),
        }
    }
}

/// `JSON.stringify(string)` : échappement minimal de V8.
fn write_json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Utilitaire public : f64 depuis un `Number` serde_json (littéral verbatim,
/// ±inf comme JSON.parse V8) — réexport pratique pour les tests.
pub fn num_of(n: &serde_json::Number) -> f64 {
    number_as_f64(n)
}
