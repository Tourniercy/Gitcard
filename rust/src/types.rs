//! Port littéral de `packages/svg-renderer/src/types.ts` (99 lignes) : structures
//! `serde` des fixtures `FetchResult` (agrégées), options de carte, locales supportées.
//!
//! Fix2 R2 : les champs sont typés [`JsVal`] (sémantique V8 — `undefined`/`null`/
//! booléen/number f64/string) au lieu d'une coercion silencieuse en `u64`/`String`.
//! Le legacy n'a aucun typage à l'exécution : `formatNumber(null)` → `"0"`,
//! champ absent → `"NaN"`, `${currentStreak}` sur `1e21` → `"1e+21"`, etc.
//! Oracle : `tests/golden/meta/counters-extract.json`.

use crate::jsval::JsVal;
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
///
/// Invariant statique (R3) : appelé uniquement sur les 20 indicatifs `A-Z` de
/// `LOCALE_ENTRIES` (données incluses dans le binaire, jamais sur donnée de
/// requête). Hors domaine, `charCodeAt` UTF-16 et l'itération `char` Rust
/// divergeraient et le legacy lèverait une RangeError (`fromCodePoint`) —
/// l'`expect` ci-dessous est l'équivalent capturable de cette RangeError.
pub fn country_to_flag(country: &str) -> String {
    country
        .chars()
        .map(|c| {
            char::from_u32(0x1f1e6 - 65 + u32::from(c)).expect("lettre A-Z (invariant statique)")
        })
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
///
/// Invariant (R4) : `locale` ∈ `LOCALE_CODES` — `parseCardOptions` (zod, ligne
/// `LOCALE_CODES.includes(v) ? v : 'en'`) rejette tout autre code vers `'en'`.
/// Le repli de [`crate::intl::format_js_number`] n'est donc jamais atteint via l'API.
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

/// `GitHubStats` (types.ts) — compteurs = `JsVal` (fix2 R2).
#[derive(Debug, Clone)]
pub struct GitHubStats {
    pub username: JsVal,
    pub name: JsVal,
    pub total_stars: JsVal,
    pub total_forks: JsVal,
    pub total_commits: JsVal,
    pub total_prs: JsVal,
    pub total_issues: JsVal,
    pub contributed_to: JsVal,
}

/// `StreakData` (types.ts).
#[derive(Debug, Clone)]
pub struct StreakData {
    pub username: JsVal,
    pub total_contributions: JsVal,
    pub current_streak: JsVal,
    pub longest_streak: JsVal,
    pub current_streak_start: JsVal,
    pub current_streak_end: JsVal,
    pub longest_streak_start: JsVal,
    pub longest_streak_end: JsVal,
}

/// `LanguageData` (types.ts).
#[derive(Debug, Clone)]
pub struct LanguageData {
    pub name: JsVal,
    pub percentage: JsVal,
    pub color: JsVal,
    pub size: JsVal,
}

/// `TopLangsData` (types.ts).
#[derive(Debug, Clone)]
pub struct TopLangsData {
    pub username: JsVal,
    pub languages: Vec<LanguageData>,
}

/// `ContributionDay` (types.ts).
#[derive(Debug, Clone)]
pub struct ContributionDay {
    pub date: JsVal,
    pub count: JsVal,
}

/// `ProfileData` (types.ts).
#[derive(Debug, Clone)]
pub struct ProfileData {
    pub username: JsVal,
    pub name: JsVal,
    pub contributions_this_year: JsVal,
    pub public_repos: JsVal,
    pub created_at: JsVal,
    pub email: JsVal,
    pub contribution_calendar: Vec<ContributionDay>,
}

// ---------------------------------------------------------------- fixtures (JSON)

fn arr_field(v: &Value, k: &str) -> Vec<ContributionDay> {
    v.get(k)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|d| ContributionDay {
                    date: JsVal::field(d, "date"),
                    count: JsVal::field(d, "count"),
                })
                .collect()
        })
        .unwrap_or_default()
}

impl GitHubStats {
    pub fn from_value(v: &Value) -> GitHubStats {
        GitHubStats {
            username: JsVal::field(v, "username"),
            name: JsVal::field(v, "name"),
            total_stars: JsVal::field(v, "totalStars"),
            total_forks: JsVal::field(v, "totalForks"),
            total_commits: JsVal::field(v, "totalCommits"),
            total_prs: JsVal::field(v, "totalPRs"),
            total_issues: JsVal::field(v, "totalIssues"),
            contributed_to: JsVal::field(v, "contributedTo"),
        }
    }
}

impl StreakData {
    pub fn from_value(v: &Value) -> StreakData {
        StreakData {
            username: JsVal::field(v, "username"),
            total_contributions: JsVal::field(v, "totalContributions"),
            current_streak: JsVal::field(v, "currentStreak"),
            longest_streak: JsVal::field(v, "longestStreak"),
            current_streak_start: JsVal::field(v, "currentStreakStart"),
            current_streak_end: JsVal::field(v, "currentStreakEnd"),
            longest_streak_start: JsVal::field(v, "longestStreakStart"),
            longest_streak_end: JsVal::field(v, "longestStreakEnd"),
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
                        name: JsVal::field(l, "name"),
                        percentage: JsVal::field(l, "percentage"),
                        color: JsVal::field(l, "color"),
                        size: JsVal::field(l, "size"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        TopLangsData {
            username: JsVal::field(v, "username"),
            languages: langs,
        }
    }
}

impl ProfileData {
    pub fn from_value(v: &Value) -> ProfileData {
        ProfileData {
            username: JsVal::field(v, "username"),
            name: JsVal::field(v, "name"),
            contributions_this_year: JsVal::field(v, "contributionsThisYear"),
            public_repos: JsVal::field(v, "publicRepos"),
            created_at: JsVal::field(v, "createdAt"),
            // `if (data.email)` JS : truthy uniquement (null/''/absent → pas de ligne)
            email: JsVal::field(v, "email"),
            contribution_calendar: arr_field(v, "contributionCalendar"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    #[test]
    fn champs_absents_null_et_hors_domaine() {
        // R2 : plus aucune coercion silencieuse — les valeurs restent JsVal.
        let v = json!({"username": "u", "totalStars": "42", "contributedTo": null});
        let s = GitHubStats::from_value(&v);
        assert_eq!(s.total_stars, JsVal::Str("42".into()));
        assert_eq!(s.contributed_to, JsVal::Null);
        assert_eq!(s.total_commits, JsVal::Undefined); // champ absent
        let p = ProfileData::from_value(&json!({"createdAt": 1735689600000u64}));
        assert_eq!(p.created_at, JsVal::Number(1735689600000.0));
        assert_eq!(p.email, JsVal::Undefined);
    }
}
