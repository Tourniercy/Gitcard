//! Porte fix2 R1 — parité du parsing de dates vs oracle legacy.
//!
//! Oracle : `tests/golden/meta/date-extract.json` produit par le code legacy
//! EXÉCUTÉ (node:26.13.1-alpine, `--network none`) : pour chaque entrée, md5 du
//! SVG legacy de (i) la carte streak (les 4 champs date = input) et (ii) la carte
//! profile (createdAt = input ET toutes les dates du contributionCalendar = input),
//! ou `err:<Type>` quand le legacy lève (RangeError « Invalid time value »).
//! Le port Rust doit produire les mêmes md5 — ou échouer avec le MÊME type
//! d'erreur — sur les DEUX listes : 87 représentatives + 2 000 pseudo-aléatoires.

use gitcard_p0::cards::base::set_card_id_counter;
use gitcard_p0::cards::{profile, render_profile_card, render_streak_card};
use gitcard_p0::jsval::JsVal;
use gitcard_p0::md5_hex;
use gitcard_p0::types::{CardOptions, ProfileData, StreakData};
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

/// `{"$absent": true}` (champ supprimé) → `JsVal::Undefined`.
fn input_of(v: &Value) -> Option<JsVal> {
    match v {
        Value::Object(o) if o.len() == 1 && o.contains_key("$absent") => None,
        other => Some(JsVal::from_json(other)),
    }
}

/// Valeur JSON équivalente pour injection dans un fixture cloné.
fn json_of(v: &JsVal) -> Value {
    match v {
        JsVal::Undefined => Value::Null, // jamais utilisé : absent → suppression
        JsVal::Null => Value::Null,
        JsVal::Bool(b) => Value::Bool(*b),
        JsVal::Number(n) => serde_json::Number::from_f64(*n)
            .map(Value::Number)
            .expect("number JSON"),
        JsVal::Str(s) => Value::String(s.clone()),
        JsVal::Other => Value::Null,
    }
}

fn check(result: Result<String, RenderError>, expected: &Value, ctx: &str, side: &str) {
    let s = expected
        .as_str()
        .unwrap_or_else(|| panic!("{ctx}/{side} : entrée oracle mal formée"));
    if let Some(kind) = s.strip_prefix("err:") {
        let err = result.err().unwrap_or_else(|| {
            panic!("{ctx}/{side} : le legacy LÈVE {kind}, le Rust a rendu un SVG")
        });
        match (kind, &err) {
            ("RangeError", RenderError::RangeError(_))
            | ("TypeError", RenderError::TypeError(_)) => {}
            (k, e) => panic!("{ctx}/{side} : attendu {k}, obtenu {e}"),
        }
    } else {
        let svg =
            result.unwrap_or_else(|e| panic!("{ctx}/{side} : legacy OK, le Rust échoue : {e}"));
        assert_eq!(md5_hex(&svg), s, "{ctx}/{side} : md5 divergent de l'oracle");
    }
}

#[test]
fn r1_dates_parity_with_legacy_oracle() {
    let ex = read_json(&golden().join("meta/date-extract.json"));
    assert_eq!(ex["kind"].as_str(), Some("date-extract"));
    let now = ex["now"].as_str().expect("now YYYY-MM");
    let (y, m) = now.split_once('-').expect("YYYY-MM");
    profile::set_clock(y.parse().expect("année"), m.parse().expect("mois"));

    let typical = read_json(&golden().join("fixtures/typical.json"));
    let options = CardOptions::from_value(&json!({}));

    let mut checked = 0usize;
    let mut representative = 0usize;
    let mut random = 0usize;
    for list in ["representative", "random"] {
        let entries = ex[list].as_array().expect("liste d'entrées");
        for (i, e) in entries.iter().enumerate() {
            let ctx = format!("{list}[{i}]");
            let input = input_of(&e["in"]);

            // (i) streak : les 4 champs date = input
            let mut sv = typical["streak"].clone();
            let obj = sv.as_object_mut().expect("objet streak");
            match &input {
                None => {
                    for k in [
                        "currentStreakStart",
                        "currentStreakEnd",
                        "longestStreakStart",
                        "longestStreakEnd",
                    ] {
                        obj.remove(k);
                    }
                }
                Some(v) => {
                    let jv = json_of(v);
                    for k in [
                        "currentStreakStart",
                        "currentStreakEnd",
                        "longestStreakStart",
                        "longestStreakEnd",
                    ] {
                        obj.insert(k.to_string(), jv.clone());
                    }
                }
            }
            set_card_id_counter(e["su"].as_u64().expect("uid streak"));
            let got = render_streak_card(&StreakData::from_value(&sv), &options);
            check(got, &e["s"], &ctx, "streak");

            // (ii) profile : createdAt = input ET toutes les dates du calendrier = input
            let mut pv = typical["profile"].clone();
            let obj = pv.as_object_mut().expect("objet profile");
            match &input {
                None => {
                    obj.remove("createdAt");
                }
                Some(v) => {
                    obj.insert("createdAt".to_string(), json_of(v));
                }
            }
            if let Some(cal) = pv
                .get_mut("contributionCalendar")
                .and_then(Value::as_array_mut)
            {
                for d in cal {
                    let obj = d.as_object_mut().expect("jour");
                    match &input {
                        None => {
                            obj.remove("date");
                        }
                        Some(v) => {
                            obj.insert("date".to_string(), json_of(v));
                        }
                    }
                }
            }
            set_card_id_counter(e["pu"].as_u64().expect("uid profile"));
            let got = render_profile_card(&ProfileData::from_value(&pv), &options);
            check(got, &e["p"], &ctx, "profile");

            checked += 2;
            if list == "representative" {
                representative += 1;
            } else {
                random += 1;
            }
        }
    }
    profile::clear_clock();
    // Portes du brief : ≥ 60 représentatives + 2 000 aléatoires, × 2 cartes
    assert!(
        representative >= 60,
        "{representative} représentatives (< 60)"
    );
    assert_eq!(random, 2000, "liste aléatoire incomplète");
    assert_eq!(checked, (representative + random) * 2);
    println!("R1 : {checked} rendus comparés à l'oracle dates ({representative} représentatives + {random} aléatoires)");
}
