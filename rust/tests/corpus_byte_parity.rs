//! Porte G2 — parité À L'OCTET du renderer P2 contre le corpus golden généré par
//! le legacy Node v26.13.1 (`scripts/gen-golden.mjs`) :
//! chaque entrée du manifeste (fixtures × cartes × options, puis l'extension
//! « 1 fixture × carte stats × 35 thèmes ») est rendue par le port Rust avec
//! - le compteur global d'uid réglé sur `entry.uid` (uidRule du manifeste) ;
//! - l'horloge injectée au 2026-11-24 UTC (date de génération du corpus).
//!
//! En cas d'écart : chemin, uid, premier offset différent et contexte ±120 caractères.

use gitcard_p0::cards::base::set_card_id_counter;
use gitcard_p0::cards::profile;
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

/// Contexte d'un écart : ±120 caractères autour du premier octet différent.
fn context_at(a: &str, b: &str, offset: usize) -> String {
    let char_of = |s: &str, o: usize| -> (usize, usize) {
        let mut start = o.min(s.len());
        while start > 0 && !s.is_char_boundary(start) {
            start -= 1;
        }
        let mut end = start;
        for _ in 0..120 {
            let mut next = (end + 1).min(s.len());
            while next < s.len() && !s.is_char_boundary(next) {
                next += 1;
            }
            if next == end {
                break;
            }
            end = next;
        }
        let mut begin = start;
        for _ in 0..120 {
            let mut prev = begin.saturating_sub(1);
            while prev > 0 && !s.is_char_boundary(prev) {
                prev -= 1;
            }
            if prev == begin {
                break;
            }
            begin = prev;
        }
        (begin, end)
    };
    let (ab, ae) = char_of(a, offset);
    let (bb, be) = char_of(b, offset);
    format!("attendu: {:?}\nrendu:   {:?}", &a[ab..ae], &b[bb..be])
}

#[test]
fn corpus_byte_parity_g2() {
    // Corpus figé au 2026-11-24 UTC (génération du manifeste) — horloge injectée.
    profile::set_clock(2026, 11);

    let manifest = read_json(&golden().join("manifest.json"));
    let entries = manifest["entries"].as_array().expect("entries[]");
    let mut identical = 0u32;
    let mut different = 0u32;
    let mut themes_added = 0u32;
    let mut failures: Vec<String> = Vec::new();

    // fixtures pré-décodées (7 + réutilisées par l'extension thèmes)
    let mut fixture_cache: std::collections::HashMap<String, Value> =
        std::collections::HashMap::new();

    for entry in entries {
        let file = entry["file"].as_str().expect("file");
        let card = entry["card"].as_str().expect("card");
        let fixture_id = entry["fixture"].as_str().expect("fixture");
        let uid = entry["uid"].as_u64().expect("uid");
        let label = entry["label"].as_str().expect("label");
        let options = CardOptions::from_value(&entry["options"]);
        if label.starts_with("x-theme-") {
            themes_added += 1;
        }

        let fixture = fixture_cache
            .entry(fixture_id.to_string())
            .or_insert_with(|| {
                read_json(&golden().join("fixtures").join(format!("{fixture_id}.json")))
            });

        let data_key = if card == "langs" { "langs" } else { card };
        let data = fixture
            .get(data_key)
            .unwrap_or_else(|| panic!("fixture {fixture_id}: clé {data_key} absente"));

        // uidRule : un createCardWrapper par rendu, compteur réglé par entrée
        set_card_id_counter(uid);

        let rendered = match card {
            "stats" => render_stats_card(&GitHubStats::from_value(data), &options)
                .unwrap_or_else(|e| panic!("{file}: le fixture typé doit rendre : {e}")),
            "streak" => render_streak_card(&StreakData::from_value(data), &options)
                .unwrap_or_else(|e| panic!("{file}: le fixture typé doit rendre : {e}")),
            "langs" => render_langs_card(&TopLangsData::from_value(data), &options)
                .unwrap_or_else(|e| panic!("{file}: le fixture typé doit rendre : {e}")),
            "profile" => render_profile_card(&ProfileData::from_value(data), &options)
                .unwrap_or_else(|e| panic!("{file}: le fixture typé doit rendre : {e}")),
            other => panic!("carte inconnue dans le manifeste: {other}"),
        };

        let expected =
            fs::read_to_string(golden().join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));

        let md5_ok = md5_hex(&rendered) == entry["md5"].as_str().expect("md5");
        if rendered == expected && md5_ok {
            identical += 1;
        } else {
            different += 1;
            let offset = rendered
                .bytes()
                .zip(expected.bytes())
                .position(|(r, e)| r != e)
                .unwrap_or(rendered.len().min(expected.len()));
            failures.push(format!(
                "ÉCART {file} (uid={uid}, label={label}, carte={card}, fixture={fixture_id})\n  longueur attendue={} rendue={} offset={offset} md5_ok={md5_ok}\n  {}",
                expected.len(),
                rendered.len(),
                context_at(&expected, &rendered, offset)
            ));
        }
    }

    let total = entries.len();
    for f in failures.iter().take(20) {
        println!("{f}");
    }
    if failures.len() > 20 {
        println!("… (+{} autres écarts)", failures.len() - 20);
    }
    println!(
        "corpus G2 : {identical} identiques / {different} différents / {total} total (extension thèmes : {themes_added} SVG)"
    );

    profile::clear_clock();
    assert_eq!(
        identical as usize + different as usize,
        total,
        "toutes les entrées traitées"
    );
    assert_eq!(different, 0, "SVG non identiques au corpus golden");
    assert!(
        themes_added >= 35,
        "extension 35 thèmes attendue, trouvé {themes_added}"
    );
}
