//! Primitives numériques JS (ECMA-262) reproduites pour la parité byte-à-byte.
//!
//! Deux règles extraites de Node v26.13.1 (tests/golden/num-samples.json, génération
//! par scripts/gen-golden.mjs) :
//! - `String(x)` : plus courte représentation round-trip, exponentiel si |x| ≥ 1e21
//!   ou 0 < |x| < 1e-6, `-0` → `"0"`, entiers sans partie décimale (`1745`, pas `1745.0`).
//! - `x.toFixed(f)` : arrondi au plus proche avec les égalités (« ties ») arrondies
//!   EN S'ÉLOIGNANT DE ZÉRO (`(-2.5).toFixed(0) = "-3"`, `(2.5).toFixed(0) = "3"`,
//!   `(-0.125).toFixed(2) = "-0.13"`), `-0` → sans signe, délégation à `String(x)`
//!   dès que |x| ≥ 1e21 (`(1e21).toFixed(0) = "1e+21"`).

/// Équivalent exact de `String(x)` / `${x}` en JS.
pub fn js_f64_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x == 0.0 {
        // couvre +0.0 et -0.0 (JS: String(-0) === "0")
        return "0".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-Infinity" } else { "Infinity" }.to_string();
    }
    let neg = x < 0.0;
    let a = x.abs();
    // Règle V8 (double-conversion / Grisu3, validée sur 6 020 échantillons Node
    // v26.13.1) : la plus courte représentation n'est pas toujours unique — V8 produit
    // l'arrondi correct de x à p chiffres significatifs pour le PLUS PETIT p tel que le
    // candidat se relise en x. Ni « le plus proche parmi les plus courts » (règle de
    // Rust/Dragonbox), ni une troncature de p-1 chiffres.
    for p in 1..=17u32 {
        let cand = format!("{:.*e}", (p - 1) as usize, a);
        if cand.parse::<f64>().map(|v| v == a).unwrap_or(false) {
            let (mant, exp) = cand.split_once('e').expect("forme exponentielle");
            let n = exp.parse::<i32>().expect("exposant entier") + 1;
            let digits: String = mant.chars().filter(|c| *c != '.').collect();
            let k = digits.len() as i32;
            return format_from_digits(&digits, n, k, neg);
        }
    }
    // p = 17 se relit toujours (l'arrondi correct à 17 chiffres est injectif) :
    unreachable!("aucun candidat à p chiffres ne se relit pour {x:?}");
}

/// Applique les règles de mise en forme ECMAScript (fixe vs exponentiel) à des chiffres
/// significatifs déjà déterminés.
fn format_from_digits(digits: &str, n: i32, k: i32, neg: bool) -> String {
    let body = if k <= n && n <= 21 {
        let mut s = digits.to_string();
        for _ in 0..(n - k) {
            s.push('0');
        }
        s
    } else if n > 0 && n <= 21 {
        // ici n < k (couvert au-dessus sinon) : insertion du point sans débordement
        let idx = n as usize;
        format!("{}.{}", &digits[..idx], &digits[idx..])
    } else if n > -6 && n <= 0 {
        let mut s = String::from("0.");
        for _ in 0..(-n) {
            s.push('0');
        }
        s.push_str(digits);
        s
    } else {
        let mut s = String::new();
        s.push_str(&digits[..1]);
        if k > 1 {
            s.push('.');
            s.push_str(&digits[1..]);
        }
        let e10 = n - 1;
        s.push('e');
        if e10 < 0 {
            s.push('-');
        } else {
            s.push('+');
        }
        s.push_str(&e10.abs().to_string());
        s
    };
    if neg {
        format!("-{body}")
    } else {
        body
    }
}

/// Équivalent exact de `x.toFixed(f)`.
///
/// `Err` = l'équivalent JS lèverait une RangeError (f > 100), ou f > 15 hors de la
/// borne arithmétique u128 assurée ici (le legacy n'utilise que f ∈ {0, 1, 2}).
pub fn js_to_fixed(x: f64, f: u32) -> Result<String, String> {
    if f > 100 {
        return Err("RangeError: toFixed() digits argument must be between 0 and 100".to_string());
    }
    if x.is_nan() {
        return Ok("NaN".to_string());
    }
    if x.is_infinite() {
        return Ok(if x < 0.0 { "-Infinity" } else { "Infinity" }.to_string());
    }
    if x.abs() >= 1e21 {
        return Ok(js_f64_to_string(x));
    }
    if f > 15 {
        return Err(format!("js_to_fixed: f={f} > 15 non supporté (borne u128)"));
    }
    let neg = x.is_sign_negative() && x != 0.0; // (-0.0).toFixed(1) === "0.0"
    let n = round_scaled(x.abs(), f as i32);
    let mut ds = n.to_string();
    if f > 0 {
        let want = f as usize + 1;
        while ds.len() < want {
            ds.insert(0, '0');
        }
        let split = ds.len() - f as usize;
        let (ip, fp) = ds.split_at(split);
        ds = format!("{ip}.{fp}");
    }
    Ok(if neg { format!("-{ds}") } else { ds })
}

/// n = entier le plus proche de a·10^f ; en cas d'égalité, celui dont la magnitude
/// est supérieure (comportement V8 mesuré sur Node v26.13.1, cf. module doc).
///
/// Calcul exact sans perte : a = m·2^e (m entier ≤ 2^53) donc a·10^f = m·5^f·2^(e+f)
/// ; avec a < 1e21 et f ≤ 15, m·5^f·2^(e+f) = a·10^f < 2^120 tient dans u128.
fn round_scaled(a: f64, f: i32) -> u128 {
    debug_assert!(a.is_finite() && a < 1e21);
    let bits = a.to_bits();
    let exp_field = ((bits >> 52) & 0x7ff) as i32;
    let frac = bits & ((1u64 << 52) - 1);
    let (m, e) = if exp_field == 0 {
        (frac, -1074)
    } else {
        (frac | (1u64 << 52), exp_field - 1075)
    };
    let p5 = 5u128.pow(f as u32);
    let num = m as u128 * p5; // < 2^53 · 5^15 < 2^99
    let g = e + f;
    if g >= 0 {
        // num << g = a·10^f < 2^120 : pas de débordement possible sous les gardes.
        let shifted = num.checked_shl(g as u32);
        debug_assert!(shifted.is_some());
        return shifted.unwrap_or(0);
    }
    let s = (-g) as u32;
    if s > 127 {
        // num < 2^99 < 2^(s-1) : a·10^f < 0.5 → arrondit à 0, pas de tie possible.
        return 0;
    }
    let d = 1u128 << s;
    let q = num / d;
    let r = num % d;
    q + u128::from(r * 2 >= d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_cas_limites_extraits_de_node() {
        // vérités mesurées sur node v26.13.1 (gen-golden.mjs, échantillon explicite)
        assert_eq!(js_to_fixed(42.25, 1).unwrap(), "42.3");
        assert_eq!(js_to_fixed(1.005, 2).unwrap(), "1.00");
        assert_eq!(js_to_fixed(2.675, 2).unwrap(), "2.67");
        assert_eq!(js_to_fixed(-0.04, 1).unwrap(), "-0.0");
        assert_eq!(js_to_fixed(2.5, 0).unwrap(), "3");
        assert_eq!(js_to_fixed(-2.5, 0).unwrap(), "-3");
        assert_eq!(js_to_fixed(0.5, 0).unwrap(), "1");
        assert_eq!(js_to_fixed(-0.5, 0).unwrap(), "-1");
        assert_eq!(js_to_fixed(0.125, 2).unwrap(), "0.13");
        assert_eq!(js_to_fixed(-0.125, 2).unwrap(), "-0.13");
        assert_eq!(js_to_fixed(0.15, 1).unwrap(), "0.1");
        assert_eq!(js_to_fixed(-0.15, 1).unwrap(), "-0.1");
        assert_eq!(js_to_fixed(12.25, 1).unwrap(), "12.3");
        assert_eq!(js_to_fixed(12.35, 1).unwrap(), "12.3");
        assert_eq!(js_to_fixed(27.375, 2).unwrap(), "27.38");
        assert_eq!(js_to_fixed(17.875, 2).unwrap(), "17.88");
        assert_eq!(js_to_fixed(99.99999, 2).unwrap(), "100.00");
        assert_eq!(js_to_fixed(-99.99999, 2).unwrap(), "-100.00");
    }

    #[test]
    fn to_fixed_speciaux() {
        assert_eq!(js_to_fixed(0.0, 1).unwrap(), "0.0");
        assert_eq!(js_to_fixed(-0.0, 1).unwrap(), "0.0"); // pas de signe pour -0
        assert_eq!(js_to_fixed(1745.0, 0).unwrap(), "1745");
        assert_eq!(js_to_fixed(f64::NAN, 2).unwrap(), "NaN");
        assert_eq!(js_to_fixed(f64::INFINITY, 2).unwrap(), "Infinity");
        assert_eq!(js_to_fixed(f64::NEG_INFINITY, 2).unwrap(), "-Infinity");
        assert_eq!(js_to_fixed(1e21, 0).unwrap(), "1e+21"); // délégation String
        assert_eq!(js_to_fixed(1e-7, 7).unwrap(), "0.0000001");
        assert_eq!(js_to_fixed(1e-6, 7).unwrap(), "0.0000010");
        assert_eq!(js_to_fixed(5e-324, 2).unwrap(), "0.00");
        assert_eq!(
            js_to_fixed(1.7976931348623157e308, 2).unwrap(),
            "1.7976931348623157e+308"
        );
        assert!(js_to_fixed(1.0, 101).is_err());
        assert!(js_to_fixed(1.0, 16).is_err()); // borne u128 documentée
    }

    #[test]
    fn f64_to_string_cas_limites_extraits_de_node() {
        assert_eq!(js_f64_to_string(0.0), "0");
        assert_eq!(js_f64_to_string(-0.0), "0");
        assert_eq!(js_f64_to_string(1745.0), "1745");
        assert_eq!(js_f64_to_string(f64::NAN), "NaN");
        assert_eq!(js_f64_to_string(f64::INFINITY), "Infinity");
        assert_eq!(js_f64_to_string(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(js_f64_to_string(1e21), "1e+21");
        assert_eq!(js_f64_to_string(2e21), "2e+21");
        assert_eq!(js_f64_to_string(1e-7), "1e-7");
        assert_eq!(js_f64_to_string(-1e-7), "-1e-7");
        assert_eq!(js_f64_to_string(1e-6), "0.000001");
        assert_eq!(js_f64_to_string(1e20), "100000000000000000000");
        assert_eq!(
            js_f64_to_string(123456789123456780000.0),
            "123456789123456780000"
        );
        assert_eq!(js_f64_to_string(5e-324), "5e-324");
        assert_eq!(js_f64_to_string(f64::MAX), "1.7976931348623157e+308");
        assert_eq!(js_f64_to_string(0.1), "0.1");
        assert_eq!(js_f64_to_string(0.30000000000000004), "0.30000000000000004");
        assert_eq!(js_f64_to_string(41.1764705882353), "41.1764705882353");
        assert_eq!(js_f64_to_string(99.99999999999999), "99.99999999999999");
        assert_eq!(js_f64_to_string(9007199254740992.0), "9007199254740992");
    }
}
