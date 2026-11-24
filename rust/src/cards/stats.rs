//! Port littéral de `packages/svg-renderer/src/cards/stats-card.ts` (101 lignes).
//! Fix2 : `Result` (le legacy lève un TypeError sur `encodeHTML(stats.name)` si
//! `name` n'est pas une string) + compteurs `JsVal` (R2).

use super::base::{
    create_card_wrapper, create_ring_chart, encode_html, format_number, js, octicon_svg, pick,
};
use crate::cards::RenderError;
use crate::jsval::{self, JsVal};
use crate::themes::get_theme;
use crate::types::{CardOptions, GitHubStats};

const CARD_WIDTH: f64 = 495.0;
const CARD_HEIGHT: f64 = 195.0;

struct StatRow<'a> {
    key: &'static str,
    label: &'static str,
    value: &'a JsVal,
    icon: &'static str,
}

/// `calculateGrade` — score flottant (forks × 0.5), seuils décroissants.
/// `ToNumber` JS : champ absent/null → NaN (score NaN → grade « C », comme le
/// legacy où `score = undefined * 1 + …` = NaN).
fn calculate_grade(stats: &GitHubStats) -> (&'static str, f64) {
    let score = jsval::to_number(&stats.total_commits) * 1.0
        + jsval::to_number(&stats.total_prs) * 3.0
        + jsval::to_number(&stats.total_issues) * 2.0
        + jsval::to_number(&stats.total_stars) * 1.0
        + jsval::to_number(&stats.total_forks) * 0.5
        + jsval::to_number(&stats.contributed_to) * 5.0;

    if score >= 5000.0 {
        ("A+", 100.0)
    } else if score >= 2500.0 {
        ("A", 85.0)
    } else if score >= 1000.0 {
        ("B+", 70.0)
    } else if score >= 500.0 {
        ("B", 55.0)
    } else {
        ("C", 40.0)
    }
}

pub fn render_stats_card(
    stats: &GitHubStats,
    options: &CardOptions,
) -> Result<String, RenderError> {
    let theme = get_theme(&options.theme);
    let icon_color = match pick(&options.icon_color) {
        Some(c) => format!("#{c}"),
        None => theme.icon.to_string(),
    };
    let (grade, percentage) = calculate_grade(stats);

    let all_stats = [
        StatRow {
            key: "stars",
            label: "Total Stars",
            value: &stats.total_stars,
            icon: "star",
        },
        StatRow {
            key: "forks",
            label: "Total Forks",
            value: &stats.total_forks,
            icon: "repo-forked",
        },
        StatRow {
            key: "commits",
            label: "Total Commits",
            value: &stats.total_commits,
            icon: "git-commit",
        },
        StatRow {
            key: "prs",
            label: "Total PRs",
            value: &stats.total_prs,
            icon: "git-pull-request",
        },
        StatRow {
            key: "issues",
            label: "Total Issues",
            value: &stats.total_issues,
            icon: "issue-opened",
        },
        StatRow {
            key: "contribs",
            label: "Contributed To",
            value: &stats.contributed_to,
            icon: "repo",
        },
    ];

    let visible_stats: Vec<&StatRow> = all_stats
        .iter()
        .filter(|s| !options.hide.iter().any(|h| h == s.key))
        .collect();

    let title_section = if options.hide_title {
        String::new()
    } else {
        format!(
            "<text x=\"25\" y=\"30\" class=\"title\">{}&#39;s GitHub Stats</text>",
            crate::sanitize::encode_html_js(&stats.name)?
        )
    };

    let title_offset: f64 = if options.hide_title { 0.0 } else { 10.0 };
    let ring_cx = 75.0;
    let ring_cy = 105.0 + title_offset;
    let ring_radius = 40.0;

    let ring_section = create_ring_chart(
        ring_cx,
        ring_cy,
        ring_radius,
        percentage,
        theme.ring,
        theme.muted,
        grade,
    );

    let stats_start_x = 160.0;
    let stats_start_y = 55.0 + title_offset;
    let row_height = 22.0;

    let mut stats_section = String::new();
    for (i, stat) in visible_stats.iter().enumerate() {
        let y = stats_start_y + i as f64 * row_height;
        let icon_part = if options.show_icons {
            octicon_svg(stat.icon, stats_start_x, y - 13.0, 16, &icon_color)?
        } else {
            String::new()
        };
        let label_x = if options.show_icons {
            stats_start_x + 22.0
        } else {
            stats_start_x
        };
        stats_section.push_str(&format!(
            "\n        <g>\n          {icon_part}\n          <text x=\"{}\" y=\"{}\" class=\"stat-label\">{}</text>\n          <text x=\"{}\" y=\"{}\" class=\"stat-value\" text-anchor=\"end\">{}</text>\n        </g>\n      ",
            js(label_x),
            js(y),
            encode_html(stat.label),
            js(CARD_WIDTH - 25.0),
            js(y),
            format_number(stat.value, &options.locale)
        ));
    }

    let content = format!("\n    {title_section}\n    {ring_section}\n    {stats_section}\n  ");

    Ok(create_card_wrapper(
        CARD_WIDTH,
        CARD_HEIGHT,
        &content,
        theme,
        options,
    ))
}
