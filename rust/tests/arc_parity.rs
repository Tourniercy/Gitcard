//! `createArcPath` (base-card.ts) est exporté mais inutilisé par les cartes :
//! porté pour parité de surface publique et vérifié contre l'extraction legacy
//! Node v26.13.1 (`tests/golden/meta/arc-extract.json`, 60 échantillons).

use gitcard_p0::create_arc_path;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn read_json(path: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("arc-extract.json illisible"))
        .expect("json invalide")
}

#[test]
fn arc_path_vs_extraction_legacy() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/golden/meta/arc-extract.json");
    let v = read_json(&path);
    let cx = v["cx"].as_f64().expect("cx");
    let cy = v["cy"].as_f64().expect("cy");
    let radius = v["radius"].as_f64().expect("radius");
    let samples = v["samples"].as_array().expect("samples[]");
    assert_eq!(samples.len(), 60, "60 échantillons extraits du legacy");
    for s in samples {
        let out = create_arc_path(
            cx,
            cy,
            radius,
            s["startAngle"].as_f64().expect("startAngle"),
            s["endAngle"].as_f64().expect("endAngle"),
        );
        assert_eq!(
            out,
            s["out"].as_str().expect("out"),
            "createArcPath({cx}, {cy}, {radius}, {}, {})",
            s["startAngle"],
            s["endAngle"]
        );
    }
}
