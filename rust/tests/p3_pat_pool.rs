//! P3 — pool de jetons PAT (port de `utils/pat-pool.ts`, comportements des
//! tests legacy `pat-pool.test.ts` + cooldown horaire à horloge INJECTABLE —
//! aucune lecture de l'heure système dans la logique testée).

use gitcard_p0::pat_pool::{collect_pats, PatPool, PatPoolError};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn clock_at(start: u64) -> (Arc<AtomicU64>, Arc<dyn Fn() -> u64 + Send + Sync>) {
    let cell = Arc::new(AtomicU64::new(start));
    let read = {
        let cell = cell.clone();
        Arc::new(move || cell.load(Ordering::SeqCst)) as Arc<dyn Fn() -> u64 + Send + Sync>
    };
    (cell, read)
}

#[test]
fn pool_vide_leve_le_message_legacy() {
    let (_, now) = clock_at(1000);
    let err = PatPool::new(vec![], now).expect_err("vide doit lever");
    assert_eq!(err, PatPoolError::Empty);
    assert_eq!(err.to_string(), "At least one PAT is required");
}

#[test]
fn round_robin_dans_l_ordre() {
    let (_, now) = clock_at(1000);
    let mut pool = PatPool::new(vec!["a".into(), "b".into(), "c".into()], now).expect("pool");
    assert_eq!(pool.get_next_token().expect("t"), "a");
    assert_eq!(pool.get_next_token().expect("t"), "b");
    assert_eq!(pool.get_next_token().expect("t"), "c");
    assert_eq!(pool.get_next_token().expect("t"), "a"); // modulo
}

#[test]
fn jeton_unique() {
    let (_, now) = clock_at(1000);
    let mut pool = PatPool::new(vec!["solo".into()], now).expect("pool");
    for _ in 0..3 {
        assert_eq!(pool.get_next_token().expect("t"), "solo");
    }
}

#[test]
fn saute_les_jetons_epuises() {
    let (_, now) = clock_at(1000);
    let mut pool = PatPool::new(vec!["a".into(), "b".into()], now).expect("pool");
    pool.mark_exhausted("a");
    assert_eq!(pool.get_next_token().expect("t"), "b");
    assert_eq!(pool.get_next_token().expect("t"), "b"); // a reste en cooldown
}

#[test]
fn tous_epuises_leve_le_message_legacy() {
    let (_, now) = clock_at(1000);
    let mut pool = PatPool::new(vec!["a".into(), "b".into()], now).expect("pool");
    pool.mark_exhausted("a");
    pool.mark_exhausted("b");
    let err = pool.get_next_token().expect_err("tous épuisés doit lever");
    assert_eq!(err, PatPoolError::AllRateLimited);
    assert_eq!(err.to_string(), "All PAT tokens are rate-limited");
}

#[test]
fn cooldown_d_une_heure_puis_remise_en_service() {
    let (cell, now) = clock_at(1000);
    let mut pool = PatPool::new(vec!["a".into()], now).expect("pool");
    pool.mark_exhausted("a");
    // avant l'expiration : indisponible
    assert!(pool.get_next_token().is_err());
    // exactement à t + 3600_000 ms : disponible (Date.now() >= until)
    cell.store(1000 + 60 * 60 * 1000, Ordering::SeqCst);
    assert_eq!(pool.get_next_token().expect("t"), "a");
}

#[test]
fn jeton_expire_sort_de_la_map() {
    let (cell, now) = clock_at(1000);
    let mut pool = PatPool::new(vec!["a".into(), "b".into()], now).expect("pool");
    pool.mark_exhausted("a");
    cell.store(1000 + 60 * 60 * 1000 + 1, Ordering::SeqCst);
    // le premier appel consomme « a » (expiré, retiré de la map), le suivant
    // reprend le cycle à « b » puis « a » — preuve que l'entrée a disparu.
    assert_eq!(pool.get_next_token().expect("t"), "a");
    assert_eq!(pool.get_next_token().expect("t"), "b");
    assert_eq!(pool.get_next_token().expect("t"), "a");
}

#[test]
fn mark_exhausted_hors_cycle_ne_casse_pas_le_round_robin() {
    let (_, now) = clock_at(1000);
    let mut pool = PatPool::new(vec!["x".into(), "y".into(), "z".into()], now).expect("pool");
    assert_eq!(pool.get_next_token().expect("t"), "x");
    pool.mark_exhausted("z"); // pas encore consommé
    assert_eq!(pool.get_next_token().expect("t"), "y");
    assert_eq!(pool.get_next_token().expect("t"), "x"); // z sauté
}

#[test]
fn collect_pats_sequentiels_depuis_1_arret_au_premier_absent() {
    let env = |k: &str| match k {
        "PAT_1" => Some("t1".into()),
        "PAT_2" => Some("t2".into()),
        "PAT_3" => Some("t3".into()),
        _ => None,
    };
    assert_eq!(collect_pats(env), vec!["t1", "t2", "t3"]);
    assert!(collect_pats(|_| None).is_empty(), "PAT_1 absent → vide");
    let only_one = |k: &str| {
        if k == "PAT_1" {
            Some("t1".into())
        } else {
            None
        }
    };
    assert_eq!(
        collect_pats(only_one),
        vec!["t1"],
        "arrêt au premier absent"
    );
    // Revue P3 M-C : le test « with_empty » d'origine était une copie de
    // only_one — il teste maintenant ce que son nom annonce : collect_pats
    // quand PAT_1 est DÉFINI MAIS VIDE (config.ts `!pat` → break immédiat).
    let pat1_empty = |k: &str| match k {
        "PAT_1" => Some(String::new()),
        "PAT_2" => Some("t2".into()),
        _ => None,
    };
    assert_eq!(
        collect_pats(pat1_empty),
        Vec::<String>::new(),
        "PAT_1 vide → liste vide (break au premier vide)"
    );
    let empty_then = |k: &str| match k {
        "PAT_1" => Some("t1".into()),
        "PAT_2" => Some(String::new()), // vide = absent
        _ => None,
    };
    assert_eq!(collect_pats(empty_then), vec!["t1"]);
}
