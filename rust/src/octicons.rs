//! Octicons embarqués : extraction de `@primer/octicons@19.23.1` (épinglé par sha512
//! dans pnpm-lock.yaml), réalisée en conteneur node:26.13.1-alpine par
//! `scripts/gen-golden.mjs`. Le legacy n'utilise que les variantes 16 px de 11 icônes.

use serde_json::Value;
use std::sync::OnceLock;

const OCTICONS_JSON: &str = include_str!("octicons.json");

/// Icônes réellement utilisées par les 4 cartes (toutes en 16 px).
pub const USED_ICONS: [&str; 11] = [
    "star",
    "repo-forked",
    "git-commit",
    "git-pull-request",
    "issue-opened",
    "repo",
    "calendar",
    "flame",
    "trophy",
    "history",
    "mail",
];

fn data() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(OCTICONS_JSON).expect("octicons.json invalide"))
}

/// `(width, path)` de la variante 16 px — équivalent de `octicons[name].heights[16]`.
pub fn icon_16(name: &str) -> Option<(u32, &'static str)> {
    let v = data()["icons"].get(name)?;
    let width = v["width"].as_u64()? as u32;
    let path = v["path"].as_str()?;
    Some((width, path))
}

pub fn source_version() -> &'static str {
    data()["source"].as_str().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_des_11_icones_utilisees() {
        assert!(
            source_version().ends_with("19.23.1"),
            "version épinglée: {}",
            source_version()
        );
        for name in USED_ICONS {
            let (width, path) = icon_16(name).unwrap_or_else(|| panic!("octicon manquant: {name}"));
            assert_eq!(
                width, 16,
                "{name}: le legacy n'utilise que la variante 16px"
            );
            assert!(path.starts_with("<path d="), "{name}: path inattendu");
        }
    }

    #[test]
    fn egalite_avec_extraction_independante() {
        // rust/src/octicons.json == copie témoin écrite séparément par le générateur
        // (tests/golden/meta/octicons-extract.json) : mêmes width + path octet à octet.
        const WITNESS: &str = include_str!("../../tests/golden/meta/octicons-extract.json");
        let witness: Value = serde_json::from_str(WITNESS).expect("témoin octicons invalide");
        for name in USED_ICONS {
            assert_eq!(
                data()["icons"][name],
                witness["icons"][name],
                "divergence octicon {name} entre rust/src et tests/golden/meta"
            );
        }
        assert_eq!(source_version(), witness["source"].as_str().unwrap());
    }
}
