//! Port littéral de `packages/svg-renderer/src/cards/streak-card.ts` (95 lignes).

use super::base::{create_card_wrapper, encode_html, format_number, js, octicon_svg, pick};
use crate::intl;
use crate::themes::get_theme;
use crate::types::{CardOptions, StreakData};

const CARD_WIDTH: f64 = 495.0;
const CARD_HEIGHT: f64 = 195.0;

/// `formatDateRange` : dates 'YYYY-MM-DD' → `Intl.DateTimeFormat('en', …)`
/// (table C3) ; '' si borne absente (falsy JS).
fn format_date_range(start: &str, end: &str) -> String {
    if start.is_empty() || end.is_empty() {
        return String::new();
    }
    let s = intl::date_en(start).unwrap_or_else(|| panic!("date invalide: {start}"));
    let e = intl::date_en(end).unwrap_or_else(|| panic!("date invalide: {end}"));
    format!("{s} - {e}")
}

pub fn render_streak_card(data: &StreakData, options: &CardOptions) -> String {
    let theme = get_theme(&options.theme);
    let accent_color = match pick(&options.icon_color) {
        Some(c) => format!("#{c}"),
        None => theme.ring.to_string(),
    };
    let text_color = match pick(&options.text_color) {
        Some(c) => format!("#{c}"),
        None => theme.text.to_string(),
    };
    let muted_color = theme.muted;

    let title_section = if options.hide_title {
        String::new()
    } else {
        format!(
            "<text x=\"{}\" y=\"28\" class=\"title\" text-anchor=\"middle\">Contribution Streak</text>",
            js(CARD_WIDTH / 2.0)
        )
    };

    let title_offset: f64 = if options.hide_title { -10.0 } else { 0.0 };

    let col_width = CARD_WIDTH / 3.0;
    let col1_x = col_width / 2.0;
    let col2_x = col_width + col_width / 2.0;
    let col3_x = col_width * 2.0 + col_width / 2.0;

    let icon_y = 44.0 + title_offset;
    let label_y = 72.0 + title_offset;
    let value_y = 110.0 + title_offset;
    let unit_y = 128.0 + title_offset;
    let date_y = 158.0 + title_offset;

    let content = format!(
        "\n    {title_section}\n\n    <!-- Total Contributions -->\n    <g>\n      {cal}\n      <text x=\"{c1}\" y=\"{ly}\" text-anchor=\"middle\" class=\"stat-label\">Contributions</text>\n      <text x=\"{c1}\" y=\"{vy}\" text-anchor=\"middle\"\n            font-size=\"24\" font-weight=\"700\" fill=\"{text_color}\">\n        {total}\n      </text>\n      <text x=\"{c1}\" y=\"{dy}\" text-anchor=\"middle\" class=\"muted\">\n        Past Year\n      </text>\n    </g>\n\n    <!-- Divider 1 -->\n    <line x1=\"{cw}\" y1=\"{d1a}\" x2=\"{cw}\" y2=\"{d1b}\"\n          stroke=\"{border}\" stroke-width=\"1\" opacity=\"0.5\" />\n\n    <!-- Current Streak (emphasized) -->\n    <g>\n      {flame}\n      <text x=\"{c2}\" y=\"{ly}\" text-anchor=\"middle\" class=\"stat-label\">Current Streak</text>\n      <text x=\"{c2}\" y=\"{vy4}\" text-anchor=\"middle\"\n            font-size=\"32\" font-weight=\"800\" fill=\"{accent_color}\">\n        {cur}\n      </text>\n      <text x=\"{c2}\" y=\"{uy6}\" text-anchor=\"middle\"\n            font-size=\"13\" fill=\"{muted_color}\">days</text>\n      <text x=\"{c2}\" y=\"{dy}\" text-anchor=\"middle\" class=\"muted\">\n        {cur_range}\n      </text>\n    </g>\n\n    <!-- Divider 2 -->\n    <line x1=\"{cw2}\" y1=\"{d1a}\" x2=\"{cw2}\" y2=\"{d1b}\"\n          stroke=\"{border}\" stroke-width=\"1\" opacity=\"0.5\" />\n\n    <!-- Longest Streak -->\n    <g>\n      {trophy}\n      <text x=\"{c3}\" y=\"{ly}\" text-anchor=\"middle\" class=\"stat-label\">Longest Streak</text>\n      <text x=\"{c3}\" y=\"{vy}\" text-anchor=\"middle\"\n            font-size=\"24\" font-weight=\"700\" fill=\"{text_color}\">\n        {long}\n      </text>\n      <text x=\"{c3}\" y=\"{uy}\" text-anchor=\"middle\"\n            font-size=\"12\" fill=\"{muted_color}\">days</text>\n      <text x=\"{c3}\" y=\"{dy}\" text-anchor=\"middle\" class=\"muted\">\n        {long_range}\n      </text>\n    </g>\n  ",
        cal = octicon_svg("calendar", col1_x - 8.0, icon_y, 16, &accent_color),
        c1 = js(col1_x),
        ly = js(label_y),
        vy = js(value_y),
        total = format_number(data.total_contributions, &options.locale),
        dy = js(date_y),
        cw = js(col_width),
        d1a = js(55.0 + title_offset),
        d1b = js(170.0 + title_offset),
        border = theme.border,
        flame = octicon_svg("flame", col2_x - 8.0, icon_y, 16, &accent_color),
        c2 = js(col2_x),
        vy4 = js(value_y + 4.0),
        accent_color = accent_color,
        cur = js(data.current_streak as f64),
        uy6 = js(unit_y + 6.0),
        muted_color = muted_color,
        cur_range = encode_html(&format_date_range(&data.current_streak_start, &data.current_streak_end)),
        cw2 = js(col_width * 2.0),
        trophy = octicon_svg("trophy", col3_x - 8.0, icon_y, 16, &accent_color),
        c3 = js(col3_x),
        long = js(data.longest_streak as f64),
        uy = js(unit_y),
        long_range = encode_html(&format_date_range(&data.longest_streak_start, &data.longest_streak_end)),
    );

    create_card_wrapper(CARD_WIDTH, CARD_HEIGHT, &content, theme, options)
}
