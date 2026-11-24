//! Meta-test image Docker (phase 6a) : tout chemin embarqué par
//! `include_str!`/`include_bytes!` depuis `rust/src/**/*.rs` qui SORT de
//! `rust/` doit être couvert par une instruction `COPY`/`ADD` du Dockerfile
//! (hors `--from=`) et ne pas être exclu par le filtre de contexte effectif
//! `<Dockerfile>.dockerignore`.
//!
//! Pourquoi : `octicons.rs:67`, `github.rs:33` et `intl.rs:17` embarquent les
//! goldens `tests/golden/meta/octicons-extract.json`,
//! `tests/golden/graphql/user-query.txt` et `tests/golden/intl-tables.json`
//! (source UNIQUE partagée avec les tests — on ne les duplique pas sous
//! `rust/`). `include_str!` se résout à la COMPILATION, dans le contexte de
//! build : si le Dockerfile ne copie pas ces fichiers, l'image ne construit
//! pas (échec type fix6 de weight : 5 × `couldn't read …blobs/…`). Ce test
//! ÉCHOUE sur une variante du Dockerfile sans le COPY des goldens (preuve :
//! `DOCKERFILE_PATH=<copie sans COPY> cargo test --test dockerfile_includes`,
//! sortie recopiée dans le rapport p6, exit 101) et passe sur le final.
//!
//! Pur std (aucune dépendance nouvelle). Sous-ensemble dockerignore géré :
//! littéraux, `*` (sans traverser `/`), négation `!` (dernier match gagné),
//! motifs de répertoires (tout ce qui est dessous) ; motifs non ancrés
//! matchés à toute profondeur.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// Littéraux des `include_str!("…")` / `include_bytes!("…")` d'une source.
fn chemins_embarques(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for ligne in source.lines() {
        let mut reste = ligne;
        while let Some(pos) = reste
            .find("include_str!(\"")
            .or_else(|| reste.find("include_bytes!(\""))
        {
            let apres = &reste[pos..];
            let Some(ouverture) = apres.find('"') else {
                break;
            };
            let corps = &apres[ouverture + 1..];
            let Some(fermeture) = corps.find('"') else {
                break;
            };
            out.push(corps[..fermeture].to_string());
            reste = &corps[fermeture + 1..];
        }
    }
    out
}

/// Parcours récursif des `.rs` sous `dir`.
fn collecter_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entrees) = std::fs::read_dir(dir) else {
        return;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            collecter_sources(&chemin, out);
        } else if chemin.extension().is_some_and(|ext| ext == "rs") {
            out.push(chemin);
        }
    }
}

/// Normalisation lexicale de `.`/`..` (pas de symlinks dans l'arbre git). Un
/// `..` qui dépasse la racine est CONSERVÉ : il marque la sortie du package
/// `rust/` (chemin résultant relatif au cwd du test = racine du package).
fn normaliser(p: &Path) -> PathBuf {
    let mut parties: Vec<std::ffi::OsString> = Vec::new();
    let mut montees = 0usize; // `..` excédentaires → sortie du package
    for composant in p.components() {
        match composant {
            Component::ParentDir => {
                if parties.pop().is_none() {
                    montees += 1;
                }
            }
            Component::CurDir => {}
            composant => parties.push(composant.as_os_str().to_os_string()),
        }
    }
    let mut out = PathBuf::new();
    for _ in 0..montees {
        out.push("..");
    }
    for partie in parties {
        out.push(partie);
    }
    out
}

/// Instructions `COPY`/`ADD` du Dockerfile, hors `--from=` (celles-ci copient
/// DEPUIS une image d'étape, pas depuis le contexte). Continuations `\`
/// jointes d'abord. Retour : (sources, destination), relatifs à la racine du
/// contexte de build.
fn instructions_copie(texte_dockerfile: &str) -> Vec<(Vec<String>, String)> {
    let mut jointes: Vec<String> = Vec::new();
    let mut en_cours = String::new();
    for ligne in texte_dockerfile.lines() {
        match ligne.trim_end().strip_suffix('\\') {
            Some(morceau) => {
                en_cours.push_str(morceau);
                en_cours.push(' ');
            }
            None => {
                en_cours.push_str(ligne.trim_end());
                jointes.push(std::mem::take(&mut en_cours));
            }
        }
    }
    if !en_cours.is_empty() {
        jointes.push(en_cours);
    }

    let mut out = Vec::new();
    for ligne in jointes {
        let tokens: Vec<&str> = ligne.split_whitespace().collect();
        match tokens.first().copied() {
            Some("COPY") | Some("ADD") => {}
            _ => continue,
        }
        if tokens.iter().any(|t| t.starts_with("--from=")) {
            continue;
        }
        let sans_flags: Vec<&str> = tokens[1..]
            .iter()
            .copied()
            .filter(|t| !t.starts_with("--"))
            .collect();
        if sans_flags.len() < 2 {
            continue;
        }
        let dest = sans_flags[sans_flags.len() - 1].to_string();
        let srcs: Vec<String> = sans_flags[..sans_flags.len() - 1]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        out.push((srcs, dest));
    }
    out
}

/// Le couple (sources, destination) couvre-t-il le chemin embarqué `chemin`
/// (relatif à la racine du contexte de build) ?
fn couvre(srcs: &[String], dest: &str, chemin: &str, racine: &Path) -> bool {
    for src in srcs {
        let src_propre = src.trim_end_matches('/');
        let dest_norm = normaliser(Path::new(dest))
            .to_string_lossy()
            .replace('\\', "/");
        let absolu = racine.join(src_propre);
        if absolu.is_dir() {
            // Répertoire : couvre tout ce qui est dessous SI la disposition
            // relative est conservée (destination se terminant par le dernier
            // segment de la source — forme `COPY a/b/c ./a/b/c`).
            let dernier = src_propre.rsplit('/').next().unwrap_or(src_propre);
            if chemin.starts_with(&format!("{src_propre}/")) && dest_norm.ends_with(dernier) {
                return true;
            }
        } else if absolu.is_file() && chemin == src_propre {
            return true;
        }
    }
    false
}

/// Motifs du filtre de contexte : (négation, motif). Commentaires `#` et
/// lignes vides ignorés.
fn motifs_filtre(texte: &str) -> Vec<(bool, String)> {
    texte
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|ligne| match ligne.strip_prefix('!') {
            Some(reste) => (true, reste.to_string()),
            None => (false, ligne.to_string()),
        })
        .collect()
}

/// Sémantique dockerignore (sous-ensemble déclaré en tête de fichier) : le
/// dernier motif qui matche gagne ; les motifs `!` réincluent.
fn exclu_par(motifs: &[(bool, String)], chemin: &str) -> bool {
    let mut exclu = false;
    for (negation, motif) in motifs {
        if motif_matche(motif, chemin) {
            exclu = !*negation;
        }
    }
    exclu
}

/// Sous-ensemble simple des motifs dockerignore : littéraux, `*` (jamais
/// traversant de `/`), motifs de répertoires (matchent tout ce qui est
/// dessous), ancrés à la racine (`/x` ou motif contenant `/`) vs non ancrés
/// (matchés à toute profondeur).
fn motif_matche(motif: &str, chemin: &str) -> bool {
    let (ancree, motif) = match motif.strip_prefix('/') {
        Some(reste) => (true, reste),
        None => (false, motif),
    };
    let segs_motif: Vec<&str> = motif.split('/').filter(|s| !s.is_empty()).collect();
    let segs_chemin: Vec<&str> = chemin.split('/').collect();
    if segs_motif.is_empty() || segs_chemin.len() < segs_motif.len() {
        return false;
    }
    let dernier_depart = segs_chemin.len() - segs_motif.len();
    let departs: Vec<usize> = if ancree {
        vec![0]
    } else {
        (0..=dernier_depart).collect()
    };
    for depart in departs {
        let fenetre = &segs_chemin[depart..depart + segs_motif.len()];
        if fenetre
            .iter()
            .zip(segs_motif.iter())
            .all(|(segment, partie)| segment_matche(partie, segment))
        {
            return true;
        }
    }
    false
}

/// Un segment de motif : `*` = toute suite sans `/`, à l'intérieur du segment.
fn segment_matche(partie: &str, segment: &str) -> bool {
    fn aux(m: &[u8], s: &[u8]) -> bool {
        match (m.first(), s.first()) {
            (None, None) => true,
            (Some(b'*'), _) => (0..=s.len()).any(|saute| aux(&m[1..], &s[saute..])),
            (Some(&a), Some(&b)) if a == b => aux(&m[1..], &s[1..]),
            _ => false,
        }
    }
    aux(partie.as_bytes(), segment.as_bytes())
}

#[test]
fn dockerfile_copie_tout_chemin_embarque_hors_rust() {
    // 1. Sources analysées : `<package>/src/**/*.rs`. Racine du package =
    //    `CARGO_MANIFEST_DIR` (injectée par cargo à l'exécution ; repli :
    //    cwd, où cargo place déjà les tests) — lue en lecture seule, jamais
    //    codée en dur.
    let racine_paquet = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("répertoire courant"));
    // Racine du dépôt = racine du contexte de build = parent du package
    // (`CARGO_MANIFEST_DIR/..`) : c'est l'espace des srcspec des COPY.
    let racine_depot = racine_paquet
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| racine_paquet.clone());
    let mut sources = Vec::new();
    collecter_sources(&racine_paquet.join("src"), &mut sources);
    assert!(
        sources.len() >= 10,
        "aucune source rust/src trouvée (CARGO_MANIFEST_DIR attendu : racine du package)"
    );

    // 2. Chemins embarqués qui sortent de `rust/` (relatifs à la racine du
    //    dépôt = racine du contexte de build).
    let mut hors_rust: BTreeSet<String> = BTreeSet::new();
    for source in &sources {
        let texte = std::fs::read_to_string(source).expect("lecture d'une source");
        for lit in chemins_embarques(&texte) {
            // Résolution : relatif au FICHIER source (`include_str!` se
            // résout par rapport au fichier contenant la macro), exprimé
            // relativement au package. Un `..` excédentaire marque la sortie
            // du package.
            let rel_source = source
                .strip_prefix(&racine_paquet)
                .unwrap_or(source.as_path());
            let relatif = normaliser(&rel_source.parent().unwrap_or(Path::new("")).join(&lit));
            let relatif = relatif.to_string_lossy().replace('\\', "/");
            let mut reste = relatif.as_str();
            let mut sorties = 0usize;
            while let Some(r) = reste.strip_prefix("../") {
                reste = r;
                sorties += 1;
            }
            if sorties == 1 {
                // Sort de `rust/` → relatif à la racine du dépôt = racine du
                // contexte de build (l'espace des srcspec des COPY).
                hors_rust.insert(reste.to_string());
            } else if sorties > 1 {
                // Sort du DÉPÔT lui-même : aucun COPY du contexte ne peut le
                // couvrir — on le garde tel quel (échec attendu du test).
                hors_rust.insert(relatif.clone());
            }
        }
    }
    assert!(
        !hors_rust.is_empty(),
        "plus aucun chemin embarqué hors rust/ détecté : le meta-test ne protège plus rien"
    );

    // 3. Dockerfile : `DOCKERFILE_PATH` surcharge (preuve sur la variante
    //    sans COPY des goldens) ; défaut = le Dockerfile DU PACKAGE
    //    (`rust/Dockerfile` vu de la racine du dépôt) — PAS le Dockerfile
    //    legacy (`apps/api/Dockerfile`), qui n'est pas à la racine du
    //    contexte de toute façon.
    let chemin_dockerfile = match std::env::var("DOCKERFILE_PATH") {
        Ok(p) => PathBuf::from(p),
        Err(_) => racine_paquet.join("Dockerfile"),
    };
    let texte_dockerfile = std::fs::read_to_string(&chemin_dockerfile).unwrap_or_else(|e| {
        panic!(
            "lecture du Dockerfile {} : {e}",
            chemin_dockerfile.display()
        )
    });
    let copies = instructions_copie(&texte_dockerfile);

    // 4. Filtre de contexte effectif : fichier FRÈRE du Dockerfile (BuildKit
    //    lit `<Dockerfile>.dockerignore`, cf. M4 revue 5 de weight). Absent →
    //    aucun filtre (sur la variante cassée l'échec vient alors du COPY
    //    manquant).
    let chemin_filtre = PathBuf::from(format!("{}.dockerignore", chemin_dockerfile.display()));
    let motifs = std::fs::read_to_string(&chemin_filtre)
        .map(|t| motifs_filtre(&t))
        .unwrap_or_default();

    let mut non_copies: Vec<String> = Vec::new();
    let mut exclus: Vec<String> = Vec::new();
    for chemin in &hors_rust {
        if !copies
            .iter()
            .any(|(srcs, dest)| couvre(srcs, dest, chemin, &racine_depot))
        {
            non_copies.push(chemin.clone());
        }
        if exclu_par(&motifs, chemin) {
            exclus.push(chemin.clone());
        }
    }

    if !non_copies.is_empty() || !exclus.is_empty() {
        panic!(
            "image non constructible (Dockerfile={}, filtre={}) : embarqués hors \
             rust/ sans COPY (absents du contexte de build → include_str! échoue \
             à la compilation) : {non_copies:?} ; exclus par le filtre de \
             contexte : {exclus:?}",
            chemin_dockerfile.display(),
            chemin_filtre.display()
        );
    }
}
