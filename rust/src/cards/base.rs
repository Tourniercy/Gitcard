//! Port littéral de `packages/svg-renderer/src/cards/base-card.ts` (162 lignes) :
//! `octiconSvg`, `formatNumber`, `createArcPath`, `createCardWrapper`,
//! `createRingChart`, `createDonutSegment` (+ `createDonutArcPath`,
//! `polarToCartesian` privés).
//!
//! Conditions de parité :
//! - C1 : `fdlibm::cos/sin` partout (jamais `f64::cos/sin`, verrouillé par clippy.toml) ;
//! - C2 : toute valeur f64 interpolée passe par `js_f64_to_string` (= `${x}` JS).

use crate::fdlibm;
use crate::intl;
use crate::jsnum::{js_f64_to_string, js_to_fixed};
use crate::octicons;
use crate::themes::Theme;
use crate::types::CardOptions;

pub use crate::sanitize::encode_html;

pub const FONT_FAMILY: &str = "'Segoe UI', system-ui, -apple-system, sans-serif";

/// `Math.PI` — même double en Rust et JS (IEEE 754).
pub const PI: f64 = std::f64::consts::PI;

// ------------------------------------------------------- compteur global d'uid

/// `let cardIdCounter = 0` (base-card.ts, module-global) — UN `createCardWrapper`
/// par rendu consomme exactement une valeur (`const uid = cardIdCounter++`).
/// Le test de corpus règle le compteur sur `entry.uid` avant chaque rendu
/// (cf. `uidRule` du manifeste golden).
static CARD_ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Règle le compteur global d'uid (équivalent `cardIdCounter = n` avant un rendu).
pub fn set_card_id_counter(n: u64) {
    CARD_ID_COUNTER.store(n, std::sync::atomic::Ordering::SeqCst);
}

fn next_card_id() -> u64 {
    CARD_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

// ------------------------------------------------------------------ helpers JS

/// Équivalent de la véracité JS d'une `string` d'option : `''` est falsy.
pub(crate) fn pick(v: &Option<String>) -> Option<&str> {
    v.as_deref().filter(|s| !s.is_empty())
}

/// `String(x)` / `${x}` JS pour toute valeur numérique interpolée (C2).
pub(crate) fn js(x: f64) -> String {
    js_f64_to_string(x)
}

/// `octiconSvg(name, x, y, size, fill)` — variante 16 px de `@primer/octicons`.
pub fn octicon_svg(name: &str, x: f64, y: f64, size: u32, fill: &str) -> String {
    let (width, path) =
        octicons::icon_16(name).unwrap_or_else(|| panic!("octicon manquant: {name}"));
    format!(
        "<svg x=\"{}\" y=\"{}\" width=\"{size}\" height=\"{size}\" viewBox=\"0 0 {size} {width}\" fill=\"{fill}\">{path}</svg>",
        js(x),
        js(y)
    )
}

/// `formatNumber(n, locale)` = `new Intl.NumberFormat(locale).format(n)`
/// (entiers ≥ 0 uniquement dans le legacy ; tables C3 extraites de Node).
pub fn format_number(n: u64, locale: &str) -> String {
    intl::format_int(n, locale).unwrap_or_else(|| js_f64_to_string(n as f64))
}

/// `createArcPath` — exporté par le legacy mais inutilisé par les cartes ;
/// porté pour parité de surface publique (vérifié contre l'extraction legacy).
pub fn create_arc_path(cx: f64, cy: f64, radius: f64, start_angle: f64, end_angle: f64) -> String {
    let x1 = cx + radius * fdlibm::cos(start_angle);
    let y1 = cy + radius * fdlibm::sin(start_angle);
    let x2 = cx + radius * fdlibm::cos(end_angle);
    let y2 = cy + radius * fdlibm::sin(end_angle);
    let large_arc = if end_angle - start_angle > PI { 1 } else { 0 };
    format!(
        "M {} {} A {} {} 0 {large_arc} 1 {} {}",
        js(x1),
        js(y1),
        js(radius),
        js(radius),
        js(x2),
        js(y2)
    )
}

/// `createCardWrapper` — consomme exactement un uid (compteur global).
pub fn create_card_wrapper(
    width: f64,
    height: f64,
    content: &str,
    theme: &Theme,
    options: &CardOptions,
) -> String {
    // options.bgColor ? `#${bgColor}` : theme.background ( '' → falsy JS )
    let bg = match pick(&options.bg_color) {
        Some(c) => format!("#{c}"),
        None => theme.background.to_string(),
    };
    let border_color = match pick(&options.border_color) {
        Some(c) => format!("#{c}"),
        None => theme.border.to_string(),
    };
    let title_fill = match pick(&options.title_color) {
        Some(c) => format!("#{c}"),
        None => theme.title.to_string(),
    };
    let label_fill = match pick(&options.text_color) {
        Some(c) => format!("#{c}"),
        None => theme.muted.to_string(),
    };
    let value_fill = match pick(&options.text_color) {
        Some(c) => format!("#{c}"),
        None => theme.text.to_string(),
    };
    let show_border = !options.hide_border;
    let uid = next_card_id();

    let mut s = String::with_capacity(2048 + content.len());
    s.push_str("\n<svg width=\"");
    s.push_str(&js(width));
    s.push_str("\" height=\"");
    s.push_str(&js(height));
    s.push_str("\" viewBox=\"0 0 ");
    s.push_str(&js(width));
    s.push(' ');
    s.push_str(&js(height));
    s.push_str(
        "\"\n     xmlns=\"http://www.w3.org/2000/svg\">\n  <defs>\n    <clipPath id=\"clip-",
    );
    s.push_str(&uid.to_string());
    s.push_str("\">\n      <rect width=\"");
    s.push_str(&js(width));
    s.push_str("\" height=\"");
    s.push_str(&js(height));
    s.push_str(
        "\" rx=\"8\" ry=\"8\" />\n    </clipPath>\n  </defs>\n\n  <style>\n    * { font-family: ",
    );
    s.push_str(FONT_FAMILY);
    s.push_str("; }\n    .title { font-size: 14px; font-weight: 600; fill: ");
    s.push_str(&title_fill);
    s.push_str("; }\n    .stat-label { font-size: 12px; fill: ");
    s.push_str(&label_fill);
    s.push_str("; }\n    .stat-value { font-size: 14px; font-weight: 700; fill: ");
    s.push_str(&value_fill);
    s.push_str("; }\n    .muted { font-size: 11px; fill: ");
    s.push_str(theme.muted);
    s.push_str("; }\n  </style>\n\n  <g clip-path=\"url(#clip-");
    s.push_str(&uid.to_string());
    s.push_str(")\">\n    <rect width=\"");
    s.push_str(&js(width));
    s.push_str("\" height=\"");
    s.push_str(&js(height));
    s.push_str("\" rx=\"8\" ry=\"8\"\n          fill=\"");
    s.push_str(&bg);
    s.push_str("\" />\n    ");
    if show_border {
        s.push_str("<rect width=\"");
        s.push_str(&js(width));
        s.push_str("\" height=\"");
        s.push_str(&js(height));
        s.push_str("\" rx=\"8\" ry=\"8\" fill=\"none\" stroke=\"");
        s.push_str(&border_color);
        s.push_str("\" stroke-width=\"1\" />");
    }
    s.push_str("\n\n    ");
    s.push_str(content);
    s.push_str("\n  </g>\n</svg>");
    s.trim().to_string()
}

/// `createRingChart` — anneau de progression + note centrale.
pub fn create_ring_chart(
    cx: f64,
    cy: f64,
    radius: f64,
    percentage: f64,
    color: &str,
    bg_color: &str,
    label: &str,
) -> String {
    let circumference = 2.0 * PI * radius;
    let offset = circumference * (1.0 - percentage / 100.0);
    format!(
        "\n    <circle cx=\"{}\" cy=\"{}\" r=\"{}\"\n            fill=\"none\" stroke=\"{bg_color}\" stroke-width=\"6\" opacity=\"0.2\" />\n    <circle cx=\"{}\" cy=\"{}\" r=\"{}\"\n            fill=\"none\" stroke=\"{color}\" stroke-width=\"6\"\n            stroke-dasharray=\"{}\"\n            stroke-dashoffset=\"{}\"\n            stroke-linecap=\"round\"\n            transform=\"rotate(-90 {} {})\"\n            />\n    <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" dominant-baseline=\"central\"\n          font-size=\"18\" font-weight=\"700\" fill=\"{color}\">\n      {}\n    </text>\n  ",
        js(cx),
        js(cy),
        js(radius),
        js(cx),
        js(cy),
        js(radius),
        js(circumference),
        js(offset),
        js(cx),
        js(cy),
        js(cx),
        js(cy),
        encode_html(label)
    )
}

/// `createDonutSegment` — segment d'anneau (langs).
pub fn create_donut_segment(
    cx: f64,
    cy: f64,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    color: &str,
) -> String {
    let inner_radius = radius * 0.6;
    let d = create_donut_arc_path(cx, cy, radius, inner_radius, start_angle, end_angle);
    format!("<path d=\"{d}\" fill=\"{color}\" />")
}

fn create_donut_arc_path(
    cx: f64,
    cy: f64,
    outer_r: f64,
    inner_r: f64,
    start_angle: f64,
    end_angle: f64,
) -> String {
    let start_outer = polar_to_cartesian(cx, cy, outer_r, start_angle);
    let end_outer = polar_to_cartesian(cx, cy, outer_r, end_angle);
    let start_inner = polar_to_cartesian(cx, cy, inner_r, end_angle);
    let end_inner = polar_to_cartesian(cx, cy, inner_r, start_angle);
    let large_arc = if end_angle - start_angle > PI { 1 } else { 0 };
    [
        format!("M {} {}", js(start_outer.0), js(start_outer.1)),
        format!(
            "A {} {} 0 {large_arc} 1 {} {}",
            js(outer_r),
            js(outer_r),
            js(end_outer.0),
            js(end_outer.1)
        ),
        format!("L {} {}", js(start_inner.0), js(start_inner.1)),
        format!(
            "A {} {} 0 {large_arc} 0 {} {}",
            js(inner_r),
            js(inner_r),
            js(end_inner.0),
            js(end_inner.1)
        ),
        "Z".to_string(),
    ]
    .join(" ")
}

/// `polarToCartesian` — C1 : réduction d'angle identique au legacy
/// (`Math.cos(angle - Math.PI / 2)`), via fdlibm.
fn polar_to_cartesian(cx: f64, cy: f64, radius: f64, angle: f64) -> (f64, f64) {
    (
        cx + radius * fdlibm::cos(angle - PI / 2.0),
        cy + radius * fdlibm::sin(angle - PI / 2.0),
    )
}

/// `js_to_fixed` sur usage legacy borné (f ∈ {0, 1, 2}) — panic avec message
/// (équivalent RangeError JS, inatteignable dans le corpus).
pub(crate) fn to_fixed(x: f64, f: u32) -> String {
    js_to_fixed(x, f).unwrap_or_else(|e| panic!("toFixed({x}, {f}): {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_tables_c3() {
        // 5 entiers × 20 locales de la table (extraite de Node v26.13.1)
        let samples = [0u64, 999, 1234567, 1500000, 123456789];
        let mut checked = 0u32;
        for code in crate::intl::locale_codes() {
            for n in samples {
                let got = format_number(n, &code);
                let want = crate::intl::format_int(n, &code).expect("format table");
                assert_eq!(got, want, "locale {code} entier {n}");
                checked += 1;
            }
        }
        assert_eq!(checked, 100);
        assert_eq!(format_number(1234567, "en"), "1,234,567");
        assert_eq!(format_number(1234567, "fr"), "1\u{202f}234\u{202f}567");
        assert_eq!(format_number(123456789, "hi"), "12,34,56,789");
    }

    #[test]
    fn compteur_uid_consomme_un_par_wrapper() {
        set_card_id_counter(41);
        let theme = crate::themes::get_theme("default");
        let options = crate::types::CardOptions {
            theme: "default".into(),
            hide: vec![],
            show_icons: true,
            hide_border: false,
            hide_title: false,
            bg_color: None,
            title_color: None,
            text_color: None,
            icon_color: None,
            border_color: None,
            cache_seconds: 14400,
            locale: "en".into(),
            layout: None,
        };
        let svg = create_card_wrapper(10.0, 5.0, "X", theme, &options);
        assert!(svg.contains("clip-41"), "uid=41 consommé : {svg}");
        let svg2 = create_card_wrapper(10.0, 5.0, "X", theme, &options);
        assert!(svg2.contains("clip-42"));
        assert!(svg.starts_with("<svg"), "trim() appliqué");
        set_card_id_counter(0);
    }
}
