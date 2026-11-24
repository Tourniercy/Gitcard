//! Valeur JSON vue avec la sémantique de V8 (fix2 R2) : un champ de fixture peut
//! être absent (`undefined`), `null`, un booléen, un `number` (f64) ou une `string`.
//! Le legacy n'a aucun typage à l'exécution : `formatNumber(null)` → `"0"`,
//! `${currentStreak}` sur `1e21` → `"1e+21"`, `0 + "42"` → `"042"`, etc.
//! Oracle : `tests/golden/meta/counters-extract.json` + `date-extract.json`.

use serde_json::Value;

/// Champ JSON arbitraire tel que le voit le JS du legacy.
#[derive(Debug, Clone, PartialEq)]
pub enum JsVal {
    /// Champ absent (`undefined`).
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    /// Valeur objet/tableau JSON (hors oracles) : truthy, `ToNumber` → NaN,
    /// `ToString` → "[object Object]" (approximation suffisante, jamais rendue).
    Other,
}

impl JsVal {
    /// `v.get(k)` JSON : clé absente → `Undefined`.
    pub fn field(v: &Value, k: &str) -> JsVal {
        match v.get(k) {
            None => JsVal::Undefined,
            Some(val) => JsVal::from_json(val),
        }
    }

    pub fn from_json(v: &Value) -> JsVal {
        match v {
            Value::Null => JsVal::Null,
            Value::Bool(b) => JsVal::Bool(*b),
            Value::Number(n) => JsVal::Number(number_as_f64(n)),
            Value::String(s) => JsVal::Str(s.clone()),
            Value::Array(_) | Value::Object(_) => JsVal::Other,
        }
    }

    /// `typeof` approximatif pour les messages d'erreur (null/undefined/number/…).
    pub fn type_name(&self) -> &'static str {
        match self {
            JsVal::Undefined => "undefined",
            JsVal::Null => "null",
            JsVal::Bool(_) => "boolean",
            JsVal::Number(_) => "number",
            JsVal::Str(_) => "string",
            JsVal::Other => "object",
        }
    }
}

/// `Number` serde_json → f64, sémantique `JSON.parse` de V8 (fix3 I3) :
/// `arbitrary_precision` conserve le littéral source (« 1e400 » reste une
/// chaîne) et `as_f64` rend alors `None` — V8, lui, rend ±Infinity. Re-parsé
/// via la forme sérialisée (le littéral verbatim) : « 1e400 » → +inf,
/// « -1e400 » → -inf, sous-normales et zéros signés inchangés.
fn number_as_f64(n: &serde_json::Number) -> f64 {
    match n.as_f64() {
        Some(f) => f,
        None => serde_json::to_string(n)
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(f64::NAN),
    }
}

/// Vérité JS (`!x`) : `''`, `0`, `NaN`, `null`, `undefined`, `false` sont falsy.
pub fn truthy(v: &JsVal) -> bool {
    match v {
        JsVal::Undefined | JsVal::Null => false,
        JsVal::Bool(b) => *b,
        JsVal::Number(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Other => true,
    }
}

/// `ToNumber` JS pour les primitives JSON.
pub fn to_number(v: &JsVal) -> f64 {
    match v {
        JsVal::Undefined => f64::NAN,
        JsVal::Null => 0.0,
        JsVal::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        JsVal::Number(n) => *n,
        JsVal::Str(s) => str_to_number(s),
        JsVal::Other => f64::NAN,
    }
}

/// `ToString` JS (`${x}` / `String(x)`) pour les primitives JSON.
pub fn js_to_string(v: &JsVal) -> String {
    match v {
        JsVal::Undefined => "undefined".to_string(),
        JsVal::Null => "null".to_string(),
        JsVal::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        JsVal::Number(n) => crate::jsnum::js_f64_to_string(*n),
        JsVal::Str(s) => s.clone(),
        JsVal::Other => "[object Object]".to_string(),
    }
}

/// `+` JS sur primitives : concaténation si l'un des deux est une string.
pub fn js_add(a: &JsVal, b: &JsVal) -> JsVal {
    let a_str = matches!(a, JsVal::Str(_) | JsVal::Other);
    let b_str = matches!(b, JsVal::Str(_) | JsVal::Other);
    if a_str || b_str {
        JsVal::Str(format!("{}{}", js_to_string(a), js_to_string(b)))
    } else {
        JsVal::Number(to_number(a) + to_number(b))
    }
}

/// `StringToNumber` JS : trim d'espaces JS, `""` → 0, `Infinity` signé,
/// littéraux 0x/0o/0b, grammaire décimale stricte, sinon NaN.
pub fn str_to_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| {
        matches!(
            c,
            ' ' | '\t' | '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{a0}' | '\u{feff}'
        )
    });
    if t.is_empty() {
        return 0.0;
    }
    let (sign, rest) = match t.as_bytes().first() {
        Some(b'+') => (1.0, &t[1..]),
        Some(b'-') => (-1.0, &t[1..]),
        _ => (1.0, t),
    };
    if rest == "Infinity" {
        return sign * f64::INFINITY;
    }
    if rest.len() > 2 && rest.starts_with('0') {
        let radix = match &rest[1..2] {
            "x" | "X" => Some(16u32),
            "o" | "O" => Some(8),
            "b" | "B" => Some(2),
            _ => None,
        };
        if let Some(radix) = radix {
            let mut acc = 0.0f64;
            for c in rest[2..].chars() {
                let d = c.to_digit(radix);
                match d {
                    Some(d) => acc = acc * radix as f64 + f64::from(d),
                    None => return f64::NAN, // digit invalide → NaN (même partiel)
                }
            }
            return sign * acc;
        }
    }
    if is_js_decimal(rest) {
        sign * rest.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// Grammaire décimale de `StringToNumber` : `DecimalDigits [. DecimalDigits] [e±d]` etc.
fn is_js_decimal(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    let mut int_len = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        int_len += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let mut frac_len = 0;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            frac_len += 1;
        }
        if int_len == 0 && frac_len == 0 {
            return false;
        }
    } else if int_len == 0 {
        return false;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let mut exp_len = 0;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            exp_len += 1;
        }
        if exp_len == 0 {
            return false;
        }
    }
    i == b.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn verite_et_tostring() {
        assert!(!truthy(&JsVal::Undefined) && !truthy(&JsVal::Null));
        assert!(!truthy(&JsVal::Str(String::new())));
        assert!(!truthy(&JsVal::Number(0.0)));
        assert!(!truthy(&JsVal::Number(f64::NAN)));
        assert!(truthy(&JsVal::Number(-5.0)));
        assert!(truthy(&JsVal::Str("0".into()))); // "0" non vide → truthy
        assert_eq!(js_to_string(&JsVal::Undefined), "undefined");
        assert_eq!(js_to_string(&JsVal::Null), "null");
        assert_eq!(js_to_string(&JsVal::Bool(true)), "true");
        assert_eq!(js_to_string(&JsVal::Number(1e21)), "1e+21");
        assert_eq!(
            js_to_string(&JsVal::Number(9007199254740993.0)),
            "9007199254740992"
        );
    }

    #[test]
    fn to_number_semantique_js() {
        assert!(to_number(&JsVal::Undefined).is_nan());
        assert_eq!(to_number(&JsVal::Null), 0.0);
        assert_eq!(to_number(&JsVal::Bool(true)), 1.0);
        assert_eq!(to_number(&JsVal::Str("42".into())), 42.0);
        assert_eq!(to_number(&JsVal::Str("  12.5 ".into())), 12.5);
        assert_eq!(to_number(&JsVal::Str("".into())), 0.0);
        assert_eq!(to_number(&JsVal::Str("0x10".into())), 16.0);
        assert_eq!(
            to_number(&JsVal::Str("-Infinity".into())),
            f64::NEG_INFINITY
        );
        assert!(to_number(&JsVal::Str("garbage".into())).is_nan());
        assert!(to_number(&JsVal::Str("1 2".into())).is_nan());
        assert!(to_number(&JsVal::Str("INF".into())).is_nan()); // casse sensible
        assert!(to_number(&JsVal::Str("0x".into())).is_nan());
        assert_eq!(to_number(&JsVal::Str("1e3".into())), 1000.0);
    }

    #[test]
    fn addition_js_concatene_avec_string() {
        assert_eq!(
            js_add(&JsVal::Number(0.0), &JsVal::Str("42".into())),
            JsVal::Str("042".into())
        );
        assert_eq!(
            js_add(&JsVal::Number(1.0), &JsVal::Null),
            JsVal::Number(1.0)
        );
        assert!(matches!(
            js_add(&JsVal::Number(1.0), &JsVal::Undefined),
            JsVal::Number(n) if n.is_nan()
        ));
        assert_eq!(
            js_add(&JsVal::Number(2.0), &JsVal::Bool(true)),
            JsVal::Number(3.0)
        );
    }

    #[test]
    fn champ_absent_vs_null() {
        let v = json!({"a": null});
        assert_eq!(JsVal::field(&v, "a"), JsVal::Null);
        assert_eq!(JsVal::field(&v, "b"), JsVal::Undefined);
    }
}
