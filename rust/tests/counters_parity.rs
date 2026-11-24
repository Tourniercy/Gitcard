//! Porte fix2 R2 + R4 — compteurs/champs texte vs oracle legacy, invariant locale.
//!
//! Oracle : `tests/golden/meta/counters-extract.json` produit par le code legacy
//! EXÉCUTÉ (node:26.13.1-alpine, `--network none`) : pour chaque carte et chaque
//! champ numérique/texte déclaré dans `types.ts`, les valeurs
//! { absent, null, 0, -5, 12.5, 9007199254740993, 1e21, "42", true } (numériques)
//! et { absent, null, "", 0, accentué, idéogrammes, emoji } (texte) ; md5 du SVG
//! legacy ou `err:<Type>` quand le legacy lève (TypeError encodeHTML/toFixed).

use gitcard_p0::cards::base::set_card_id_counter;
use gitcard_p0::cards::{
    render_langs_card, render_profile_card, render_stats_card, render_streak_card,
};
use gitcard_p0::md5_hex;
use gitcard_p0::types::{
    locale_codes, CardOptions, GitHubStats, ProfileData, StreakData, TopLangsData,
};
use gitcard_p0::RenderError;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

fn golden() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/golden")
}

fn read_json(path: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("fichier golden illisible"))
        .expect("json golden invalide")
}

fn value_of(v: &Value) -> Option<Value> {
    match v {
        Value::Object(o) if o.len() == 1 && o.contains_key("$absent") => None,
        other => Some(other.clone()),
    }
}

fn head_mut<'a>(root: &'a mut Value, head: &str) -> &'a mut Value {
    match head.split_once('[') {
        Some((k, idx)) => {
            let i: usize = idx.trim_end_matches(']').parse().expect("index");
            let arr = root
                .as_object_mut()
                .expect("objet")
                .entry(k.to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            &mut arr.as_array_mut().expect("tableau")[i]
        }
        None => root
            .as_object_mut()
            .expect("objet")
            .entry(head.to_string())
            .or_insert(Value::Null),
    }
}

fn set_path(root: &mut Value, path: &str, value: Option<Value>) {
    if let Some(star) = path.find("[*]") {
        let head = &path[..star];
        let rest = path[star + 3..].trim_start_matches('.');
        let arr = if head.is_empty() {
            root
        } else {
            head_mut(root, head)
        };
        for el in arr.as_array_mut().expect("tableau") {
            set_path(el, rest, value.clone());
        }
        return;
    }
    match path.split_once('.') {
        None => match value {
            Some(v) => {
                root.as_object_mut()
                    .expect("objet")
                    .insert(path.to_string(), v);
            }
            None => {
                root.as_object_mut().expect("objet").remove(path);
            }
        },
        Some((head, tail)) => {
            let child = head_mut(root, head);
            set_path(child, tail, value);
        }
    }
}

fn check(result: Result<String, RenderError>, expected: &Value, ctx: &str) {
    if let Some(err_kind) = expected["err"].as_str() {
        let err = result
            .err()
            .unwrap_or_else(|| panic!("{ctx} : le legacy LÈVE {err_kind}, le Rust a rendu"));
        match (err_kind, &err) {
            ("RangeError", RenderError::RangeError(_))
            | ("TypeError", RenderError::TypeError(_)) => {}
            (k, e) => panic!("{ctx} : attendu {k}, obtenu {e}"),
        }
    } else {
        let md5 = expected["md5"]
            .as_str()
            .unwrap_or_else(|| panic!("{ctx} : entrée oracle mal formée"));
        let svg = result.unwrap_or_else(|e| panic!("{ctx} : legacy OK, le Rust échoue : {e}"));
        assert_eq!(md5_hex(&svg), md5, "{ctx} : md5 divergent de l'oracle");
    }
}

/// Champs déclarés dans `packages/svg-renderer/src/types.ts` (liste exacte).
const NUMERIC_FIELDS: &[(&str, &[&str])] = &[
    (
        "stats",
        &[
            "totalStars",
            "totalForks",
            "totalCommits",
            "totalPRs",
            "totalIssues",
            "contributedTo",
        ],
    ),
    (
        "streak",
        &["totalContributions", "currentStreak", "longestStreak"],
    ),
    ("langs", &["languages[0].percentage", "languages[0].size"]),
    (
        "profile",
        &[
            "contributionsThisYear",
            "publicRepos",
            "contributionCalendar[*].count",
        ],
    ),
];
const TEXT_FIELDS: &[(&str, &[&str])] = &[
    ("stats", &["username", "name"]),
    (
        "streak",
        &[
            "username",
            "currentStreakStart",
            "currentStreakEnd",
            "longestStreakStart",
            "longestStreakEnd",
        ],
    ),
    (
        "langs",
        &["username", "languages[0].name", "languages[0].color"],
    ),
    (
        "profile",
        &[
            "username",
            "name",
            "createdAt",
            "email",
            "contributionCalendar[*].date",
        ],
    ),
];

#[test]
fn r2_compteurs_et_texte_parity_with_legacy_oracle() {
    let ex = read_json(&golden().join("meta/counters-extract.json"));
    assert_eq!(ex["kind"].as_str(), Some("counters-extract"));
    let now = ex["now"].as_str().expect("now YYYY-MM");
    let (y, m) = now.split_once('-').expect("YYYY-MM");
    gitcard_p0::cards::profile::set_clock(y.parse().expect("année"), m.parse().expect("mois"));

    // la matrice de l'oracle couvre bien TOUTE la liste types.ts (pin du brief)
    for (kind, expected) in [("number", NUMERIC_FIELDS), ("text", TEXT_FIELDS)] {
        let key = if kind == "number" {
            "numericFields"
        } else {
            "textFields"
        };
        for (card, fields) in expected {
            let got: Vec<&str> = ex[key][card]
                .as_array()
                .unwrap_or_else(|| panic!("{key}.{card} absent de l'oracle"))
                .iter()
                .map(|v| v.as_str().expect("champ string"))
                .collect();
            assert_eq!(&got, fields, "{key}.{card} : matrice oracle ≠ types.ts");
        }
    }
    let n_num: usize = NUMERIC_FIELDS.iter().map(|(_, f)| f.len()).sum();
    let n_text: usize = TEXT_FIELDS.iter().map(|(_, f)| f.len()).sum();
    let n_values_num = ex["numericValues"].as_array().unwrap().len();
    let n_values_text = ex["textValues"].as_array().unwrap().len();
    assert_eq!(n_values_num, 9, "valeurs numériques du brief");
    assert_eq!(n_values_text, 7, "valeurs texte du brief");
    assert_eq!(
        ex["entries"].as_array().unwrap().len(),
        n_num * n_values_num + n_text * n_values_text,
        "matrice compteurs incomplète"
    );

    let typical = read_json(&golden().join("fixtures/typical.json"));
    let options = CardOptions::from_value(&json!({}));

    for (i, e) in ex["entries"].as_array().unwrap().iter().enumerate() {
        let ctx = format!(
            "entries[{i}] {}.{}",
            e["card"].as_str().unwrap(),
            e["field"].as_str().unwrap()
        );
        let card = e["card"].as_str().expect("card");
        let field = e["field"].as_str().expect("field");
        let mut data = if card == "langs" {
            typical["langs"].clone()
        } else {
            typical[card].clone()
        };
        set_path(&mut data, field, value_of(&e["value"]));
        set_card_id_counter(e["u"].as_u64().unwrap_or_else(|| panic!("{ctx} : uid")));
        let result = match card {
            "stats" => render_stats_card(&GitHubStats::from_value(&data), &options),
            "streak" => render_streak_card(&StreakData::from_value(&data), &options),
            "langs" => render_langs_card(&TopLangsData::from_value(&data), &options),
            "profile" => render_profile_card(&ProfileData::from_value(&data), &options),
            other => panic!("carte inconnue : {other}"),
        };
        check(result, e, &ctx);
    }
    gitcard_p0::cards::profile::clear_clock();
    println!(
        "R2 : {} rendus comparés à l'oracle compteurs ({} champs × {} valeurs numériques + {} champs × {} valeurs texte)",
        n_num * n_values_num + n_text * n_values_text,
        n_num,
        n_values_num,
        n_text,
        n_values_text
    );
}

/// R4 : l'invariant « locale ∈ LOCALE_CODES imposée par parseCardOptions ».
/// Le zod legacy transforme tout code hors LOCALE_CODES en 'en' ; le port Rust
/// reconstruit les options depuis le manifeste golden — on épingle donc que (1)
/// toutes les locales du manifeste sont dans la table, (2) le repli hors table
/// vaut la locale par défaut du runtime (oracle `Intl.NumberFormat('xx')`).
#[test]
fn r4_locales_du_manifeste_dans_la_table_et_repli_oracle() {
    let manifest = read_json(&golden().join("manifest.json"));
    let codes = locale_codes();
    let mut seen = std::collections::BTreeSet::new();
    for e in manifest["entries"].as_array().expect("entries[]") {
        let loc = e["options"]["locale"].as_str().expect("locale");
        assert!(
            codes.contains(&loc),
            "option locale « {loc} » hors LOCALE_CODES (l'API legacy l'impose)"
        );
        seen.insert(loc.to_string());
    }
    assert!(
        seen.contains("en") && seen.len() >= 5,
        "locales du manifeste : {seen:?}"
    );

    let tables = read_json(&golden().join("intl-tables.json"));
    for (bad, expected) in tables["localeFallback"]
        .as_object()
        .expect("localeFallback")
    {
        assert_eq!(
            gitcard_p0::intl::format_js_number(1_234_567.0, bad),
            expected.as_str().expect("string"),
            "repli locale {bad} ≠ oracle Intl"
        );
    }
}
