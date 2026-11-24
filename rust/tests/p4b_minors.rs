//! Tests P4b — mineurs m-2/m-3 de la revue P4, tranchés par l'oracle exécuté
//! (`tests/golden/config-minors/`, legacy Node 26.13.1).
//!
//! m-2 : PORT non entier — zod l'accepte (`z.coerce.number()`), mais le LISTEN
//!       Node jette `RangeError [ERR_SOCKET_BAD_PORT]` et exit 1. Ce test
//!       ÉCHOUERAIT sans le correctif (l'ancien code tronquait `as u16` et
//!       démarrait sur 3000).
//! m-3 : port occupé — ORACLE SURPRISE : Node ≥ 23 active SO_REUSEPORT par
//!       défaut → le legacy Node 26.13.1 NE CRASHE PAS (exit 0, stderr vide,
//!       manifest m3_bind_occupe). Le port (std Rust, pas de SO_REUSEPORT)
//!       échoue le bind → `Error: …` + exit 1 : écart assumé documenté au
//!       rapport (aucun code ne le reproduit — ajouter socket2/SO_REUSEPORT
//!       dévierait davantage).

use gitcard_p0::config::{load_config, port_as_u16};

#[test]
fn m2_port_non_entier_zod_accepte_mais_listen_rejette() {
    // zod (coerce.number) ACCEPTE 3000.5 — parité de la validation :
    let env: Vec<(&str, Option<String>)> = vec![
        ("PAT_1", Some("fake-token-1".into())),
        ("PORT", Some("3000.5".into())),
    ];
    let cfg = load_config(&env).expect("zod accepte PORT=3000.5 (coerce number)");
    assert_eq!(cfg.port, 3000.5, "zod ne rejette PAS les non-entiers");

    // mais le LISTEN doit rejeter (oracle : exit 1 + ERR_SOCKET_BAD_PORT) :
    assert!(
        port_as_u16(cfg.port).is_none(),
        "3000.5 → None (le legacy crasherait au listen)"
    );
    // l'ancien code faisait `3000.5 as u16 == 3000` et DÉMARRAIT :
    assert_ne!(3000.5f64 as u16, 3005, "sanity du troncature historique");
    assert_eq!(
        3000.5f64 as u16, 3000,
        "troncature historique = démarrait à tort"
    );

    // les entiers restent valides :
    assert_eq!(port_as_u16(3000.0), Some(3000));
    assert_eq!(port_as_u16(1.0), Some(1));
    assert_eq!(port_as_u16(65535.0), Some(65535));
    // bornes hors 0..65536 (inatteignables via zod, garde de validatePort) :
    assert_eq!(port_as_u16(0.0), Some(0));
    assert_eq!(port_as_u16(65536.0), None);
    assert_eq!(port_as_u16(-1.0), None);
    assert_eq!(port_as_u16(f64::NAN), None);
    assert_eq!(port_as_u16(f64::INFINITY), None);
}
