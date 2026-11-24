//! Port littéral de `packages/svg-renderer/src/cards/profile-card.ts` (211 lignes) :
//! agrégation hebdo, interpolation cubique monotone (Fritsch-Carlson), `accountAge`
//! avec horloge injectable (corpus figé au 2026-11-24 UTC).
//!
//! Fix2 :
//! - R1 : ticks X via `crate::jsdate` — `Err(RangeError)` au lieu de `panic!` sur
//!   label de tranche invalide (le legacy lève « Invalid time value ») ;
//! - R2 : compteurs `JsVal` — somme `0 + count` avec concaténation string (oracle :
//!   `0 + "42"` → `"042"`), `Math.max` absorbant NaN, `email` truthy puis
//!   `encodeHTML` (TypeError capturable si non-string) ;
//! - R5 : `js_round` = `Math.round` exact (0,49999999999999994 → 0, -0,5 → -0).

use super::base::{create_card_wrapper, format_number, js, octicon_svg, pick};
use crate::cards::RenderError;
use crate::intl;
use crate::jsdate;
use crate::jsval::{self, JsVal};
use crate::sanitize::encode_html_js;
use crate::themes::get_theme;
use crate::types::{CardOptions, ContributionDay, ProfileData};
use std::sync::Mutex;

const CARD_WIDTH: f64 = 700.0;
const CARD_HEIGHT: f64 = 200.0;

// Bornes de la zone de graphique
const CHART_LEFT: f64 = 320.0;
const CHART_RIGHT: f64 = 670.0;
const CHART_TOP: f64 = 45.0;
const CHART_BOTTOM: f64 = 155.0;

// ------------------------------------------------------------ horloge injectable

/// `new Date()` d'`accountAge`, injectable : (année, mois 1-12) en UTC.
/// Défaut : horloge système UTC. Le corpus golden est figé au 2026-11-24.
static CLOCK: Mutex<Option<(i64, u32)>> = Mutex::new(None);

/// Injecte l'horloge (équivalent `now = new Date(...)`) — utilisé par les tests.
pub fn set_clock(year: i64, month: u32) {
    *lock() = Some((year, month));
}

/// Restaure l'horloge système UTC.
pub fn clear_clock() {
    *lock() = None;
}

fn lock() -> std::sync::MutexGuard<'static, Option<(i64, u32)>> {
    // pas de panic en cas de lock empoisonné (donnée de test uniquement)
    CLOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn now_ym() -> (i64, u32) {
    if let Some(ym) = *lock() {
        return ym;
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, _) = jsdate::civil_from_days_full(secs.div_euclid(86_400));
    (y, m)
}

// ------------------------------------------------------------------- agrégation

struct WeekBucket {
    label: JsVal,
    count: JsVal,
}

/// `aggregateWeekly` : tranches de 7 jours, label = jour médian (index 3 sur 7),
/// `mid?.date ?? ''` (null/absent → ''), total = `reduce((s, d) => s + d.count, 0)`
/// avec la sémantique `+` JS (oracle : `0 + "42"` → `"042"`, `0 + null` → 0,
/// `0 + undefined` → NaN).
fn aggregate_weekly(days: &[ContributionDay]) -> Vec<WeekBucket> {
    let mut buckets: Vec<WeekBucket> = Vec::new();
    let mut i = 0;
    while i < days.len() {
        let week = &days[i..(i + 7).min(days.len())];
        let mut total = JsVal::Number(0.0);
        for d in week {
            total = jsval::js_add(&total, &d.count);
        }
        let mid = &week[week.len() / 2];
        let label = match &mid.date {
            JsVal::Undefined | JsVal::Null => JsVal::Str(String::new()),
            other => other.clone(),
        };
        buckets.push(WeekBucket {
            label,
            count: total,
        });
        i += 7;
    }
    buckets
}

/// Interpolation cubique monotone (Fritsch-Carlson) → path SVG.
fn monotone_cubic_path(points: &[(f64, f64)]) -> String {
    let n = points.len();
    if n < 2 {
        return String::new();
    }
    if n == 2 {
        return format!("L{},{}", js(points[1].0), js(points[1].1));
    }

    // 1) pentes des sécantes
    let mut deltas: Vec<f64> = Vec::new();
    let mut slopes: Vec<f64> = Vec::new();
    for i in 0..n - 1 {
        let dx = points[i + 1].0 - points[i].0;
        let dy = points[i + 1].1 - points[i].1;
        deltas.push(dx);
        slopes.push(dy / dx);
    }

    // 2) tangentes Fritsch-Carlson
    let mut tangents: Vec<f64> = vec![slopes[0]];
    for i in 1..n - 1 {
        let s0 = slopes[i - 1];
        let s1 = slopes[i];
        if s0 * s1 <= 0.0 {
            tangents.push(0.0);
        } else {
            tangents.push(
                (3.0 * (deltas[i - 1] + deltas[i]))
                    / ((2.0 * deltas[i] + deltas[i - 1]) / s0
                        + (deltas[i] + 2.0 * deltas[i - 1]) / s1),
            );
        }
    }
    tangents.push(slopes[n - 2]);

    // 3) segments de Bézier cubiques
    let mut parts: Vec<String> = Vec::new();
    for i in 0..n - 1 {
        let dx = deltas[i] / 3.0;
        let cp1x = points[i].0 + dx;
        let cp1y = points[i].1 + tangents[i] * dx;
        let cp2x = points[i + 1].0 - dx;
        let cp2y = points[i + 1].1 - tangents[i + 1] * dx;
        parts.push(format!(
            "C{},{} {},{} {},{}",
            js(cp1x),
            js(cp1y),
            js(cp2x),
            js(cp2y),
            js(points[i + 1].0),
            js(points[i + 1].1)
        ));
    }
    parts.join(" ")
}

/// `Math.round` JS exact (fix2 R5) : plus proche entier, moitiés vers +∞.
/// La formule naïve `(x + 0.5).floor()` diverge pour x = 0,49999999999999994
/// (V8 → 0, naïf → 1) — corrigé par la garde `r - x > 0.5`.
fn js_round(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() || x == 0.0 {
        return x;
    }
    let r = (x + 0.5).floor();
    if r == 0.0 && x < 0.0 {
        return -0.0; // Math.round(-0.4) === -0, Math.round(-0.5) === -0
    }
    // garde exacte : (x + 0.5) a pu être arrondi VERS LE HAUT au-delà du milieu
    // (0,49999999999999994 → somme = 1.0) — les deux comparaisons couvrent les
    // petites valeurs (soustraction r-x arrondit à 0.5) et les grandes (≥ 2^52).
    if r - x > 0.5 || r - 0.5 > x {
        r - 1.0 // 0,49999999999999994 → 0 (V8), pas 1
    } else {
        r
    }
}

/// `Math.max(a, b)` JS : NaN dans UN argument → NaN (f64::max de Rust l'ignore).
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// `accountAge` : `now.getFullYear() - created.getFullYear()` ; si < 1 an,
/// diff en mois `(y2-y1)*12 + m2-m1` → `${months}mo`, sinon `${years}y`.
/// `new Date(createdAt)` via `crate::jsdate` (R1) : date invalide → NaN →
/// `NaN < 1` false → `"NaNy"`, comme le legacy (oracle). Accepte aussi
/// null (epoch) et number (ms).
pub fn account_age(created_at: &JsVal) -> String {
    let ms = match jsdate::new_date(created_at) {
        jsdate::JsDate::Valid(ms) => ms,
        jsdate::JsDate::Invalid => return "NaNy".to_string(),
    };
    let days = (ms / 86_400_000.0).floor() as i64;
    let (cy, cm, _) = jsdate::civil_from_days_full(days);
    let (ny, nm) = now_ym();
    let years = ny - cy;
    if years < 1 {
        let months = (ny - cy) * 12 + i64::from(nm) - i64::from(cm);
        return format!("{months}mo");
    }
    format!("{years}y")
}

/// `renderAreaChart` : grille, aire, ligne, ticks X (≈ tous les 2 mois) et Y.
fn render_area_chart(
    weeks: &[WeekBucket],
    ring_color: &str,
    muted_color: &str,
    text_color: &str,
) -> Result<String, RenderError> {
    if weeks.is_empty() {
        return Ok(String::new());
    }

    // Math.max(...counts, 1) — absorbant NaN comme V8 (oracle R2 : count absent → NaN)
    let mut max_count = 1.0f64;
    for w in weeks {
        max_count = js_max(max_count, jsval::to_number(&w.count));
    }
    let n = weeks.len();

    // points
    let points: Vec<(f64, f64)> = weeks
        .iter()
        .enumerate()
        .map(|(i, w)| {
            (
                CHART_LEFT + (i as f64 / (n - 1) as f64) * (CHART_RIGHT - CHART_LEFT),
                CHART_BOTTOM
                    - (jsval::to_number(&w.count) / max_count) * (CHART_BOTTOM - CHART_TOP),
            )
        })
        .collect();

    // aire + ligne
    let first = points[0];
    let last = points[points.len() - 1];
    let line_path = format!(
        "M{},{} {}",
        js(first.0),
        js(first.1),
        monotone_cubic_path(&points)
    );
    let area_path = format!(
        "{line_path} L{},{} L{},{} Z",
        js(last.0),
        js(CHART_BOTTOM),
        js(first.0),
        js(CHART_BOTTOM)
    );

    // ticks X (≈ tous les 2 mois = ~8 semaines)
    let mut x_ticks: Vec<String> = Vec::new();
    let tick_interval = 1.max(n / 6);
    let mut i = 0;
    while i < n {
        let w = &weeks[i];
        let x = CHART_LEFT + (i as f64 / (n - 1) as f64) * (CHART_RIGHT - CHART_LEFT);
        let month = intl::month_en_tick(&w.label);
        x_ticks.push(format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"9\" fill=\"{muted_color}\">{month}</text>",
            js(x),
            js(CHART_BOTTOM + 14.0)
        ));
        i += tick_interval;
    }

    // ticks Y
    let mid = js_round(max_count / 2.0);
    let y_ticks: [(f64, f64); 3] = [
        (0.0, CHART_BOTTOM),
        (
            mid,
            CHART_BOTTOM - (mid / max_count) * (CHART_BOTTOM - CHART_TOP),
        ),
        (max_count, CHART_TOP),
    ];
    let y_tick_svg: Vec<String> = y_ticks
        .iter()
        .map(|(val, y)| {
            format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"9\" fill=\"{muted_color}\">{}</text>",
                js(CHART_LEFT - 6.0),
                js(y + 3.0),
                js(*val)
            )
        })
        .collect();

    // lignes de grille
    let grid_lines: Vec<String> = y_ticks
        .iter()
        .map(|(_, y)| {
            format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{muted_color}\" stroke-opacity=\"0.15\" stroke-width=\"0.5\" />",
                js(CHART_LEFT),
                js(*y),
                js(CHART_RIGHT),
                js(*y)
            )
        })
        .collect();

    Ok(format!(
        "\n    {grid}\n    <path d=\"{area_path}\" fill=\"{ring_color}\" fill-opacity=\"0.15\" />\n    <path d=\"{line_path}\" fill=\"none\" stroke=\"{ring_color}\" stroke-width=\"1.5\" />\n    {xt}\n    {yt}\n    <text x=\"{x_mid}\" y=\"{y_lab}\" text-anchor=\"middle\" font-size=\"10\" fill=\"{text_color}\">contributions in the last year</text>\n  ",
        grid = grid_lines.join(""),
        xt = x_ticks.join(""),
        yt = y_tick_svg.join(""),
        x_mid = js((CHART_LEFT + CHART_RIGHT) / 2.0),
        y_lab = js(CHART_BOTTOM + 28.0),
    ))
}

pub fn render_profile_card(
    data: &ProfileData,
    options: &CardOptions,
) -> Result<String, RenderError> {
    let theme = get_theme(&options.theme);
    let icon_color = match pick(&options.icon_color) {
        Some(c) => format!("#{c}"),
        None => theme.icon.to_string(),
    };
    let text_color = match pick(&options.text_color) {
        Some(c) => format!("#{c}"),
        None => theme.text.to_string(),
    };

    let title_section = if options.hide_title {
        String::new()
    } else {
        // `data.name || data.username` JS : falsy (''/null/0/absent) → username
        let shown = if jsval::truthy(&data.name) {
            data.name.clone()
        } else {
            data.username.clone()
        };
        format!(
            "<text x=\"25\" y=\"30\" class=\"title\">{}</text>",
            encode_html_js(&shown)?
        )
    };
    let title_offset: f64 = if options.hide_title { -10.0 } else { 0.0 };

    let mut rows: Vec<(&'static str, &'static str, JsVal)> = vec![
        (
            "history",
            "Contributions",
            JsVal::Str(format_number(
                &data.contributions_this_year,
                &options.locale,
            )),
        ),
        (
            "repo",
            "Public Repos",
            JsVal::Str(format_number(&data.public_repos, &options.locale)),
        ),
        (
            "calendar",
            "Account Age",
            JsVal::Str(account_age(&data.created_at)),
        ),
    ];

    // `if (data.email)` JS : truthy uniquement — la valeur brute part dans
    // `encodeHTML(row.value)` : TypeError capturable si non-string (oracle R2 :
    // email: 0 → truthy → legacy lève ; null/''/absent → pas de rangée).
    if jsval::truthy(&data.email) {
        rows.push(("mail", "Email", data.email.clone()));
    }

    let row_height = 26.0;
    let start_y = 55.0 + title_offset;

    let mut rows_svg = String::new();
    for (i, (icon, label, value)) in rows.iter().enumerate() {
        let y = start_y + i as f64 * row_height;
        let icon_part = if options.show_icons {
            octicon_svg(icon, 25.0, y - 13.0, 16, &icon_color)?
        } else {
            String::new()
        };
        let label_x = if options.show_icons { 47.0 } else { 25.0 };
        rows_svg.push_str(&format!(
            "\n        <g>\n          {icon_part}\n          <text x=\"{}\" y=\"{}\" class=\"stat-label\">{}</text>\n          <text x=\"290\" y=\"{}\" class=\"stat-value\" text-anchor=\"end\">{}</text>\n        </g>\n      ",
            js(label_x),
            js(y),
            crate::sanitize::encode_html(label),
            js(y),
            encode_html_js(value)?
        ));
    }

    // séparateur vertical
    let divider = format!(
        "<line x1=\"305\" y1=\"15\" x2=\"305\" y2=\"185\" stroke=\"{}\" stroke-width=\"1\" />",
        theme.border
    );

    // graphique d'aire des contributions
    let weeks = aggregate_weekly(&data.contribution_calendar);
    let chart = render_area_chart(&weeks, theme.ring, theme.muted, &text_color)?;

    let content = format!("\n    {title_section}\n    {rows_svg}\n    {divider}\n    {chart}\n  ");

    Ok(create_card_wrapper(
        CARD_WIDTH,
        CARD_HEIGHT,
        &content,
        theme,
        options,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_age_formules_du_legacy() {
        // PAS de pluriel, PAS de troncature : écart de composantes calendaires.
        set_clock(2026, 11); // corpus figé au 2026-11-24 UTC
        assert_eq!(
            account_age(&JsVal::Str("2026-11-01T00:00:00Z".into())),
            "0mo"
        ); // même mois
        assert_eq!(
            account_age(&JsVal::Str("2026-11-24T23:59:59Z".into())),
            "0mo"
        );
        assert_eq!(
            account_age(&JsVal::Str("2026-05-01T00:00:00Z".into())),
            "6mo"
        );
        // années >= 1 → `${years}y` : le legacy ne rend JAMAIS un reste en mois
        assert_eq!(
            account_age(&JsVal::Str("2025-12-31T00:00:00Z".into())),
            "1y"
        );
        // dernier palier du branchement `mo` : janvier → 10mo (mois 0-indexés)
        assert_eq!(
            account_age(&JsVal::Str("2026-01-15T00:00:00Z".into())),
            "10mo"
        );
        // 1 an exact (écart d'année calendaire ≥ 1 → 'y', même à 1 jour près)
        assert_eq!(
            account_age(&JsVal::Str("2025-11-24T00:00:00Z".into())),
            "1y"
        );
        assert_eq!(
            account_age(&JsVal::Str("2025-12-01T00:00:00Z".into())),
            "1y"
        );
        assert_eq!(
            account_age(&JsVal::Str("2020-01-15T12:00:00Z".into())),
            "6y"
        );
        assert_eq!(
            account_age(&JsVal::Str("2013-04-01T00:00:00Z".into())),
            "13y"
        );
        // R1 : non-strings via new Date (oracle) — null → epoch 1970 ; absent → NaNy
        assert_eq!(account_age(&JsVal::Null), "56y");
        assert_eq!(account_age(&JsVal::Undefined), "NaNy");
        assert_eq!(account_age(&JsVal::Number(-86_400_000.0)), "57y");
        assert_eq!(account_age(&JsVal::Str("garbage".into())), "NaNy");
        clear_clock();
    }

    #[test]
    fn js_round_math_round_exact_r5() {
        // Oracle Node v26.13.1 : Math.round(0.49999999999999994) === 0
        assert_eq!(js_round(0.499_999_999_999_999_94), 0.0);
        assert_eq!(js_round(0.5), 1.0);
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(2.4), 2.0);
        assert_eq!(js_round(-0.5), 0.0); // String(-0) → "0" à l'interpolation
        assert!(js_round(-0.5).is_sign_negative());
        assert_eq!(js_round(-0.4), 0.0);
        assert!(js_round(-0.4).is_sign_negative()); // Math.round(-0.4) === -0
        assert!(js_round(f64::NAN).is_nan());
        assert_eq!(js_round(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn js_max_absorbe_nan_comme_math_max() {
        assert!(js_max(f64::NAN, 1.0).is_nan());
        assert!(js_max(1.0, f64::NAN).is_nan());
        assert_eq!(js_max(-5.0, 1.0), 1.0);
        assert_eq!(js_max(42.0, 1.0), 42.0);
    }

    #[test]
    fn aggregate_weekly_tranches_de_7() {
        let days: Vec<ContributionDay> = (0..10)
            .map(|i| ContributionDay {
                date: JsVal::Str(format!("2026-11-{i:02}")),
                count: JsVal::Number(1.0),
            })
            .collect();
        let weeks = aggregate_weekly(&days);
        assert_eq!(weeks.len(), 2);
        assert_eq!(weeks[0].count, JsVal::Number(7.0));
        assert_eq!(weeks[1].count, JsVal::Number(3.0));
        assert_eq!(weeks[0].label, JsVal::Str("2026-11-03".into())); // index 3/7
        assert_eq!(weeks[1].label, JsVal::Str("2026-11-08".into())); // index 1/3
    }

    #[test]
    fn somme_semantique_js_sur_compteurs() {
        // oracle R2 : 0 + "42" → "042" (concat), 0 + null → 0, 0 + undefined → NaN
        let mk = |c: JsVal| ContributionDay {
            date: JsVal::Str("2026-11-01".into()),
            count: c,
        };
        let w = aggregate_weekly(&[mk(JsVal::Str("42".into()))]);
        assert_eq!(w[0].count, JsVal::Str("042".into()));
        let w = aggregate_weekly(&[mk(JsVal::Null)]);
        assert_eq!(w[0].count, JsVal::Number(0.0));
        let w = aggregate_weekly(&[mk(JsVal::Undefined)]);
        assert!(matches!(w[0].count, JsVal::Number(n) if n.is_nan()));
        let w = aggregate_weekly(&[mk(JsVal::Number(-5.0))]);
        assert_eq!(w[0].count, JsVal::Number(-5.0));
    }
}
