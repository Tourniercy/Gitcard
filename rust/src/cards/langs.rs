//! Port littéral de `packages/svg-renderer/src/cards/langs-card.ts` (141 lignes) :
//! donut standard + variante `layout=compact` (barre empilée, légende 2 colonnes).

use super::base::{create_card_wrapper, create_donut_segment, encode_html, js, pick, to_fixed};
use crate::fdlibm;
use crate::themes::{get_theme, Theme};
use crate::types::{CardOptions, TopLangsData};

const CARD_WIDTH: f64 = 495.0;
const CARD_HEIGHT: f64 = 195.0;

const COMPACT_WIDTH: f64 = 300.0;

fn render_compact_langs_card(data: &TopLangsData, options: &CardOptions, theme: &Theme) -> String {
    let title_section = if options.hide_title {
        String::new()
    } else {
        "<text x=\"25\" y=\"30\" class=\"title\">Most Used Languages</text>".to_string()
    };

    let bar_y: f64 = if options.hide_title { 20.0 } else { 48.0 };
    let bar_height = 8.0;
    let bar_left = 25.0;
    let bar_width = COMPACT_WIDTH - 50.0;

    // segments de la barre empilée — accumulation flottante identique au legacy
    let mut bar_x = bar_left;
    let mut bar_segments = String::new();
    for lang in &data.languages {
        let w = (lang.percentage / 100.0) * bar_width;
        bar_segments.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\">\n        <animate attributeName=\"width\" from=\"0\" to=\"{}\" dur=\"0.6s\" fill=\"freeze\" />\n      </rect>",
            js(bar_x),
            js(bar_y),
            js(w),
            js(bar_height),
            lang.color,
            js(w)
        ));
        bar_x += w;
    }

    // clip aux coins arrondis (id statique, pas d'uid consommé ici)
    let bar_clip_id = "compact-bar-clip";
    let bar_clip = format!(
        "<clipPath id=\"{bar_clip_id}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"5\" ry=\"5\" /></clipPath>",
        js(bar_left),
        js(bar_y),
        js(bar_width),
        js(bar_height)
    );

    // légende 2 colonnes sous la barre
    let legend_start_y = bar_y + bar_height + 18.0;
    let legend_row_height = 25.0;
    let col_width = (COMPACT_WIDTH - 50.0) / 2.0;

    let mut legend_items = String::new();
    for (i, lang) in data.languages.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let x = bar_left + col as f64 * col_width;
        let y = legend_start_y + row as f64 * legend_row_height;
        let delay = to_fixed(450.0 + i as f64 * 150.0, 0);
        let fill = match pick(&options.text_color) {
            Some(c) => format!("#{c}"),
            None => theme.text.to_string(),
        };
        legend_items.push_str(&format!(
            "\n        <g opacity=\"0\">\n          <circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\" />\n          <text x=\"{}\" y=\"{}\" class=\"lang-name\" font-size=\"11\" fill=\"{}\">{} {}%</text>\n          <animate attributeName=\"opacity\" from=\"0\" to=\"1\" dur=\"0.3s\" begin=\"{delay}ms\" fill=\"freeze\" />\n        </g>\n      ",
            js(x),
            js(y - 4.0),
            lang.color,
            js(x + 14.0),
            js(y),
            fill,
            encode_html(&lang.name),
            to_fixed(lang.percentage, 2)
        ));
    }

    let legend_rows = data.languages.len().div_ceil(2);
    let dynamic_height = legend_start_y + legend_rows as f64 * legend_row_height + 10.0;

    let empty_message = if data.languages.is_empty() {
        format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" class=\"muted\">No language data available</text>",
            js(COMPACT_WIDTH / 2.0),
            js(dynamic_height / 2.0)
        )
    } else {
        String::new()
    };

    let content = format!(
        "\n    <defs>{bar_clip}</defs>\n    {title_section}\n    <g clip-path=\"url(#{bar_clip_id})\">{bar_segments}</g>\n    {legend_items}\n    {empty_message}\n  "
    );

    create_card_wrapper(COMPACT_WIDTH, dynamic_height, &content, theme, options)
}

pub fn render_langs_card(data: &TopLangsData, options: &CardOptions) -> String {
    let theme = get_theme(&options.theme);

    if options.layout.as_deref() == Some("compact") {
        return render_compact_langs_card(data, options, theme);
    }

    let title_section = if options.hide_title {
        String::new()
    } else {
        "<text x=\"25\" y=\"30\" class=\"title\">Most Used Languages</text>".to_string()
    };

    let title_offset: f64 = if options.hide_title { -10.0 } else { 0.0 };
    let donut_cx = 90.0;
    let donut_cy = 110.0 + title_offset;
    let donut_radius = 55.0;

    // segments du donut — accumulation d'angle identique au legacy
    let mut current_angle = 0.0f64;
    let mut donut_segments = String::new();
    for lang in &data.languages {
        let sweep_angle = (lang.percentage / 100.0) * std::f64::consts::PI * 2.0;
        // petit espace entre segments
        let gap = 0.02;
        let start_angle = current_angle + gap;
        let end_angle = current_angle + sweep_angle - gap;
        current_angle += sweep_angle;

        if sweep_angle < 0.05 {
            continue; // segments trop petits ignorés
        }

        donut_segments.push_str(&create_donut_segment(
            donut_cx,
            donut_cy,
            donut_radius,
            start_angle,
            end_angle,
            &lang.color,
        ));
    }

    // légende
    let legend_x = 185.0;
    let legend_start_y = 60.0 + title_offset;
    let legend_row_height = 24.0;

    let mut legend_items = String::new();
    for (i, lang) in data.languages.iter().enumerate() {
        let y = legend_start_y + i as f64 * legend_row_height;
        legend_items.push_str(&format!(
            "\n        <g>\n          <circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\" />\n          <text x=\"{}\" y=\"{}\" class=\"stat-label\">{}</text>\n          <text x=\"{}\" y=\"{}\" class=\"stat-value\" text-anchor=\"end\">{}%</text>\n        </g>\n      ",
            js(legend_x),
            js(y - 4.0),
            lang.color,
            js(legend_x + 14.0),
            js(y),
            encode_html(&lang.name),
            js(CARD_WIDTH - 25.0),
            js(y),
            to_fixed(lang.percentage, 1)
        ));
    }

    let empty_message = if data.languages.is_empty() {
        format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" class=\"muted\">No language data available</text>",
            js(CARD_WIDTH / 2.0),
            js(CARD_HEIGHT / 2.0)
        )
    } else {
        String::new()
    };

    let content = format!(
        "\n    {title_section}\n    {donut_segments}\n    {legend_items}\n    {empty_message}\n  "
    );

    create_card_wrapper(CARD_WIDTH, CARD_HEIGHT, &content, theme, options)
}

// `fdlibm` référencé pour documenter C1 : les cos/sin du donut passent par
// `createDonutSegment` → `polarToCartesian` (cards/base), qui utilise fdlibm.
const _: fn(f64) -> f64 = fdlibm::cos;
