//! P3 — oracles du brief (« legacy exécuté », `scripts/gen-graphql.mjs`) :
//! 1. agrégations : chaque fixture GraphQL → `extract_all` → JSON du port,
//!    comparaison D'OCTETS avec `<fixture>.expected.json` (le `JSON.stringify`
//!    du legacy, Node v26.13.1) ;
//! 2. sweep du sérialiseur de nombres : 2000 valeurs (mélange fixe) — littéral
//!    d'entrée → `number_as_f64` → `JsJson::Num` → `to_json_string` == attendu ;
//! 3. bonus de confiance : rendu SVG des 4 cartes par fixture, md5 ==
//!    `svg-md5.json` (uid du générateur réglé par carte via
//!    `set_card_id_counter`, horloge figée au mois de génération comme le
//!    corpus, options par défaut = `parseCardOptions({})`).

use gitcard_p0::cards::base::set_card_id_counter;
use gitcard_p0::cards::profile;
use gitcard_p0::github::extract_all;
use gitcard_p0::js_json::JsJson;
use gitcard_p0::jsval::number_as_f64;
use gitcard_p0::types::CardOptions;
use gitcard_p0::{
    md5_hex, render_langs_card, render_profile_card, render_stats_card, render_streak_card,
    GitHubStats, ProfileData, StreakData, TopLangsData,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn golden() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/golden")
}

fn read_json(path: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("fichier golden illisible"))
        .expect("json golden invalide")
}

/// Fixtures triées (même ordre que `readdirSync().sort()` du générateur).
fn fixture_ids() -> Vec<String> {
    let dir = golden().join("graphql/fixtures");
    let mut ids: Vec<String> = fs::read_dir(&dir)
        .expect("dir fixtures")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".json"))
        .map(|n| n.trim_end_matches(".json").to_string())
        .collect();
    ids.sort();
    ids
}

fn expected_json(id: &str) -> String {
    fs::read_to_string(golden().join("graphql").join(format!("{id}.expected.json")))
        .unwrap_or_else(|e| panic!("{id}: attendu illisible : {e}"))
}

/// 1. Agrégations → JSON de cache : octets == `JSON.stringify` du legacy.
#[test]
fn oracles_agregations_parite_bytes() {
    let ids = fixture_ids();
    assert_eq!(ids.len(), 11, "11 fixtures graphql attendues");
    let mut identical = 0u32;
    let mut failures: Vec<String> = Vec::new();
    for id in &ids {
        let fixture = read_json(&golden().join("graphql/fixtures").join(format!("{id}.json")));
        let user = &fixture["data"]["user"];
        let login = user["login"].as_str().expect("login");
        let result = extract_all(user, login);
        let got = result.to_json_string();
        let expected = expected_json(id);
        if got == expected {
            identical += 1;
        } else {
            let offset = got
                .bytes()
                .zip(expected.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(got.len().min(expected.len()));
            failures.push(format!(
                "ÉCART {id} (offset {offset})\n  attendu: {expected:?}\n  rendu:   {got:?}"
            ));
        }
    }
    println!(
        "oracles agrégations : {identical}/{} identiques (octets)",
        ids.len()
    );
    for f in failures.iter().take(5) {
        println!("{f}");
    }
    assert_eq!(
        identical as usize,
        ids.len(),
        "agrégations ≠ attendus legacy"
    );
}

/// 2. Sweep du sérialiseur de nombres : 2000 valeurs contre `JSON.stringify`.
#[test]
fn oracles_number_sweep_2000() {
    let sweep = read_json(&golden().join("graphql/number-sweep.json"));
    let entries = sweep.as_array().expect("sweep[]");
    assert_eq!(entries.len(), 2000, "2000 valeurs attendues");
    let mut identical = 0u32;
    let mut failures: Vec<String> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        let input = e["input"].as_str().expect("input");
        let expected = e["expected"].as_str().expect("expected");
        let v: Value = serde_json::from_str(input)
            .unwrap_or_else(|err| panic!("sweep[{i}] : littéral {input} illisible : {err}"));
        let Value::Number(n) = &v else {
            panic!("sweep[{i}] : pas un nombre : {v}");
        };
        let got = JsJson::Num(number_as_f64(n)).to_json_string();
        if got == expected {
            identical += 1;
        } else {
            failures.push(format!(
                "ÉCART sweep[{i}] input={input} : attendu={expected} rendu={got}"
            ));
        }
    }
    println!("sweep nombres : {identical}/2000 identiques");
    for f in failures.iter().take(10) {
        println!("{f}");
    }
    assert_eq!(
        identical, 2000,
        "sérialiseur ≠ JSON.stringify Node v26.13.1"
    );
}

/// Bonus SVG : rendu des 4 cartes par fixture — uid réglé par carte d'après
/// `svg-md5.json` via `set_card_id_counter`, options par défaut =
/// `parseCardOptions({})`, horloge figée comme la génération du corpus ;
/// comparaison des md5 avec le legacy.
#[test]
fn svg_bonus_11_fixtures_x4_cartes_md5() {
    profile::set_clock(2026, 11);
    let options = CardOptions::from_value(&serde_json::json!({}));
    let svg_md5 = read_json(&golden().join("graphql/svg-md5.json"));
    let ids = fixture_ids();
    let mut identical = 0u32;
    let mut total = 0u32;
    let mut failures: Vec<String> = Vec::new();
    for id in &ids {
        let fixture = read_json(&golden().join("graphql/fixtures").join(format!("{id}.json")));
        let user = &fixture["data"]["user"];
        let login = user["login"].as_str().expect("login");
        // Le FetchResult re-passe par le JSON de cache (phase 4), comme en prod.
        let cache_json = extract_all(user, login).to_json_string();
        let v: Value = serde_json::from_str(&cache_json).expect("cache json valide");
        let per = &svg_md5[id];
        for (card, data) in [
            ("stats", &v["stats"]),
            ("streak", &v["streak"]),
            ("langs", &v["languages"]),
            ("profile", &v["profile"]),
        ] {
            total += 1;
            let uid = per[card]["uid"].as_u64().expect("uid");
            let expected_md5 = per[card]["md5"].as_str().expect("md5");
            set_card_id_counter(uid);
            let rendered = match card {
                "stats" => render_stats_card(&GitHubStats::from_value(data), &options),
                "streak" => render_streak_card(&StreakData::from_value(data), &options),
                "langs" => render_langs_card(
                    &TopLangsData::from_value(data).expect("langs valide"),
                    &options,
                ),
                "profile" => render_profile_card(
                    &ProfileData::from_value(data).expect("profile valide"),
                    &options,
                ),
                _ => unreachable!("carte inconnue"),
            }
            .expect("le rendu doit réussir");
            if md5_hex(&rendered) == expected_md5 {
                identical += 1;
            } else {
                failures.push(format!(
                    "ÉCART {id}/{card} (uid={uid}) : md5 rendu={} attendu={expected_md5}",
                    md5_hex(&rendered)
                ));
            }
        }
    }
    profile::clear_clock();
    println!(
        "bonus SVG : {identical}/{total} md5 identiques (uid 0..{})",
        total - 1
    );
    for f in failures.iter().take(10) {
        println!("{f}");
    }
    assert_eq!(total, ids.len() as u32 * 4, "4 cartes par fixture");
    assert_eq!(identical, total, "SVG bonus ≠ md5 legacy");
}
