//! Port littéral de `packages/svg-renderer/src/themes/themes.ts` (402 lignes) +
//! `themes/index.ts` (138 lignes) : 35 thèmes dans l'ordre exact d'insertion de
//! l'objet `themes` (= `Object.keys(themes)`, oracle Node v26.13.1), groupes,
//! `getTheme` avec repli `default`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub background: &'static str,
    pub border: &'static str,
    pub title: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub icon: &'static str,
    pub ring: &'static str,
}

/// Les 35 thèmes, DANS L'ORDRE EXACT d'insertion de `themes/index.ts`
/// (verifié contre l'oracle Node : tests/golden/meta/themes-extract.json).
pub const THEMES: [Theme; 35] = [
    Theme {
        name: "default",
        background: "rgba(255, 255, 255, 0.7)",
        border: "rgba(0, 0, 0, 0.1)",
        title: "#1f2937",
        text: "#374151",
        muted: "#6b7280",
        icon: "#3b82f6",
        ring: "#3b82f6",
    },
    Theme {
        name: "flat",
        background: "rgba(255, 255, 255, 1)",
        border: "#e5e7eb",
        title: "#111827",
        text: "#374151",
        muted: "#9ca3af",
        icon: "#6366f1",
        ring: "#6366f1",
    },
    Theme {
        name: "breezy",
        background: "rgba(240, 249, 255, 0.8)",
        border: "rgba(56, 189, 248, 0.2)",
        title: "#0c4a6e",
        text: "#075985",
        muted: "#7dd3fc",
        icon: "#0ea5e9",
        ring: "#0ea5e9",
    },
    Theme {
        name: "rose",
        background: "rgba(255, 241, 242, 0.8)",
        border: "rgba(244, 63, 94, 0.15)",
        title: "#881337",
        text: "#9f1239",
        muted: "#fda4af",
        icon: "#f43f5e",
        ring: "#f43f5e",
    },
    Theme {
        name: "mint",
        background: "rgba(236, 253, 245, 0.8)",
        border: "rgba(16, 185, 129, 0.15)",
        title: "#064e3b",
        text: "#065f46",
        muted: "#6ee7b7",
        icon: "#10b981",
        ring: "#10b981",
    },
    Theme {
        name: "sand",
        background: "rgba(255, 251, 235, 0.8)",
        border: "rgba(217, 119, 6, 0.15)",
        title: "#78350f",
        text: "#92400e",
        muted: "#fcd34d",
        icon: "#d97706",
        ring: "#d97706",
    },
    Theme {
        name: "lavender",
        background: "rgba(245, 243, 255, 0.8)",
        border: "rgba(139, 92, 246, 0.15)",
        title: "#4c1d95",
        text: "#5b21b6",
        muted: "#c4b5fd",
        icon: "#8b5cf6",
        ring: "#8b5cf6",
    },
    Theme {
        name: "dark",
        background: "rgba(13, 17, 23, 0.8)",
        border: "rgba(255, 255, 255, 0.1)",
        title: "#e6edf3",
        text: "#c9d1d9",
        muted: "#8b949e",
        icon: "#58a6ff",
        ring: "#58a6ff",
    },
    Theme {
        name: "dim",
        background: "rgba(34, 39, 46, 0.85)",
        border: "rgba(68, 76, 86, 0.6)",
        title: "#adbac7",
        text: "#768390",
        muted: "#545d68",
        icon: "#539bf5",
        ring: "#539bf5",
    },
    Theme {
        name: "midnight",
        background: "rgba(15, 23, 42, 0.85)",
        border: "rgba(51, 65, 85, 0.5)",
        title: "#f1f5f9",
        text: "#cbd5e1",
        muted: "#64748b",
        icon: "#818cf8",
        ring: "#818cf8",
    },
    Theme {
        name: "onyx",
        background: "rgba(0, 0, 0, 0.9)",
        border: "rgba(255, 255, 255, 0.08)",
        title: "#ffffff",
        text: "#a1a1aa",
        muted: "#52525b",
        icon: "#a78bfa",
        ring: "#a78bfa",
    },
    Theme {
        name: "dracula",
        background: "rgba(40, 42, 54, 0.8)",
        border: "rgba(98, 114, 164, 0.3)",
        title: "#f8f8f2",
        text: "#f8f8f2",
        muted: "#6272a4",
        icon: "#bd93f9",
        ring: "#bd93f9",
    },
    Theme {
        name: "monokai",
        background: "rgba(39, 40, 34, 0.85)",
        border: "rgba(117, 113, 94, 0.3)",
        title: "#f8f8f2",
        text: "#f8f8f2",
        muted: "#75715e",
        icon: "#a6e22e",
        ring: "#a6e22e",
    },
    Theme {
        name: "nord",
        background: "rgba(46, 52, 64, 0.85)",
        border: "rgba(76, 86, 106, 0.4)",
        title: "#eceff4",
        text: "#d8dee9",
        muted: "#4c566a",
        icon: "#88c0d0",
        ring: "#88c0d0",
    },
    Theme {
        name: "solarized",
        background: "rgba(253, 246, 227, 0.85)",
        border: "rgba(147, 161, 161, 0.2)",
        title: "#073642",
        text: "#586e75",
        muted: "#93a1a1",
        icon: "#268bd2",
        ring: "#268bd2",
    },
    Theme {
        name: "solarized-dark",
        background: "rgba(0, 43, 54, 0.85)",
        border: "rgba(88, 110, 117, 0.3)",
        title: "#eee8d5",
        text: "#839496",
        muted: "#586e75",
        icon: "#268bd2",
        ring: "#268bd2",
    },
    Theme {
        name: "gruvbox",
        background: "rgba(40, 40, 40, 0.85)",
        border: "rgba(168, 153, 132, 0.3)",
        title: "#ebdbb2",
        text: "#d5c4a1",
        muted: "#a89984",
        icon: "#fabd2f",
        ring: "#fabd2f",
    },
    Theme {
        name: "gruvbox-light",
        background: "rgba(251, 241, 199, 0.85)",
        border: "rgba(168, 153, 132, 0.2)",
        title: "#3c3836",
        text: "#504945",
        muted: "#928374",
        icon: "#d79921",
        ring: "#d79921",
    },
    Theme {
        name: "tokyo-night",
        background: "rgba(26, 27, 38, 0.85)",
        border: "rgba(41, 46, 66, 0.5)",
        title: "#c0caf5",
        text: "#a9b1d6",
        muted: "#565f89",
        icon: "#7aa2f7",
        ring: "#7aa2f7",
    },
    Theme {
        name: "one-dark",
        background: "rgba(40, 44, 52, 0.85)",
        border: "rgba(76, 82, 99, 0.4)",
        title: "#abb2bf",
        text: "#abb2bf",
        muted: "#5c6370",
        icon: "#61afef",
        ring: "#61afef",
    },
    Theme {
        name: "one-light",
        background: "rgba(250, 250, 250, 0.85)",
        border: "rgba(56, 58, 66, 0.1)",
        title: "#383a42",
        text: "#383a42",
        muted: "#a0a1a7",
        icon: "#4078f2",
        ring: "#4078f2",
    },
    Theme {
        name: "catppuccin-mocha",
        background: "rgba(30, 30, 46, 0.85)",
        border: "rgba(69, 71, 90, 0.5)",
        title: "#cdd6f4",
        text: "#bac2de",
        muted: "#6c7086",
        icon: "#cba6f7",
        ring: "#cba6f7",
    },
    Theme {
        name: "catppuccin-latte",
        background: "rgba(239, 241, 245, 0.85)",
        border: "rgba(172, 176, 190, 0.3)",
        title: "#4c4f69",
        text: "#5c5f77",
        muted: "#9ca0b0",
        icon: "#8839ef",
        ring: "#8839ef",
    },
    Theme {
        name: "synthwave",
        background: "rgba(36, 18, 59, 0.9)",
        border: "rgba(148, 80, 199, 0.3)",
        title: "#f97e72",
        text: "#e8e3e3",
        muted: "#848bbd",
        icon: "#ff7edb",
        ring: "#ff7edb",
    },
    Theme {
        name: "cobalt",
        background: "rgba(0, 28, 54, 0.9)",
        border: "rgba(25, 66, 113, 0.5)",
        title: "#ffffff",
        text: "#e1efff",
        muted: "#7590ab",
        icon: "#ffc600",
        ring: "#ffc600",
    },
    Theme {
        name: "night-owl",
        background: "rgba(1, 22, 39, 0.9)",
        border: "rgba(23, 62, 88, 0.5)",
        title: "#d6deeb",
        text: "#d6deeb",
        muted: "#637777",
        icon: "#7fdbca",
        ring: "#7fdbca",
    },
    Theme {
        name: "aurora",
        background: "rgba(7, 15, 43, 0.9)",
        border: "rgba(76, 201, 240, 0.2)",
        title: "#f0f0f0",
        text: "#c8d6e5",
        muted: "#576574",
        icon: "#48dbfb",
        ring: "#48dbfb",
    },
    Theme {
        name: "ocean",
        background: "rgba(3, 28, 59, 0.9)",
        border: "rgba(0, 105, 148, 0.3)",
        title: "#e0f7fa",
        text: "#b2ebf2",
        muted: "#4db6ac",
        icon: "#00bcd4",
        ring: "#00bcd4",
    },
    Theme {
        name: "forest",
        background: "rgba(16, 36, 24, 0.9)",
        border: "rgba(46, 125, 50, 0.3)",
        title: "#c8e6c9",
        text: "#a5d6a7",
        muted: "#66bb6a",
        icon: "#4caf50",
        ring: "#4caf50",
    },
    Theme {
        name: "sunset",
        background: "rgba(54, 17, 31, 0.9)",
        border: "rgba(239, 108, 0, 0.2)",
        title: "#fff3e0",
        text: "#ffe0b2",
        muted: "#ff8a65",
        icon: "#ff6d00",
        ring: "#ff6d00",
    },
    Theme {
        name: "cherry",
        background: "rgba(45, 10, 20, 0.9)",
        border: "rgba(211, 47, 47, 0.25)",
        title: "#fce4ec",
        text: "#f8bbd0",
        muted: "#e57373",
        icon: "#ef5350",
        ring: "#ef5350",
    },
    Theme {
        name: "slate",
        background: "rgba(30, 41, 59, 0.85)",
        border: "rgba(71, 85, 105, 0.4)",
        title: "#f8fafc",
        text: "#e2e8f0",
        muted: "#64748b",
        icon: "#38bdf8",
        ring: "#38bdf8",
    },
    Theme {
        name: "cotton-candy",
        background: "rgba(253, 242, 248, 0.85)",
        border: "rgba(236, 72, 153, 0.15)",
        title: "#831843",
        text: "#9d174d",
        muted: "#f9a8d4",
        icon: "#ec4899",
        ring: "#ec4899",
    },
    Theme {
        name: "high-contrast",
        background: "rgba(0, 0, 0, 1)",
        border: "#ffffff",
        title: "#ffffff",
        text: "#ffffff",
        muted: "#cccccc",
        icon: "#ffff00",
        ring: "#ffff00",
    },
    Theme {
        name: "paper",
        background: "rgba(250, 248, 240, 0.9)",
        border: "rgba(180, 166, 140, 0.25)",
        title: "#433422",
        text: "#5c4b37",
        muted: "#a09080",
        icon: "#8b6914",
        ring: "#8b6914",
    },
];

/// `THEME_NAMES = Object.keys(themes)` — ordre d'insertion, 35 entrées.
pub const THEME_NAMES: [&str; 35] = [
    "default",
    "flat",
    "breezy",
    "rose",
    "mint",
    "sand",
    "lavender",
    "dark",
    "dim",
    "midnight",
    "onyx",
    "dracula",
    "monokai",
    "nord",
    "solarized",
    "solarized-dark",
    "gruvbox",
    "gruvbox-light",
    "tokyo-night",
    "one-dark",
    "one-light",
    "catppuccin-mocha",
    "catppuccin-latte",
    "synthwave",
    "cobalt",
    "night-owl",
    "aurora",
    "ocean",
    "forest",
    "sunset",
    "cherry",
    "slate",
    "cotton-candy",
    "high-contrast",
    "paper",
];

/// `ThemeGroup` (themes/index.ts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeGroup {
    pub label: &'static str,
    pub themes: &'static [&'static str],
}

/// `THEME_GROUPS` — 5 groupes (Light 13, Dark 5, Editor 11, Atmosphere 5,
/// Accessibility 1) dans l'ordre du legacy.
pub const THEME_GROUPS: [ThemeGroup; 5] = [
    ThemeGroup {
        label: "Light",
        themes: &[
            "default",
            "flat",
            "breezy",
            "rose",
            "mint",
            "sand",
            "lavender",
            "solarized",
            "gruvbox-light",
            "one-light",
            "catppuccin-latte",
            "cotton-candy",
            "paper",
        ],
    },
    ThemeGroup {
        label: "Dark",
        themes: &["dark", "dim", "midnight", "onyx", "slate"],
    },
    ThemeGroup {
        label: "Editor",
        themes: &[
            "dracula",
            "monokai",
            "nord",
            "solarized-dark",
            "gruvbox",
            "tokyo-night",
            "one-dark",
            "catppuccin-mocha",
            "synthwave",
            "cobalt",
            "night-owl",
        ],
    },
    ThemeGroup {
        label: "Atmosphere",
        themes: &["aurora", "ocean", "forest", "sunset", "cherry"],
    },
    ThemeGroup {
        label: "Accessibility",
        themes: &["high-contrast"],
    },
];

/// `getTheme(name)` : repli silencieux sur `default` pour tout nom inconnu
/// (y compris casse différente : la propriété d'objet JS est sensible à la casse).
pub fn get_theme(name: &str) -> &'static Theme {
    THEMES.iter().find(|t| t.name == name).unwrap_or(&THEMES[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Oracle Node v26.13.1 (brief P2 §3 + tests/golden/meta/themes-extract.json).
    const ORACLE_NAMES: [&str; 35] = [
        "default",
        "flat",
        "breezy",
        "rose",
        "mint",
        "sand",
        "lavender",
        "dark",
        "dim",
        "midnight",
        "onyx",
        "dracula",
        "monokai",
        "nord",
        "solarized",
        "solarized-dark",
        "gruvbox",
        "gruvbox-light",
        "tokyo-night",
        "one-dark",
        "one-light",
        "catppuccin-mocha",
        "catppuccin-latte",
        "synthwave",
        "cobalt",
        "night-owl",
        "aurora",
        "ocean",
        "forest",
        "sunset",
        "cherry",
        "slate",
        "cotton-candy",
        "high-contrast",
        "paper",
    ];

    #[test]
    fn theme_names_ordre_et_nombre_vs_oracle() {
        assert_eq!(THEME_NAMES.len(), 35);
        assert_eq!(THEMES.len(), 35);
        for (i, name) in THEME_NAMES.iter().enumerate() {
            assert_eq!(*name, ORACLE_NAMES[i], "position {i}");
            assert_eq!(THEMES[i].name, ORACLE_NAMES[i]);
        }
    }

    #[test]
    fn theme_names_vs_extraction_legacy() {
        let v: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../tests/golden/meta/themes-extract.json"
            ))
            .expect("themes-extract.json manquant"),
        )
        .expect("json invalide");
        let names: Vec<&str> = v["themeNames"]
            .as_array()
            .expect("themeNames")
            .iter()
            .map(|x| x.as_str().expect("nom"))
            .collect();
        assert_eq!(names, THEME_NAMES);
    }

    #[test]
    fn theme_groups_vs_legacy() {
        let v: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../tests/golden/meta/themes-extract.json"
            ))
            .expect("themes-extract.json manquant"),
        )
        .expect("json invalide");
        let groups = v["groups"].as_array().expect("groups");
        assert_eq!(groups.len(), 5);
        for (g, legacy) in THEME_GROUPS.iter().zip(groups) {
            assert_eq!(g.label, legacy["label"].as_str().expect("label"));
            let th: Vec<&str> = legacy["themes"]
                .as_array()
                .expect("themes[]")
                .iter()
                .map(|x| x.as_str().expect("thème"))
                .collect();
            assert_eq!(g.themes, th.as_slice(), "groupe {}", g.label);
        }
        let sizes: Vec<usize> = THEME_GROUPS.iter().map(|g| g.themes.len()).collect();
        assert_eq!(sizes, vec![13, 5, 11, 5, 1]);
        // tout thème appartient à exactement un groupe
        let mut total = 0;
        for g in &THEME_GROUPS {
            for t in g.themes {
                total += 1;
                assert!(THEME_NAMES.contains(t), "{t} hors THEME_NAMES");
            }
        }
        assert_eq!(total, 35);
    }

    #[test]
    fn get_theme_repli_et_casse() {
        // nom inconnu → objet identique à getTheme('default') (oracle legacy)
        let unknown = get_theme("pas-un-theme");
        let dflt = get_theme("default");
        assert_eq!(unknown, dflt);
        // casse différente → repli (propriété d'objet JS sensible à la casse)
        assert_eq!(get_theme("Dracula"), dflt);
        assert_eq!(get_theme("DRACULA"), dflt);
        assert_eq!(get_theme(""), dflt);
        // thèmes existants : sensible à la casse, trouvés exacts
        let dracula = get_theme("dracula");
        assert_eq!(dracula.background, "rgba(40, 42, 54, 0.8)");
        assert_eq!(dracula.muted, "#6272a4");
        // clés d'un thème : name, background, border, title, text, muted, icon, ring
        assert_eq!(dracula.name, "dracula");
        assert_eq!(get_theme("high-contrast").border, "#ffffff");
        // extraction legacy : unknownTheme identique à defaultTheme
        let v: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../tests/golden/meta/themes-extract.json"
            ))
            .expect("themes-extract.json manquant"),
        )
        .expect("json invalide");
        assert_eq!(
            v["unknownTheme"]["background"].as_str().expect("bg"),
            unknown.background
        );
        assert_eq!(
            v["caseSensitiveUnknown"]["name"].as_str().expect("name"),
            "default"
        );
    }
}
