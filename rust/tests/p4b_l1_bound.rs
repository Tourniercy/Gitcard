//! Tests P4b — borne protectrice L1 (revue P4 I-1, divergence assumée).
//!
//! Le legacy est NON BORNÉ (`KeyvCacheableMemory`, Map nue) : le port garde la
//! parité par défaut (`AppCache::new` = non borné) mais `main.rs` applique
//! `L1_MAX_MIB` (défaut 24 Mio, `0` = non borné) via `AppCache::with_l1_bound`
//! — poids d'octets (clé + valeur), éviction LRU au lieu d'OOM.

use gitcard_p0::cache::{AppCache, NoopL2};
use gitcard_p0::config::{load_config, OsEnv};

fn value_6k() -> String {
    "x".repeat(6144)
}

fn key_n(i: usize) -> String {
    format!("svg:stats:load:{i:05}")
}

#[tokio::test]
async fn borne_1mib_le_poids_reste_borne_et_la_majorite_est_evincee() {
    let cache = AppCache::with_l1_bound(NoopL2, 1024 * 1024); // L1_MAX_MIB=1
    for i in 0..10_000usize {
        cache.set(&key_n(i), value_6k(), 14400.0).await;
    }
    // drain complet des évictions en attente (moka traite par lots)
    let mut prev = u64::MAX;
    loop {
        cache.run_pending_tasks().await;
        let c = cache.entry_count();
        if c == prev {
            break;
        }
        prev = c;
    }
    // poids par entrée ≈ 6144 + 22 o → capacité 1 Mio ≈ 170 entrées (marge) :
    // LE poids total reste < 1 Mio — la propriété protectrice contre l'OOM.
    let entries = cache.entry_count();
    assert!(
        entries <= 200,
        "poids total resté < 1 Mio : {entries} entrées × ~6 166 o (sans borne : 10 000)"
    );
    // la borne évince RÉELLEMENT la grande majorité des 10 000 entrées
    // (NB : moka évince en TinyLFU/admission, pas en LRU strict — sous
    // fréquences uniformes, les survivantes ne sont pas « les plus récentes »
    // mais un sous-ensemble borné ; divergence documentée au rapport §C —
    // l'invariant protecteur est le POIDS, l'ordre d'éviction est libre car
    // le legacy n'évince jamais du tout).
    let mut evicted = 0;
    for i in 0..10_000usize {
        if cache.get(&key_n(i)).await.is_none() {
            evicted += 1;
        }
    }
    assert!(
        evicted >= 9_800,
        "la borne évince réellement : {evicted}/10 000 évincées"
    );
    // le cache reste fonctionnel après éviction : (re)insertion récente → hit
    cache.set(&key_n(9_999), value_6k(), 14400.0).await;
    assert_eq!(
        cache.get(&key_n(9_999)).await.as_deref(),
        Some(value_6k().as_str()),
        "entrée récente servie"
    );
}

#[tokio::test]
async fn borne_zero_non_borne_parite_stricte_legacy() {
    let cache = AppCache::with_l1_bound(NoopL2, 0); // L1_MAX_MIB=0 → parité
    for i in 0..10_000usize {
        cache.set(&key_n(i), value_6k(), 14400.0).await;
    }
    cache.run_pending_tasks().await;
    // entry_count est éventuellement consistant chez moka : la preuve de
    // parité est la PRÉSENCE de l'entrée la plus ancienne (aucune éviction).
    let entries = cache.entry_count();
    assert!(
        (9_900..=10_000).contains(&entries),
        "sans borne : aucune éviction significative (= Map du legacy), {entries}"
    );
    assert!(
        cache.get(&key_n(0)).await.is_some(),
        "entrée ancienne conservée"
    );
    assert!(cache.get(&key_n(9_999)).await.is_some());
}

/// `L1_MAX_MIB` : défaut 24 quand absent/invalide, lu tel quel sinon — hors
/// zod (aucune validation legacy à reproduire), voir `Config::l1_max_mib`.
#[test]
fn config_l1_max_mib_defaut_24_et_parsing() {
    let base: Vec<(&str, Option<String>)> = vec![("PAT_1", Some("fake-token-1".into()))];
    assert_eq!(
        load_config(&base).expect("cfg").l1_max_mib,
        24,
        "absent → défaut 24"
    );

    let invalid: Vec<(&str, Option<String>)> = vec![
        ("PAT_1", Some("fake-token-1".into())),
        ("L1_MAX_MIB", Some("abc".into())),
    ];
    assert_eq!(
        load_config(&invalid).expect("cfg").l1_max_mib,
        24,
        "invalide → défaut 24"
    );

    let zero: Vec<(&str, Option<String>)> = vec![
        ("PAT_1", Some("fake-token-1".into())),
        ("L1_MAX_MIB", Some("0".into())),
    ];
    assert_eq!(
        load_config(&zero).expect("cfg").l1_max_mib,
        0,
        "0 → non borné (parité)"
    );

    let five: Vec<(&str, Option<String>)> = vec![
        ("PAT_1", Some("fake-token-1".into())),
        ("L1_MAX_MIB", Some("5".into())),
    ];
    assert_eq!(load_config(&five).expect("cfg").l1_max_mib, 5);

    // les oracles zod 19 (tests/golden/http/config) restent intacts : la
    // lecture est HORS de la chaîne zod — une env inconnue n'émet aucun issue.
    let with_unknown: Vec<(&str, Option<String>)> = vec![
        ("PAT_1", Some("fake-token-1".into())),
        ("L1_MAX_MIB", Some("24".into())),
        ("CACHE_TTL", Some("14400".into())),
    ];
    assert!(load_config(&with_unknown).is_ok());
    let _ = OsEnv; // la prod lit process.env (même code de lecture)
}

fn vm_rss_kib() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("/proc/self/status");
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kib = rest
                .trim()
                .trim_end_matches(" kB")
                .trim()
                .parse::<u64>()
                .expect("VmRSS kib");
            return kib;
        }
    }
    panic!("VmRSS absent");
}

/// Mesure RSS (activée À LA MAIN dans le conteneur, cf. brief C) :
/// `cargo test --test p4b_l1_bound -- --ignored --nocapture`
/// → `RSS_BOOT_KIB=… RSS_AFTER_KIB=…` (borne par défaut 24 Mio, 10 000 × ~6 Ko).
#[tokio::test]
#[ignore = "mesure manuelle conteneur : docker stats équivalent (brief C)"]
async fn rss_borne_defaut_boot_puis_10k_entrees() {
    let boot = vm_rss_kib();
    let cache = AppCache::with_l1_bound(NoopL2, 24 * 1024 * 1024); // défaut
    for i in 0..10_000usize {
        cache.set(&key_n(i), value_6k(), 14400.0).await;
    }
    cache.run_pending_tasks().await;
    let after = vm_rss_kib();
    println!("RSS_BOOT_KIB={boot} RSS_AFTER_KIB={after}");
    assert!(after > 0);
}
