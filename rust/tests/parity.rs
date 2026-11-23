//! Parité P0 contre le corpus golden généré par le legacy Node v26.13.1
//! (`scripts/gen-golden.mjs`) :
//! 1. `js_to_fixed` / `js_f64_to_string` vs 24 047 échantillons Node (num-samples.json) ;
//! 2. `f64::cos`/`f64::sin` (libm musl) vs V8, en binaire (`to_bits`) ET tel qu'entré
//!    dans le SVG (via `js_f64_to_string`), sur les angles réels des donuts + balayage dense ;
//! 3. corpus complet : reconstruction Rust des primitifs numériques (Intl, toFixed,
//!    chemins donut cos/sin, interpolation cubique du profile, ring, hashes ETag/params)
//!    et vérification de leur présence exacte dans les 371 SVG du legacy.

use gitcard_p0::{fdlibm, intl, jsnum, md5_hex, params_hash};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn golden() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/rust → le corpus est à <repo>/tests/golden
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/golden")
}

fn read_json(path: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("fichier golden illisible"))
        .expect("json golden invalide")
}

fn f_of(hex_bits: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex_bits, 16).expect("bits hex"))
}

const PI: f64 = std::f64::consts::PI;
const J: fn(f64) -> String = jsnum::js_f64_to_string;

// ---------------------------------------------------------------- num-samples

#[test]
fn num_samples_parity_with_node() {
    let v = read_json(&golden().join("num-samples.json"));
    let mut fixed_checked = 0u32;
    for e in v["fixed"].as_array().expect("fixed[]") {
        let x = f_of(e[0].as_str().expect("bits"));
        let d = e[1].as_u64().expect("digits") as u32;
        let expected = e[2].as_str().expect("expected");
        let got =
            jsnum::js_to_fixed(x, d).unwrap_or_else(|err| panic!("toFixed({x:?},{d}): {err}"));
        assert_eq!(got, expected, "toFixed({x:?}, {d})");
        fixed_checked += 1;
    }
    let mut string_checked = 0u32;
    for e in v["string"].as_array().expect("string[]") {
        let x = f_of(e[0].as_str().expect("bits"));
        let expected = e[1].as_str().expect("expected");
        assert_eq!(jsnum::js_f64_to_string(x), expected, "String({x:?})");
        string_checked += 1;
    }
    assert!(fixed_checked >= 18_000, "toFixed vérifiés: {fixed_checked}");
    assert!(
        string_checked >= 6_000,
        "String vérifiées: {string_checked}"
    );
    println!(
        "num-samples: {fixed_checked} toFixed + {string_checked} String reproduits à l'octet près"
    );
}

// ---------------------------------------------------------------- libm sweep

#[test]
fn libm_cos_sin_parity_with_node() {
    let v = read_json(&golden().join("libm-sweep.json"));
    let mut total = 0u32;
    // fdlibm (porté, = V8) : doit être bit-identique à Node
    let mut fdlibm_diffs = 0u32;
    // musl (f64::cos/sin du target) : mesure d'écart pour le verdict G1
    let mut musl_diffs = 0u32;
    let mut musl_max_ulp = 0i64;
    let mut examples: Vec<String> = Vec::new();
    for e in v["angles"].as_array().expect("angles[]") {
        let angle = f_of(e[0].as_str().expect("angle bits"));
        let cos_ref = f_of(e[1].as_str().expect("cos bits"));
        let sin_ref = f_of(e[2].as_str().expect("sin bits"));
        total += 1;
        // fdlibm porté
        let (cos_f, sin_f) = (fdlibm::cos(angle), fdlibm::sin(angle));
        let fd_diff = cos_f.to_bits() != cos_ref.to_bits() || sin_f.to_bits() != sin_ref.to_bits();
        if fd_diff && examples.len() < 5 {
            examples.push(format!(
                "fdlibm angle={angle:e} cos(rust={:016x}, node={:016x}) sin(rust={:016x}, node={:016x})",
                cos_f.to_bits(),
                cos_ref.to_bits(),
                sin_f.to_bits(),
                sin_ref.to_bits()
            ));
        }
        // valeur entrant dans le SVG (interpolation ${x}) : même chiffres + même forme
        assert_eq!(J(cos_f), J(cos_ref), "cos formaté angle={angle:e}");
        assert_eq!(J(sin_f), J(sin_ref), "sin formaté angle={angle:e}");
        // musl : combien d'ULP d'écart ?
        let ulp = |a: f64, b: f64| -> i64 {
            let (ia, ib) = (a.to_bits() as i64, b.to_bits() as i64);
            (ia - ib).abs()
        };
        let u = ulp(angle.cos(), cos_ref).max(ulp(angle.sin(), sin_ref));
        if u > 0 {
            musl_diffs += 1;
            musl_max_ulp = musl_max_ulp.max(u);
        }
        if fd_diff {
            fdlibm_diffs += 1;
        }
    }
    println!(
        "libm: {total} angles ; musl: {musl_diffs} écarts (max {musl_max_ulp} ULP) ; \
         fdlibm porté: {fdlibm_diffs} écart(s) ; format SVG identique"
    );
    assert!(total > 4000, "sweep trop court: {total}");
    assert_eq!(fdlibm_diffs, 0, "écarts fdlibm vs Node: {examples:?}");
    assert!(
        musl_max_ulp <= 1,
        "écart musl inattendu: {musl_max_ulp} ULP"
    );
}

// ---------------------------------------------------------------- corpus

fn build_options_json(o: &Value) -> String {
    // Ordre d'insertion exact de parseCardOptions ; undefined omis (étude §1.7).
    let mut s = String::from("{\"theme\":");
    s.push_str(&serde_json::to_string(o["theme"].as_str().expect("theme")).expect("theme json"));
    s.push_str(",\"hide\":");
    s.push_str(&serde_json::to_string(&o["hide"]).expect("hide json"));
    for (k, default) in [
        ("showIcons", true),
        ("hideBorder", false),
        ("hideTitle", false),
    ] {
        let v = o.get(k).and_then(Value::as_bool).unwrap_or(default);
        s.push_str(&format!(",\"{k}\":{}", if v { "true" } else { "false" }));
    }
    for k in [
        "bgColor",
        "titleColor",
        "textColor",
        "iconColor",
        "borderColor",
    ] {
        if let Some(v) = o.get(k).and_then(Value::as_str) {
            s.push_str(&format!(
                ",\"{k}\":{}",
                serde_json::to_string(v).expect("couleur json")
            ));
        }
    }
    s.push_str(&format!(
        ",\"cacheSeconds\":{}",
        o["cacheSeconds"].as_i64().expect("cacheSeconds")
    ));
    s.push_str(&format!(
        ",\"locale\":{}",
        serde_json::to_string(o["locale"].as_str().expect("locale")).expect("locale json")
    ));
    if let Some(l) = o.get("layout").and_then(Value::as_str) {
        s.push_str(&format!(
            ",\"layout\":{}",
            serde_json::to_string(l).expect("layout json")
        ));
    }
    s.push('}');
    s
}

fn polar(cx: f64, cy: f64, r: f64, angle: f64) -> (f64, f64) {
    // cos/sin via fdlibm (identique à V8) : les octets du path viennent de Math.cos/sin
    let a = angle - PI / 2.0;
    (cx + r * fdlibm::cos(a), cy + r * fdlibm::sin(a))
}

fn donut_arc_path(cx: f64, cy: f64, outer: f64, inner: f64, start: f64, end: f64) -> String {
    let so = polar(cx, cy, outer, start);
    let eo = polar(cx, cy, outer, end);
    let si = polar(cx, cy, inner, end);
    let ei = polar(cx, cy, inner, start);
    let large = i32::from(end - start > PI);
    format!(
        "M {} {} A {} {} 0 {large} 1 {} {} L {} {} A {} {} 0 {large} 0 {} {} Z",
        J(so.0),
        J(so.1),
        J(outer),
        J(outer),
        J(eo.0),
        J(eo.1),
        J(si.0),
        J(si.1),
        J(inner),
        J(inner),
        J(ei.0),
        J(ei.1)
    )
}

fn check_stats(svg: &str, stats: &Value, opts: &Value, locale: &str) {
    let rows = [
        ("stars", "totalStars"),
        ("forks", "totalForks"),
        ("commits", "totalCommits"),
        ("prs", "totalPRs"),
        ("issues", "totalIssues"),
        ("contribs", "contributedTo"),
    ];
    let hidden: Vec<String> = opts["hide"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let mut visible_needles: Vec<String> = Vec::new();
    let mut hidden_needles: Vec<(String, String)> = Vec::new();
    for (key, field) in rows {
        let v = stats[field].as_u64().expect("compteur");
        let needle = format!(
            ">{}</text>",
            intl::format_int(v, locale).expect("locale table")
        );
        if hidden.iter().any(|h| h == key) {
            hidden_needles.push((key.to_string(), needle));
        } else {
            visible_needles.push(needle);
        }
    }
    for needle in &visible_needles {
        assert!(svg.contains(needle), "ligne visible absente: {needle}");
    }
    for (key, needle) in &hidden_needles {
        // NB : sur la fixture « empty » toutes les valeurs valent 0 — le needle d'une
        // ligne cachée peut collisionner avec celui d'une ligne visible à 0.
        assert!(
            !svg.contains(needle) || visible_needles.contains(needle),
            "{key} caché mais présent sans collision visible: {needle}"
        );
    }
    // Grade + ring chart (port exact de calculateGrade / createRingChart)
    let score = stats["totalCommits"].as_f64().expect("n") * 1.0
        + stats["totalPRs"].as_f64().expect("n") * 3.0
        + stats["totalIssues"].as_f64().expect("n") * 2.0
        + stats["totalStars"].as_f64().expect("n") * 1.0
        + stats["totalForks"].as_f64().expect("n") * 0.5
        + stats["contributedTo"].as_f64().expect("n") * 5.0;
    let percentage = if score >= 5000.0 {
        100.0
    } else if score >= 2500.0 {
        85.0
    } else if score >= 1000.0 {
        70.0
    } else if score >= 500.0 {
        55.0
    } else {
        40.0
    };
    let circumference = 2.0 * PI * 40.0;
    let offset = circumference * (1.0 - percentage / 100.0);
    assert!(
        svg.contains(&format!("stroke-dasharray=\"{}\"", J(circumference))),
        "dasharray"
    );
    assert!(
        svg.contains(&format!("stroke-dashoffset=\"{}\"", J(offset))),
        "dashoffset"
    );
}

fn check_streak(svg: &str, streak: &Value, locale: &str) {
    let total = streak["totalContributions"].as_u64().expect("n");
    // streak-card rend la valeur sur sa propre ligne dans le <text>
    let needle = format!(
        "\n        {}\n      </text>",
        intl::format_int(total, locale).expect("locale")
    );
    assert!(svg.contains(&needle), "total contributions: {needle}");
    for field in ["currentStreak", "longestStreak"] {
        let n = streak[field].as_i64().expect("streak entier");
        let needle = format!("\n        {n}\n      </text>");
        assert!(svg.contains(&needle), "{field}: {needle:?}");
    }
    for (s_field, e_field) in [
        ("currentStreakStart", "currentStreakEnd"),
        ("longestStreakStart", "longestStreakEnd"),
    ] {
        let s = streak[s_field].as_str().expect("date");
        let e = streak[e_field].as_str().expect("date");
        if s.is_empty() || e.is_empty() {
            assert!(
                svg.contains("\n        \n      </text>"),
                "dates vides attendues"
            );
        } else {
            let range = format!(
                "{} - {}",
                intl::date_en(s).expect("date en"),
                intl::date_en(e).expect("date en")
            );
            assert!(
                svg.contains(&format!("\n        {range}\n      </text>")),
                "range {range}"
            );
        }
    }
}

fn check_langs(svg: &str, langs: &Value, opts: &Value) {
    let compact = opts.get("layout").and_then(Value::as_str) == Some("compact");
    let hide_title = opts["hideTitle"].as_bool().unwrap_or(false);
    let languages: Vec<(String, f64)> = langs["languages"]
        .as_array()
        .expect("languages")
        .iter()
        .map(|l| {
            (
                l["name"].as_str().expect("name").to_string(),
                l["percentage"].as_f64().expect("pct"),
            )
        })
        .collect();
    if compact {
        let bar_width = 250.0; // COMPACT_WIDTH - 50
        for (name, pct) in &languages {
            let w = (pct / 100.0) * bar_width;
            assert!(
                svg.contains(&format!("from=\"0\" to=\"{}\"", J(w))),
                "barre {name}: to"
            );
            assert!(
                svg.contains(&format!("width=\"{}\" height=\"8\"", J(w))),
                "barre {name}: width"
            );
        }
        for (name, pct) in &languages {
            let p2 = jsnum::js_to_fixed(*pct, 2).expect("toFixed2");
            assert!(
                svg.contains(&format!("{p2}%</text>")),
                "légende compacte {name}: {p2}%"
            );
        }
    } else {
        let donut_cy = if hide_title { 100.0 } else { 110.0 };
        let mut current = 0.0f64;
        for (name, pct) in &languages {
            let sweep = (pct / 100.0) * PI * 2.0;
            let start = current + 0.02;
            let end = current + sweep - 0.02;
            current += sweep;
            if sweep < 0.05 {
                continue; // segments minuscules sautés (langs-card.ts)
            }
            let path = donut_arc_path(90.0, donut_cy, 55.0, 55.0 * 0.6, start, end);
            assert!(
                svg.contains(&format!("d=\"{path}\"")),
                "donut {name}: path {path}"
            );
        }
        for (name, pct) in &languages {
            let p1 = jsnum::js_to_fixed(*pct, 1).expect("toFixed1");
            assert!(
                svg.contains(&format!("{p1}%</text>")),
                "légende {name}: {p1}%"
            );
        }
    }
}

fn monotone_cubic_path(points: &[(f64, f64)]) -> String {
    let n = points.len();
    if n < 2 {
        return String::new();
    }
    if n == 2 {
        return format!("L{},{}", J(points[1].0), J(points[1].1));
    }
    let mut deltas = Vec::new();
    let mut slopes = Vec::new();
    for i in 0..n - 1 {
        let dx = points[i + 1].0 - points[i].0;
        let dy = points[i + 1].1 - points[i].1;
        deltas.push(dx);
        slopes.push(dy / dx);
    }
    let mut tangents = vec![slopes[0]];
    for i in 1..n - 1 {
        let s0 = slopes[i - 1];
        let s1 = slopes[i];
        if s0 * s1 <= 0.0 {
            tangents.push(0.0);
        } else {
            tangents.push(
                (3.0 * (deltas[i - 1] + deltas[i]))
                    / ((2.0 * deltas[i] + deltas[i - 1]) / s0
                        + (deltas[i] + 2.0 * deltas[i - 1]) / s1),
            );
        }
    }
    tangents.push(slopes[n - 2]);
    let mut parts = Vec::new();
    for i in 0..n - 1 {
        let dx = deltas[i] / 3.0;
        let cp1x = points[i].0 + dx;
        let cp1y = points[i].1 + tangents[i] * dx;
        let cp2x = points[i + 1].0 - dx;
        let cp2y = points[i + 1].1 - tangents[i + 1] * dx;
        parts.push(format!(
            "C{},{} {},{} {},{}",
            J(cp1x),
            J(cp1y),
            J(cp2x),
            J(cp2y),
            J(points[i + 1].0),
            J(points[i + 1].1)
        ));
    }
    parts.join(" ")
}

fn check_profile(svg: &str, profile: &Value, locale: &str) {
    for field in ["contributionsThisYear", "publicRepos"] {
        let v = profile[field].as_u64().expect("compteur");
        let needle = format!(">{}</text>", intl::format_int(v, locale).expect("locale"));
        assert!(svg.contains(&needle), "{field}: {needle}");
    }
    // accountAge : évalué par le legacy à la date de génération (2026-11-24, UTC) ;
    // tous les fixtures ont > 1 an → rendu "Ny" avec N = 2026 - année de création.
    let created_year: i32 = profile["createdAt"].as_str().expect("createdAt")[..4]
        .parse()
        .expect("année");
    let years = 2026 - created_year;
    assert!(years >= 1, "fixture postérieure au calibrated now");
    assert!(
        svg.contains(&format!(">{years}y</text>")),
        "accountAge {years}y"
    );
    // Graphe d'aire : agrégation hebdo + interpolation cubique monotone (Fritsch-Carlson)
    let calendar: Vec<(String, i64)> = profile["contributionCalendar"]
        .as_array()
        .expect("calendar")
        .iter()
        .map(|d| {
            (
                d["date"].as_str().expect("date").to_string(),
                d["count"].as_i64().expect("count"),
            )
        })
        .collect();
    let mut weeks: Vec<(String, i64)> = Vec::new();
    for chunk in calendar.chunks(7) {
        let total: i64 = chunk.iter().map(|(_, c)| c).sum();
        let mid = &chunk[chunk.len() / 2];
        weeks.push((mid.0.clone(), total));
    }
    if weeks.is_empty() {
        return;
    }
    let max_count = weeks.iter().map(|w| w.1).max().expect("non vide").max(1) as f64;
    let n = weeks.len();
    // NB : pour n == 1 le legacy divise par (n-1) = 0 → NaN dans le path (comportement
    // réel reproduit : Rust IEEE donne NaN aussi).
    let points: Vec<(f64, f64)> = weeks
        .iter()
        .enumerate()
        .map(|(i, w)| {
            (
                320.0 + (i as f64 / (n as f64 - 1.0)) * 350.0,
                155.0 - ((w.1 as f64) / max_count) * 110.0,
            )
        })
        .collect();
    let line = format!(
        "M{},{} {}",
        J(points[0].0),
        J(points[0].1),
        monotone_cubic_path(&points)
    );
    assert!(svg.contains(&format!("d=\"{line}\"")), "linePath {line}");
    let last = points[n - 1];
    let area = format!(
        "{line} L{},{} L{},{} Z",
        J(last.0),
        J(155.0),
        J(points[0].0),
        J(155.0)
    );
    assert!(svg.contains(&format!("d=\"{area}\"")), "areaPath");
    let tick_interval = std::cmp::max(1, n / 6);
    let mut i = 0;
    while i < n {
        let month = intl::month_en(&weeks[i].0).expect("mois");
        assert!(svg.contains(&format!(">{month}</text>")), "tick {month}");
        i += tick_interval;
    }
    let mid = ((max_count / 2.0) + 0.5).floor() as i64; // Math.round
    assert!(svg.contains(&format!(">{mid}</text>")), "tick Y {mid}");
}

#[test]
fn corpus_primitives_parity() {
    let manifest = read_json(&golden().join("manifest.json"));
    let entries = manifest["entries"].as_array().expect("entries").clone();
    assert!(entries.len() >= 100, "corpus trop petit: {}", entries.len());
    let mut checked = 0u32;
    for (idx, entry) in entries.iter().enumerate() {
        // ordre de rendu figé : uid = index global croissant depuis 0 (manifesté)
        assert_eq!(
            entry["uid"].as_i64().expect("uid"),
            idx as i64,
            "uid inattendu pour {}",
            entry["file"]
        );
        let svg = fs::read_to_string(golden().join(entry["file"].as_str().expect("file")))
            .expect("SVG golden illisible");
        // ETag du legacy = md5 utf8 du SVG
        assert_eq!(md5_hex(&svg), entry["md5"].as_str().expect("md5"), "ETag");
        // paramsHash = md5(JSON.stringify(options))[:8], ordre d'insertion préservé
        let options = &entry["options"];
        assert_eq!(
            params_hash(&build_options_json(options)),
            entry["paramsHash"].as_str().expect("paramsHash"),
            "paramsHash pour {}",
            entry["file"]
        );
        let fixture = read_json(&golden().join("fixtures").join(format!(
            "{}.json",
            entry["fixture"].as_str().expect("fixture")
        )));
        let locale = options["locale"].as_str().expect("locale");
        match entry["card"].as_str().expect("card") {
            "stats" => check_stats(&svg, &fixture["stats"], options, locale),
            "streak" => check_streak(&svg, &fixture["streak"], locale),
            "langs" => check_langs(&svg, &fixture["langs"], options),
            "profile" => check_profile(&svg, &fixture["profile"], locale),
            other => panic!("carte inconnue: {other}"),
        }
        checked += 1;
    }
    assert_eq!(
        manifest["counts"]["files"].as_u64().expect("count"),
        u64::from(checked)
    );
    println!("corpus: {checked} SVG vérifiés (Intl, toFixed, paths cos/sin, cubique, hashes)");
}
