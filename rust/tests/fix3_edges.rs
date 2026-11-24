//! fix3 — épinglage des 6 constats de la revue fix2 (I1-I3, M1-M3) contre
//! l'oracle legacy EXÉCUTÉ (`node:26.13.1-alpine`, `--network none`, dépôt
//! monté en lecture) : pour chaque rendu réussi, le couple (uid, md5) relevé
//! dans le SVG legacy (`clip-<uid>`) — le test replace le compteur global puis
//! compare le md5 ; pour chaque levée, le type d'erreur JS (`RangeError`,
//! `TypeError`). Aucune valeur recopiée du brief ou de la revue.
//!
//! I2 doit passer en debug ET en release (`cargo test --release --test
//! fix3_edges`) : la borne d'année est appliquée AVANT tout calcul entier.

use gitcard_p0::cards::base::set_card_id_counter;
use gitcard_p0::cards::profile;
use gitcard_p0::cards::{
    render_langs_card, render_profile_card, render_stats_card, render_streak_card, RenderError,
};
use gitcard_p0::intl::locale_format;
use gitcard_p0::jsdate::{format_date_en, parse_string};
use gitcard_p0::jsval::JsVal;
use gitcard_p0::md5_hex;
use gitcard_p0::types::{CardOptions, GitHubStats, ProfileData, StreakData, TopLangsData};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

fn golden() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/golden")
}

fn typical() -> Value {
    serde_json::from_str(
        &fs::read_to_string(golden().join("fixtures/typical.json"))
            .expect("typical.json illisible"),
    )
    .expect("typical.json invalide")
}

fn options() -> CardOptions {
    CardOptions::from_value(&json!({}))
}

/// Rend et compare le md5 à l'oracle, après avoir replacé le compteur d'uid
/// (règle su/pu : le md5 dépend de l'uid exact placé avant le rendu).
fn assert_svg(svg: Result<String, RenderError>, md5: &str, ctx: &str) {
    let s = svg.unwrap_or_else(|e| panic!("{ctx} : le legacy rend un SVG, le Rust échoue : {e}"));
    assert_eq!(md5_hex(&s), md5, "{ctx} : md5 divergent de l'oracle");
}

fn assert_err(err: Result<String, RenderError>, kind: &str, ctx: &str) {
    let e = err
        .err()
        .unwrap_or_else(|| panic!("{ctx} : le legacy LÈVE {kind}, le Rust a rendu un SVG"));
    match (kind, &e) {
        ("RangeError", RenderError::RangeError(_)) | ("TypeError", RenderError::TypeError(_)) => {}
        (k, got) => panic!("{ctx} : attendu {k}, obtenu {got}"),
    }
}

/// Dates de la carte streak = input ; profile : createdAt = input ET toutes
/// les dates du contributionCalendar = input (harnais identique à date-extract).
fn date_probe_data(input: &str) -> (Value, Value) {
    let t = typical();
    let mut streak = t["streak"].clone();
    for k in [
        "currentStreakStart",
        "currentStreakEnd",
        "longestStreakStart",
        "longestStreakEnd",
    ] {
        streak[k] = json!(input);
    }
    let mut profile = t["profile"].clone();
    profile["createdAt"] = json!(input);
    for d in profile["contributionCalendar"]
        .as_array_mut()
        .expect("calendrier")
    {
        d["date"] = json!(input);
    }
    (streak, profile)
}

// ---------------------------------------------------------------------------
// I1 — run de chiffres non borné dans une date : Invalid, jamais de panic.
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Les 6 constats s'exécutent dans UN SEUL test : le compteur global d'uid
// (`set_card_id_counter` / AtomicU64) et l'horloge sont partagés par le
// processus de test — `cargo test` lance les tests en parallèle et les md5
// dépendent de l'uid exact, donc tout test séparé courrait sur un compteur
// muté par ses voisins.
// ---------------------------------------------------------------------------
#[test]
fn fix3_six_constats_revue_fix2() {
    i1_runs_de_chiffres_longues_invalides();
    i2_annees_hors_fenetre_invalides();
    i3_infinis_compteurs();
    m1_negzero();
    m2_annees_ere();
    m3_conteneurs_non_tableau();
}

fn i1_runs_de_chiffres_longues_invalides() {
    profile::set_clock(2026, 11);
    for input in [
        "99999999999999999999",
        "12345678901234567890",
        "99999999999999999999x",
        "99999999999999999999june",
    ] {
        assert!(
            parse_string(input).is_none(),
            "I1 {input} : le legacy rend Invalid, le port a parsé"
        );
    }
    let options = options();
    let (sv, pv) = date_probe_data("99999999999999999999");
    assert_err(
        render_streak_card(&StreakData::from_value(&sv), &options),
        "RangeError",
        "I1 streak 99999999999999999999",
    );
    set_card_id_counter(0);
    assert_svg(
        render_profile_card(
            &ProfileData::from_value(&pv).expect("I1 : calendrier tableau"),
            &options,
        ),
        "afd7b5016a1b89f04c680b274dabc109",
        "I1 profile 99999999999999999999 (uid 0)",
    );
}

// ---------------------------------------------------------------------------
// I2 — année entière mais hors fenêtre ±275 760 : Invalid, sans overflow i64.
// Le test passe en debug ET en release (borne AVANT days_from_civil).
// ---------------------------------------------------------------------------
fn i2_annees_hors_fenetre_invalides() {
    profile::set_clock(2026, 11);
    for input in [
        "9223372036854775807", // i64::MAX — débordait dans era * 146_097
        "100000000000000000",  // 1e17
        "9999999999999999999", // 19 chiffres, parseable i64
    ] {
        assert!(
            parse_string(input).is_none(),
            "I2 {input} : le legacy rend Invalid, le port a parsé"
        );
    }
    // Bornes incluses de la fenêtre TimeClip : rendues.
    assert_eq!(parse_string("275760-09-13"), Some(8_640_000_000_000_000.0));
    assert_eq!(
        parse_string("-271821-04-20"),
        Some(-8_640_000_000_000_000.0)
    );
    // Hors fenêtre : Invalid.
    assert_eq!(parse_string("275760-12-31"), None);
    assert_eq!(parse_string("-275760-01-01"), None);
    // Année négative courte du parseur legacy : le signe est absorbé (V8),
    // « -2026 » → 1er janvier 2026 (oracle : t = 1767225600000).
    assert_eq!(parse_string("-2026"), Some(1_767_225_600_000.0));

    let options = options();
    for (input, uid, md5) in [
        (
            "9223372036854775807",
            1u64,
            "784bce3e01c079590f31cec52aedd4c1",
        ),
        ("100000000000000000", 2, "a687bf5f68b5f74b9d3792497211263b"),
        ("9999999999999999999", 3, "0b5526938fc6bbfdbab520a7e386e95b"),
    ] {
        let (_, pv) = date_probe_data(input);
        set_card_id_counter(uid);
        assert_svg(
            render_profile_card(
                &ProfileData::from_value(&pv).expect("I2 : calendrier tableau"),
                &options,
            ),
            md5,
            &format!("I2 profile {input} (uid {uid})"),
        );
    }
    // Borne haute incluse : la streak rend encore (oracle uid 9).
    let (sv, _) = date_probe_data("275760-09-13");
    set_card_id_counter(9);
    assert_svg(
        render_streak_card(&StreakData::from_value(&sv), &options),
        "7ad6ef545404da5203d654a3ab66d93e",
        "I2 streak 275760-09-13 (uid 9)",
    );
}

// ---------------------------------------------------------------------------
// I3 — ±Infinity dans un compteur : « ∞ » / « -∞ » (ICU), jamais d'index
// hors bornes sur « inf ».
// ---------------------------------------------------------------------------
fn i3_infinis_compteurs() {
    profile::set_clock(2026, 11);
    let fmt = locale_format("en").expect("locale en");
    assert_eq!(fmt.format_f64(f64::INFINITY), "∞");
    assert_eq!(fmt.format_f64(f64::NEG_INFINITY), "-∞");

    // JSON « 1e400 » → littéral conservé par serde_json arbitrary_precision (le
    // `Value` serde_json ne peut pas porter ±inf) ; la conversion JsVal restitue
    // +Infinity comme JSON.parse de V8 (chemin `jsval::number_as_f64`).
    let v: Value = serde_json::from_str("{\"totalStars\": 1e400}").expect("1e400 doit parser");
    let jv = JsVal::from_json(&v["totalStars"]);
    assert!(
        matches!(jv, JsVal::Number(f) if f == f64::INFINITY),
        "serde_json doit mapper 1e400 sur l'infini via JsVal, obtenu {jv:?}"
    );

    let t = typical();
    let mut stats = t["stats"].clone();
    stats["totalStars"] = v["totalStars"].clone();
    set_card_id_counter(4);
    assert_svg(
        render_stats_card(&GitHubStats::from_value(&stats), &options()),
        "e9ef2b1cf95b0ab9dfc1bf42a692217f",
        "I3 stats totalStars=1e400 (uid 4)",
    );
    let mut stats = t["stats"].clone();
    stats["totalForks"] = serde_json::from_str::<Value>("1e400").expect("1e400 doit parser");
    set_card_id_counter(5);
    assert_svg(
        render_stats_card(&GitHubStats::from_value(&stats), &options()),
        "af3ea5d91f37d04f44ff541f77e54f1e",
        "I3 stats totalForks=1e400 (uid 5)",
    );
}

// ---------------------------------------------------------------------------
// M1 — -0 : « -0 » (l'oracle prime sur le commentaire du code legacy).
// ---------------------------------------------------------------------------
fn m1_negzero() {
    profile::set_clock(2026, 11);
    let fmt = locale_format("en").expect("locale en");
    assert_eq!(fmt.format_f64(-0.0), "-0");
    assert_eq!(fmt.format_f64(0.0), "0");

    // Marques ICU par locale (oracle fix3 : Intl.NumberFormat(code).format(…),
    // Node v26.13.1) — « ar » préfixe toute sortie signée d'un LRM U+200E,
    // « sv » utilise le vrai signe moins U+2212 ; « en » : tiret ASCII, pas de
    // marque. Positifs et NaN sans marque ni signe (corpus ar inchangé).
    let ar = locale_format("ar").expect("locale ar");
    assert_eq!(ar.format_f64(-0.0), "\u{200e}-0");
    assert_eq!(ar.format_f64(-5.0), "\u{200e}-5");
    assert_eq!(ar.format_f64(f64::NEG_INFINITY), "\u{200e}-∞");
    assert_eq!(ar.format_f64(f64::INFINITY), "∞");
    let sv = locale_format("sv").expect("locale sv");
    assert_eq!(sv.format_f64(-5.0), "\u{2212}5");
    assert_eq!(sv.format_f64(f64::NEG_INFINITY), "\u{2212}∞");

    // JSON « -0.0 » → -0.0 préservé (serde_json arbitrary_precision, fix3 :
    // la valeur reste la chaîne source, as_f64 garde le signe — et JSON.parse
    // Node comme « -0 » entier rend aussi -0, parité vérifiée à l'oracle).
    let v: Value = serde_json::from_str("{\"totalStars\": -0.0}").expect("-0.0 doit parser");
    let jv = JsVal::from_json(&v["totalStars"]);
    assert!(
        matches!(jv, JsVal::Number(f) if f == 0.0 && f.is_sign_negative()),
        "serde_json doit préserver le -0 via JsVal, obtenu {jv:?}"
    );

    let t = typical();
    let mut stats = t["stats"].clone();
    stats["totalStars"] = v["totalStars"].clone();
    set_card_id_counter(6);
    assert_svg(
        render_stats_card(&GitHubStats::from_value(&stats), &options()),
        "ecd06d63f3703f7da15131f0af63a10e",
        "M1 stats totalStars=-0 (uid 6)",
    );
}

// ---------------------------------------------------------------------------
// M2 — années ≤ 0 : ICU affiche l'année d'ÈRE (1 − y).
// ---------------------------------------------------------------------------
fn m2_annees_ere() {
    profile::set_clock(2026, 11);
    for (input, t_ms, fmt) in [
        ("-000001-01-01", -62_198_755_200_000.0, "Jan 1, 2"),
        ("-002026-03-02", -126_096_393_600_000.0, "Mar 2, 2027"),
        ("0000-01-01", -62_167_219_200_000.0, "Jan 1, 1"),
        ("-000002-06-15", -62_216_035_200_000.0, "Jun 15, 3"),
    ] {
        assert_eq!(parse_string(input), Some(t_ms), "M2 {input} : time value");
        assert_eq!(format_date_en(t_ms), fmt, "M2 {input} : format en");
    }

    let options = options();
    let t = typical();
    let mut streak = t["streak"].clone();
    streak["currentStreakStart"] = json!("-000001-01-01");
    streak["currentStreakEnd"] = json!("-000001-01-01");
    streak["longestStreakStart"] = json!("-002026-03-02");
    streak["longestStreakEnd"] = json!("-002026-03-02");
    set_card_id_counter(7);
    assert_svg(
        render_streak_card(&StreakData::from_value(&streak), &options),
        "5b01c3b022e403c2512d7da1cf3b6f50",
        "M2 streak bc-same (uid 7)",
    );
    let mut streak = t["streak"].clone();
    streak["currentStreakStart"] = json!("-000001-01-01");
    streak["currentStreakEnd"] = json!("-002026-03-02");
    streak["longestStreakStart"] = json!("-000001-01-01");
    streak["longestStreakEnd"] = json!("-002026-03-02");
    set_card_id_counter(8);
    assert_svg(
        render_streak_card(&StreakData::from_value(&streak), &options),
        "66e6211f37ce5f66513c71299de1f388",
        "M2 streak bc-mixed (uid 8)",
    );
}

// ---------------------------------------------------------------------------
// M3 — conteneurs non tableau : TypeError comme `.map` sur undefined (langs)
// et comme `undefined.length` (profile) ; objet/nombre/booléen → calendrier
// vide RENDU (oracle), jamais de carte vide silencieuse pour langs.
// ---------------------------------------------------------------------------
fn m3_conteneurs_non_tableau() {
    profile::set_clock(2026, 11);
    let t = typical();
    let options = options();

    // langs : TOUT non-tableau → TypeError (`.map` n'existe pas ailleurs).
    for (value, label) in [
        (None, "absent"),
        (Some(json!(null)), "null"),
        (Some(json!(42)), "42"),
        (Some(json!("x")), "\"x\""),
        (Some(json!({})), "{}"),
        (Some(json!(true)), "true"),
    ] {
        let mut langs = t["langs"].clone();
        match value {
            None => {
                langs
                    .as_object_mut()
                    .expect("langs objet")
                    .remove("languages");
            }
            Some(v) => langs["languages"] = v,
        };
        assert_err(
            TopLangsData::from_value(&langs)
                .map(|d| render_langs_card(&d, &options))
                .and_then(|r| r),
            "TypeError",
            &format!("M3 langs {label}"),
        );
    }
    // langs [] : rendu (message « No language data available »), oracle uid 10.
    let mut langs = t["langs"].clone();
    langs["languages"] = json!([]);
    set_card_id_counter(10);
    let langs_data = TopLangsData::from_value(&langs).expect("M3 langs [] : tableau");
    assert_svg(
        render_langs_card(&langs_data, &options),
        "fc77f5a28286afad92ea0c4dff52b821",
        "M3 langs [] (uid 10)",
    );

    // profile : absent/null/string → TypeError ; objet/nombre/booléen → rendu.
    for (value, label) in [
        (None, "absent"),
        (Some(json!(null)), "null"),
        (Some(json!("abc")), "\"abc\""),
    ] {
        let mut profile_data = t["profile"].clone();
        match value {
            None => {
                profile_data
                    .as_object_mut()
                    .expect("profile objet")
                    .remove("contributionCalendar");
            }
            Some(v) => profile_data["contributionCalendar"] = v,
        };
        assert_err(
            ProfileData::from_value(&profile_data)
                .map(|d| render_profile_card(&d, &options))
                .and_then(|r| r),
            "TypeError",
            &format!("M3 profile {label}"),
        );
    }
    // [] / {} / 42 / true : tous rendent le calendrier vide — SVG identiques
    // modulo uid (l'oracle donne des md5 différents uniquement à cause du
    // compteur). On le prouve : rendus au MÊME uid 12 → même md5 que {} (uid 12).
    let expected = "c9dfd2f637cb5c3e9a522cea7148ca55";
    for (value, label) in [
        (Some(json!([])), "[]"),
        (Some(json!({})), "{}"),
        (Some(json!(42)), "42"),
        (Some(json!(true)), "true"),
    ] {
        let mut profile_data = t["profile"].clone();
        profile_data["contributionCalendar"] = value.expect("Some");
        set_card_id_counter(12);
        let pd = ProfileData::from_value(&profile_data)
            .unwrap_or_else(|e| panic!("M3 profile {label} : doit rendre : {e}"));
        let svg = render_profile_card(&pd, &options)
            .unwrap_or_else(|e| panic!("M3 profile {label} : doit rendre : {e}"));
        assert_eq!(
            md5_hex(&svg),
            expected,
            "M3 profile {label} : doit équivaloir au calendrier vide (uid 12)"
        );
    }
}
