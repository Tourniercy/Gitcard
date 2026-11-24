//! Tables Intl extraites de Node v26.13.1 (`tests/golden/intl-tables.json`,
//! régénérées par `scripts/gen-golden.mjs`) + formateur d'entiers/dates reproduisant
//! `Intl.NumberFormat(locale)`, `Intl.DisplayNames`, `Intl.DateTimeFormat('en', …)`.
//!
//! Le legacy n'appelle `formatNumber` que sur des entiers positifs (compteurs) ;
//! les dates ne passent que par le format fixe `'en'` (streak, ticks profile).
//! Fix2 R1/R2 : ouverture aux valeurs hors pipeline (`JsVal`) — dates quelconques
//! via [`crate::jsdate`], numbers JS quelconques via [`LocaleFormat::format_f64`],
//! repli de locale vers « en » (oracle : `Intl.NumberFormat('xx')` → grouping root).

use crate::cards::RenderError;
use crate::jsdate;
use crate::jsval::JsVal;
use serde_json::Value;
use std::sync::OnceLock;

const TABLES_JSON: &str = include_str!("../../tests/golden/intl-tables.json");

pub(crate) const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn tables() -> &'static Value {
    static TABLES: OnceLock<Value> = OnceLock::new();
    TABLES.get_or_init(|| serde_json::from_str(TABLES_JSON).expect("intl-tables.json invalide"))
}

/// Table de formatage d'une locale, telle qu'extraite de Node 26.13.1.
///
/// Invariant statique (R3) : construite uniquement depuis `intl-tables.json`
/// généré par l'oracle Node — `group_sizes_right_to_left[0] ≥ 1` pour les 20
/// locales (vérifié par `tables_extraites_de_node_reproduites`) et `format`
/// est de toute façon sans boucle si `group_sizes[0] == 0` (chunk entier).
pub struct LocaleFormat {
    separators_right_to_left: Vec<String>,
    group_sizes_right_to_left: Vec<usize>,
    digits: Vec<String>,
    decimal: String,
    label: String,
    /// Signe moins ICU de la locale (fix3 : oracle `format(-5)` — U+2212 pour
    /// « sv »/« nb », tiret ASCII ailleurs).
    minus: char,
    /// Préfixe directionnel d'ICU devant toute sortie signée (fix3 : LRM U+200E
    /// pour « ar » uniquement, chaîne vide pour les 19 autres locales).
    sign_prefix: &'static str,
}

/// Signe moins et préfixe directionnel d'ICU par locale (oracle fix3 :
/// `new Intl.NumberFormat(code).format(-5)` / `.format(-Infinity)`, Node
/// v26.13.1) : « sv »/« nb » composent les négatifs avec le vrai signe moins
/// U+2212 (« \u{2212}5 », « \u{2212}∞ ») ; « ar » préfixe toute sortie signée
/// d'une marque LRM U+200E en gardant le tiret ASCII (« \u{200e}-5 »,
/// « \u{200e}-∞ ») ; les 17 autres locales : tiret ASCII, aucun préfixe. Les
/// positifs et NaN ne portent jamais de marque ni de signe (corpus ar inchangé).
fn minus_sign_data(code: &str) -> (char, &'static str) {
    match code {
        "sv" | "nb" => ('\u{2212}', ""),
        "ar" => ('-', "\u{200e}"),
        _ => ('-', ""),
    }
}

impl LocaleFormat {
    /// Regroupe une chaîne de chiffres ASCII (droite → gauche) et mappe les
    /// chiffres sur la table de la locale. Sans boucle si primary == 0 (R3).
    fn group(&self, digit_str: &str) -> String {
        let digit_of = |d: u8| -> String { self.digits[usize::from(d - b'0')].clone() };
        if digit_str == "0" {
            return digit_of(b'0');
        }
        let bytes = digit_str.as_bytes();
        let primary = self.group_sizes_right_to_left.first().copied().unwrap_or(0);
        let secondary = self
            .group_sizes_right_to_left
            .get(1)
            .copied()
            .unwrap_or(if primary == 0 { usize::MAX } else { primary });
        let mut chunks: Vec<&[u8]> = Vec::new();
        let mut end = bytes.len();
        let mut first = true;
        while end > 0 {
            let size = if first { primary } else { secondary };
            first = false;
            if size == 0 {
                // pas de groupement : un seul chunk
                chunks.push(bytes);
                break;
            }
            let start = end.saturating_sub(size);
            chunks.push(&bytes[start..end]);
            end = start;
        }
        chunks.reverse();
        let gaps = chunks.len() - 1;
        let mut out = String::new();
        for (i, chunk) in chunks.iter().enumerate() {
            for &b in *chunk {
                out.push_str(&digit_of(b));
            }
            if i < gaps {
                let gap_idx = gaps - 1 - i; // 0 = séparateur le plus à droite
                let sep = &self.separators_right_to_left
                    [gap_idx.min(self.separators_right_to_left.len() - 1)];
                out.push_str(sep);
            }
        }
        out
    }

    /// `Intl.NumberFormat(locale).format(n)` pour un entier positif ou nul.
    pub fn format(&self, n: u64) -> String {
        self.group(&n.to_string())
    }

    /// `Intl.NumberFormat(locale).format(x)` pour tout `number` JS (fix2 R2) :
    /// NaN → `"NaN"` ; ±∞ → `« ∞ »` / `« -∞ »` (I3, fix3 — ICU, signe moins
    /// ASCII) ; regroupement de la partie entière ; au plus 3 décimales
    /// (`maximumFractionDigits` par défaut d'ICU, arrondi demi-éloigné-de-zéro) ;
    /// écriture décimale complète dès 1e21 (ICU n'utilise jamais d'exposant ;
    /// oracle : `format(1e21)` → `"1,000,000,000,000,000,000,000"`).
    pub fn format_f64(&self, x: f64) -> String {
        if x.is_nan() {
            return "NaN".to_string();
        }
        // I3 (fix3) : sans cette garde, a ≥ 1e21 formerait « inf » et `group`
        // lirait `digits[57]` hors bornes (panic). Oracle : Intl.format(±∞)
        // → « ∞ » / « -∞ » ; carte stats `totalStars: 1e400` → rendu.
        if x.is_infinite() {
            return if x > 0.0 {
                "∞".to_string()
            } else {
                format!("{}{}∞", self.sign_prefix, self.minus)
            };
        }
        // M1 (fix3) : -0 → « -0 » (oracle `Intl.NumberFormat('en').format(-0)`)
        // ; l'ancien commentaire s'appuyait sur String(-0) = « 0 » — il est faux.
        let neg = x < 0.0 || (x == 0.0 && x.is_sign_negative());
        let a = x.abs();
        let (int_part, frac) = if a >= 1e21 {
            // expansion décimale exacte du f64 (Rust imprime la valeur binaire exacte)
            (format!("{a:.0}"), String::new())
        } else {
            let scaled = (a * 1000.0).round() as u128; // maxFractionDigits = 3, half-expand
            let frac_v = (scaled % 1000) as u32;
            let f = if frac_v == 0 {
                String::new()
            } else {
                format!("{frac_v:03}").trim_end_matches('0').to_string()
            };
            ((scaled / 1000).to_string(), f)
        };
        let mut out = self.group(&int_part);
        if !frac.is_empty() {
            out.push_str(&self.decimal);
            for c in frac.chars() {
                out.push_str(&self.digits[usize::from(c as u8 - b'0')]);
            }
        }
        if neg {
            format!("{}{}{out}", self.sign_prefix, self.minus)
        } else {
            out
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Codes de locales acceptés par le legacy (`LOCALE_CODES`, 20 entrées).
pub fn locale_codes() -> Vec<String> {
    tables()["locales"]
        .as_object()
        .expect("locales")
        .keys()
        .cloned()
        .collect()
}

pub fn locale_format(code: &str) -> Option<LocaleFormat> {
    let l = tables()["locales"].get(code)?;
    Some(LocaleFormat {
        separators_right_to_left: l["separatorRightToLeft"]
            .as_array()?
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect(),
        group_sizes_right_to_left: l["groupSizesRightToLeft"]
            .as_array()?
            .iter()
            .map(|v| v.as_u64().unwrap_or(3) as usize)
            .collect(),
        digits: l["digits"]
            .as_array()?
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect(),
        decimal: l["decimal"].as_str().unwrap_or(".").to_string(),
        label: l["label"].as_str().unwrap_or_default().to_string(),
        minus: minus_sign_data(code).0,
        sign_prefix: minus_sign_data(code).1,
    })
}

/// `Intl.NumberFormat(locale).format(n)` — entiers ≥ 0 uniquement (usage du legacy).
pub fn format_int(n: u64, code: &str) -> Option<String> {
    Some(locale_format(code)?.format(n))
}

/// `formatNumber(v, locale)` pour toute valeur (fix2 R2) : `ToNumber` JS puis
/// format ICU. Locale hors table (impossible via `parseCardOptions` qui impose
/// LOCALE_CODES) : résolution vers la locale par défaut du runtime = « en »
/// (oracle : `Intl.NumberFormat('xx').format(1234567)` → `"1,234,567"`).
pub fn format_js_number(n: f64, code: &str) -> String {
    let lf = locale_format(code)
        .or_else(|| locale_format("en"))
        .expect("table 'en' présente dans intl-tables.json");
    lf.format_f64(n)
}

/// `Intl.DisplayNames([code], { type: 'language' }).of(code)` (labels SUPPORTED_LOCALES).
pub fn display_name(code: &str) -> Option<String> {
    Some(locale_format(code)?.label().to_string())
}

/// `formatDateRange` du streak : `Intl.DateTimeFormat('en', …).format(new Date(v))`
/// pour toute valeur ; `Err(RangeError)` = date invalide (le legacy lève
/// « Invalid time value » — fix2 R1, plus aucun panic).
pub fn date_en_js(v: &JsVal) -> Result<String, RenderError> {
    match jsdate::new_date(v) {
        jsdate::JsDate::Valid(ms) => Ok(jsdate::format_date_en(ms)),
        jsdate::JsDate::Invalid => Err(RenderError::RangeError("Invalid time value".to_string())),
    }
}

/// Tick X du profile : `new Date(label).toLocaleString('en', { month: 'short' })`.
/// ORACLE (fix2 R1) : contrairement à `Intl.DateTimeFormat().format`, la méthode
/// `toLocaleString` NE LÈVE PAS sur date invalide — elle rend la string
/// « Invalid Date » (ECMA-402 : si x est NaN, renvoyer « Invalid Date »).
pub fn month_en_tick(v: &JsVal) -> String {
    match jsdate::new_date(v) {
        jsdate::JsDate::Valid(ms) => jsdate::format_month_en(ms),
        jsdate::JsDate::Invalid => "Invalid Date".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_extraites_de_node_reproduites() {
        // Preuve complète : le formateur Rust reproduit TOUS les échantillons extraits
        // de Node v26.13.1 (20 locales × 5 entiers) + labels + dates + mois + repli.
        let t = tables();
        let mut checked = 0u32;
        for (code, l) in t["locales"].as_object().unwrap() {
            for (n_str, expected) in l["number"].as_object().unwrap() {
                let n: u64 = n_str.parse().expect("échantillon entier");
                assert_eq!(
                    format_int(n, code).unwrap(),
                    expected.as_str().unwrap(),
                    "locale {code} entier {n}"
                );
                checked += 1;
            }
            assert_eq!(display_name(code).unwrap(), l["label"].as_str().unwrap());
            // invariant R3 : primary ≥ 1 → pas de boucle possible dans group()
            assert!(
                locale_format(code).unwrap().group_sizes_right_to_left[0] >= 1,
                "locale {code} : regroupement primaire nul"
            );
        }
        for (d, expected) in t["dates"].as_object().unwrap() {
            let ms = jsdate::parse_string(d).unwrap_or_else(|| panic!("date oracle {d}"));
            assert_eq!(
                jsdate::format_date_en(ms),
                expected.as_str().unwrap(),
                "date {d}"
            );
        }
        for (d, expected) in t["months"].as_object().unwrap() {
            let ms = jsdate::parse_string(d).unwrap_or_else(|| panic!("date oracle {d}"));
            assert_eq!(
                jsdate::format_month_en(ms),
                expected.as_str().unwrap(),
                "mois {d}"
            );
        }
        // R4 : repli de locale = locale par défaut du runtime (oracle Node).
        for (bad, expected) in t["localeFallback"].as_object().unwrap() {
            assert_eq!(
                format_js_number(1_234_567.0, bad),
                expected.as_str().unwrap(),
                "repli locale {bad}"
            );
        }
        assert_eq!(t["localeCount"].as_u64().unwrap(), 20);
        assert_eq!(locale_codes().len(), 20);
        println!("intl: {checked} échantillons numériques + 20 labels + dates/mois vérifiés");
    }

    #[test]
    fn exemples_ancres() {
        assert_eq!(format_int(1234567, "en").unwrap(), "1,234,567");
        assert_eq!(
            format_int(1234567, "fr").unwrap(),
            "1\u{202f}234\u{202f}567"
        ); // U+202F
        assert_eq!(format_int(1234567, "ru").unwrap(), "1\u{a0}234\u{a0}567"); // U+00A0
        assert_eq!(format_int(1234567, "de").unwrap(), "1.234.567");
        assert_eq!(format_int(1234567, "hi").unwrap(), "12,34,567"); // regroupement lakhs
        assert_eq!(format_int(123456789, "hi").unwrap(), "12,34,56,789");
        assert_eq!(format_int(0, "en").unwrap(), "0");
        assert_eq!(
            date_en_js(&JsVal::Str("2026-02-25".into())).unwrap(),
            "Feb 25, 2026"
        );
        assert_eq!(month_en_tick(&JsVal::Str("2026-11-11".into())), "Nov");
        // oracle : toLocaleString sur invalide → « Invalid Date », sans lever
        assert_eq!(month_en_tick(&JsVal::Str("garbage".into())), "Invalid Date");
        assert_eq!(month_en_tick(&JsVal::Str("".into())), "Invalid Date");
    }

    #[test]
    fn format_f64_matrice_oracle_r2() {
        // verdicts Node v26.13.1 (sonde fix2) — Intl.NumberFormat('en')
        let en = locale_format("en").unwrap();
        assert_eq!(en.format_f64(f64::NAN), "NaN"); // undefined → NaN
        assert_eq!(en.format_f64(0.0), "0"); // null → 0
        assert_eq!(en.format_f64(-5.0), "-5");
        assert_eq!(en.format_f64(12.5), "12.5");
        assert_eq!(en.format_f64(9007199254740993.0), "9,007,199,254,740,992");
        assert_eq!(en.format_f64(1e21), "1,000,000,000,000,000,000,000");
        assert_eq!(en.format_f64(42.0), "42"); // "42" → ToNumber
        assert_eq!(en.format_f64(1.0), "1"); // true → 1
    }
}
