//! Port littéral de `packages/svg-renderer/src/types.ts` (99 lignes) : structures
//! `serde` des fixtures `FetchResult` (agrégées), options de carte, locales supportées.
//!
//! Les nombres entiers des fixtures (compteurs) sont portés en `u64` ; `percentage`
//! reste `f64` (divisions flottantes côté agrégation, P3).

use serde_json::Value;

/// `LOCALE_ENTRIES` (ordre d'insertion : 20 entrées, types.ts).
pub const LOCALE_ENTRIES: [(&str, &str); 20] = [
    ("en", "GB"),
    ("fr", "FR"),
    ("de", "DE"),
    ("es", "ES"),
    ("pt", "PT"),
    ("pt-BR", "BR"),
    ("it", "IT"),
    ("nl", "NL"),
    ("ja", "JP"),
    ("ko", "KR"),
    ("zh", "CN"),
    ("zh-TW", "TW"),
    ("ru", "RU"),
    ("ar", "SA"),
    ("hi", "IN"),
    ("pl", "PL"),
    ("tr", "TR"),
    ("sv", "SE"),
    ("da", "DK"),
    ("nb", "NO"),
];

/// `countryToFlag` : indicatifs régionaux U+1F1E6.. (Regional Indicator Symbols).
pub fn country_to_flag(country: &str) -> String {
    country
        .chars()
        .map(|c| char::from_u32(0x1f1e6 - 65 + u32::from(c)).expect("lettre A-Z"))
        .collect()
}

/// `SUPPORTED_LOCALES` : `Intl.DisplayNames([code], { type: 'language' }).of(code) ?? code`.
pub fn supported_locales() -> Vec<(&'static str, String, String)> {
    LOCALE_ENTRIES
        .iter()
        .map(|&(code, country)| {
            let label = crate::intl::display_name(code).unwrap_or_else(|| code.to_string());
            (code, label, country_to_flag(country))
        })
        .collect()
}

/// `LOCALE_CODES` (ordre d'insertion).
pub fn locale_codes() -> Vec<&'static str> {
    LOCALE_ENTRIES.iter().map(|&(c, _)| c).collect()
}

/// `CardOptions` (types.ts) — produits par `parseCardOptions` côté API (P3) ;
/// ici reconstruits depuis le manifeste golden pour la parité de rendu.
#[derive(Debug, Clone)]
pub struct CardOptions {
    pub theme: String,
    pub hide: Vec<String>,
    pub show_icons: bool,
    pub hide_border: bool,
    pub hide_title: bool,
    pub bg_color: Option<String>,
    pub title_color: Option<String>,
    pub text_color: Option<String>,
    pub icon_color: Option<String>,
    pub border_color: Option<String>,
    pub cache_seconds: i64,
    pub locale: String,
    pub layout: Option<String>,
}

impl CardOptions {
    /// Reconstruit les options depuis l'objet `options` d'une entrée du manifeste
    /// (= `JSON.parse(JSON.stringify(options))` du legacy, mêmes clés camelCase).
    pub fn from_value(v: &Value) -> CardOptions {
        let s = |k: &str| {
            v.get(k)
                .and_then(Value::as_str)
                .filter(|x| !x.is_empty())
                .map(str::to_string)
        };
        CardOptions {
            theme: v
                .get("theme")
                .and_then(Value::as_str)
                .unwrap_or("default")
                .to_string(),
            hide: v
                .get("hide")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            show_icons: v.get("showIcons").and_then(Value::as_bool).unwrap_or(true),
            hide_border: v
                .get("hideBorder")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            hide_title: v.get("hideTitle").and_then(Value::as_bool).unwrap_or(false),
            bg_color: s("bgColor"),
            title_color: s("titleColor"),
            text_color: s("textColor"),
            icon_color: s("iconColor"),
            border_color: s("borderColor"),
            cache_seconds: v
                .get("cacheSeconds")
                .and_then(Value::as_i64)
                .unwrap_or(14400),
            locale: v
                .get("locale")
                .and_then(Value::as_str)
                .unwrap_or("en")
                .to_string(),
            layout: s("layout"),
        }
    }
}

/// `GitHubStats` (types.ts).
#[derive(Debug, Clone, Default)]
pub struct GitHubStats {
    pub username: String,
    pub name: String,
    pub total_stars: u64,
    pub total_forks: u64,
    pub total_commits: u64,
    pub total_prs: u64,
    pub total_issues: u64,
    pub contributed_to: u64,
}

/// `StreakData` (types.ts).
#[derive(Debug, Clone, Default)]
pub struct StreakData {
    pub username: String,
    pub total_contributions: u64,
    pub current_streak: u64,
    pub longest_streak: u64,
    pub current_streak_start: String,
    pub current_streak_end: String,
    pub longest_streak_start: String,
    pub longest_streak_end: String,
}

/// `LanguageData` (types.ts).
#[derive(Debug, Clone, Default)]
pub struct LanguageData {
    pub name: String,
    pub percentage: f64,
    pub color: String,
    pub size: u64,
}

/// `TopLangsData` (types.ts).
#[derive(Debug, Clone, Default)]
pub struct TopLangsData {
    pub username: String,
    pub languages: Vec<LanguageData>,
}

/// `ContributionDay` (types.ts).
#[derive(Debug, Clone, Default)]
pub struct ContributionDay {
    pub date: String,
    pub count: u64,
}

/// `ProfileData` (types.ts).
#[derive(Debug, Clone, Default)]
pub struct ProfileData {
    pub username: String,
    pub name: String,
    pub contributions_this_year: u64,
    pub public_repos: u64,
    pub created_at: String,
    pub email: Option<String>,
    pub contribution_calendar: Vec<ContributionDay>,
}

// ---------------------------------------------------------------- fixtures (JSON)

fn str_field(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn u64_field(v: &Value, k: &str) -> u64 {
    v.get(k).and_then(Value::as_u64).unwrap_or_default()
}

impl GitHubStats {
    pub fn from_value(v: &Value) -> GitHubStats {
        GitHubStats {
            username: str_field(v, "username"),
            // `name` peut être null dans les fixtures (newcomer) → String vide ;
            // la sémantique JS `stats.name` devient "" (utilisé tel quel par le titre).
            name: v
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            total_stars: u64_field(v, "totalStars"),
            total_forks: u64_field(v, "totalForks"),
            total_commits: u64_field(v, "totalCommits"),
            total_prs: u64_field(v, "totalPRs"),
            total_issues: u64_field(v, "totalIssues"),
            contributed_to: u64_field(v, "contributedTo"),
        }
    }
}

impl StreakData {
    pub fn from_value(v: &Value) -> StreakData {
        StreakData {
            username: str_field(v, "username"),
            total_contributions: u64_field(v, "totalContributions"),
            current_streak: u64_field(v, "currentStreak"),
            longest_streak: u64_field(v, "longestStreak"),
            current_streak_start: str_field(v, "currentStreakStart"),
            current_streak_end: str_field(v, "currentStreakEnd"),
            longest_streak_start: str_field(v, "longestStreakStart"),
            longest_streak_end: str_field(v, "longestStreakEnd"),
        }
    }
}

impl TopLangsData {
    pub fn from_value(v: &Value) -> TopLangsData {
        let langs = v
            .get("languages")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|l| LanguageData {
                        name: str_field(l, "name"),
                        percentage: l
                            .get("percentage")
                            .and_then(Value::as_f64)
                            .unwrap_or_default(),
                        color: str_field(l, "color"),
                        size: u64_field(l, "size"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        TopLangsData {
            username: str_field(v, "username"),
            languages: langs,
        }
    }
}

impl ProfileData {
    pub fn from_value(v: &Value) -> ProfileData {
        ProfileData {
            username: str_field(v, "username"),
            name: v
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            contributions_this_year: u64_field(v, "contributionsThisYear"),
            public_repos: u64_field(v, "publicRepos"),
            created_at: str_field(v, "createdAt"),
            // email: null (JSON) ou "" (falsy JS) → pas de ligne Email
            email: v
                .get("email")
                .and_then(Value::as_str)
                .filter(|e| !e.is_empty())
                .map(str::to_string),
            contribution_calendar: v
                .get("contributionCalendar")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .map(|d| ContributionDay {
                            date: str_field(d, "date"),
                            count: u64_field(d, "count"),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drapeaux_et_labels_locales() {
        assert_eq!(country_to_flag("FR"), "\u{1f1eb}\u{1f1f7}");
        assert_eq!(country_to_flag("GB"), "\u{1f1ec}\u{1f1e7}");
        assert_eq!(country_to_flag("TW"), "\u{1f1f9}\u{1f1fc}");
        let sup = supported_locales();
        assert_eq!(sup.len(), 20);
        assert_eq!(sup[0].0, "en");
        assert_eq!(sup[0].1, "English");
        assert_eq!(sup[1].1, "français");
        assert_eq!(locale_codes()[13], "ar");
        assert_eq!(locale_codes().len(), 20);
    }
}
