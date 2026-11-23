//! Tables Intl extraites de Node v26.13.1 (`tests/golden/intl-tables.json`,
//! régénérées par `scripts/gen-golden.mjs`) + formateur d'entiers/dates reproduisant
//! `Intl.NumberFormat(locale)`, `Intl.DisplayNames`, `Intl.DateTimeFormat('en', …)`.
//!
//! Le legacy n'appelle `formatNumber` que sur des entiers positifs (compteurs) ;
//! les dates ne passent que par le format fixe `'en'` (streak, ticks profile).

use serde_json::Value;
use std::sync::OnceLock;

const TABLES_JSON: &str = include_str!("../../tests/golden/intl-tables.json");

const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn tables() -> &'static Value {
    static TABLES: OnceLock<Value> = OnceLock::new();
    TABLES.get_or_init(|| serde_json::from_str(TABLES_JSON).expect("intl-tables.json invalide"))
}

/// Table de formatage d'une locale, telle qu'extraite de Node 26.13.1.
pub struct LocaleFormat {
    separators_right_to_left: Vec<String>,
    group_sizes_right_to_left: Vec<usize>,
    digits: Vec<String>,
    label: String,
}

impl LocaleFormat {
    /// `Intl.NumberFormat(locale).format(n)` pour un entier positif ou nul.
    pub fn format(&self, n: u64) -> String {
        let digit_str = |d: u8| -> String { self.digits[usize::from(d)].clone() };
        if n == 0 {
            return digit_str(0);
        }
        let s = n.to_string();
        let bytes = s.as_bytes();
        let primary = self.group_sizes_right_to_left[0];
        let secondary = self
            .group_sizes_right_to_left
            .get(1)
            .copied()
            .unwrap_or(primary);
        let mut chunks: Vec<&[u8]> = Vec::new();
        let mut end = bytes.len();
        let mut first = true;
        while end > 0 {
            let size = if first { primary } else { secondary };
            first = false;
            let start = end.saturating_sub(size);
            chunks.push(&bytes[start..end]);
            end = start;
        }
        chunks.reverse();
        let gaps = chunks.len() - 1;
        let mut out = String::new();
        for (i, chunk) in chunks.iter().enumerate() {
            for &b in *chunk {
                out.push_str(&self.digits[usize::from(b - b'0')]);
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
        label: l["label"].as_str().unwrap_or_default().to_string(),
    })
}

/// `Intl.NumberFormat(locale).format(n)` — entiers ≥ 0 uniquement (usage du legacy).
pub fn format_int(n: u64, code: &str) -> Option<String> {
    Some(locale_format(code)?.format(n))
}

/// `Intl.DisplayNames([code], { type: 'language' }).of(code)` (labels SUPPORTED_LOCALES).
pub fn display_name(code: &str) -> Option<String> {
    Some(locale_format(code)?.label().to_string())
}

/// `new Date("YYYY-MM-DD")` + `Intl.DateTimeFormat('en', {month:'short', day:'numeric', year:'numeric'})`.
pub fn date_en(date: &str) -> Option<String> {
    let (y, m, d) = parse_ymd(date)?;
    Some(format!("{} {}, {y}", MONTHS_EN[m - 1], d))
}

/// `new Date("YYYY-MM-DD").toLocaleString('en', { month: 'short' })` (ticks X du profile).
pub fn month_en(date: &str) -> Option<String> {
    let (_, m, _) = parse_ymd(date)?;
    Some(MONTHS_EN[m - 1].to_string())
}

fn parse_ymd(date: &str) -> Option<(&str, usize, usize)> {
    let mut it = date.split('-');
    let y = it.next()?;
    let m: usize = it.next()?.parse().ok()?;
    let d: usize = it.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_extraites_de_node_reproduites() {
        // Preuve complète : le formateur Rust reproduit TOUS les échantillons extraits
        // de Node v26.13.1 (20 locales × 5 entiers) + labels + dates + mois.
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
        }
        for (d, expected) in t["dates"].as_object().unwrap() {
            assert_eq!(date_en(d).unwrap(), expected.as_str().unwrap(), "date {d}");
        }
        for (d, expected) in t["months"].as_object().unwrap() {
            assert_eq!(month_en(d).unwrap(), expected.as_str().unwrap(), "mois {d}");
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
        assert_eq!(date_en("2026-02-25").unwrap(), "Feb 25, 2026");
        assert_eq!(month_en("2026-11-11").unwrap(), "Nov");
    }
}
