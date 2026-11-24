//! Test d'intégration P4b — le port parle à un VRAI Redis.
//!
//! `#[ignore]` : activé à la main DANS LE CONTENEUR de test, sur le réseau
//! interne dédié (aucun accès externe possible) :
//! ```sh
//! docker network create --internal gc-test
//! docker run -d --name gc-redis --network gc-test redis:7.4.12-alpine3.22
//! docker run … --network gc-test -e REDIS_LIVE_URL=redis://gc-redis:6379 \
//!   … cargo test --test redis_live -- --ignored
//! ```
//! Preuves : enveloppe `{"value","expires"}` byte-exact lue BRUT dans Redis,
//! PTTL ≈ 14400000 ms (unité PX ms de l'oracle), hit L2 = chaîne telle quelle,
//! flush (SCAN+UNLINK) sur un serveur avec clés.
use gitcard_p0::cache::{L2Cache, RedisL2};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn raw_cmd(addr: &str, parts: &[&str]) -> String {
    let mut sock = TcpStream::connect(addr).await.expect("connect redis");
    let mut req = format!("*{}\r\n", parts.len()).into_bytes();
    for p in parts {
        let b = p.as_bytes();
        req.extend_from_slice(format!("${}\r\n", b.len()).as_bytes());
        req.extend_from_slice(b);
        req.extend_from_slice(b"\r\n");
    }
    sock.write_all(&req).await.expect("write");
    // Redis ne ferme JAMAIS la connexion → lecture avec deadline, terminée par
    // le framing RESP2 : `$<len>\r\n<bytes>\r\n` ou ligne simple `<x>\r\n`.
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match tokio::time::timeout(Duration::from_millis(400), sock.read(&mut chunk)).await {
            Ok(Ok(0)) | Err(_) | Ok(Err(_)) => break,
            Ok(Ok(n)) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.starts_with(b"$-1\r\n") {
                    break;
                }
                if buf.starts_with(b"$") {
                    if let Some(head_end) = find_sub(&buf, b"\r\n") {
                        if let Some(len) = std::str::from_utf8(&buf[1..head_end])
                            .ok()
                            .and_then(|s| s.parse::<usize>().ok())
                        {
                            let want = head_end + 2 + len + 2;
                            if buf.len() >= want {
                                break;
                            }
                        }
                    }
                } else if buf.ends_with(b"\r\n") {
                    break;
                }
            }
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[tokio::test]
#[ignore = "réseau interne dédié : REDIS_LIVE_URL + redis:7.4.12-alpine3.22 (voir en-tête)"]
async fn redis_live_envelope_ttl_hit_et_flush() {
    let url = std::env::var("REDIS_LIVE_URL")
        .expect("REDIS_LIVE_URL requis (conteneur redis:7.4.12-alpine3.22, réseau interne)");
    let addr = url
        .strip_prefix("redis://")
        .expect("redis://host:port")
        .to_string();

    let l2 = RedisL2::new(&url).expect("client");
    let key = "svg:stats:liveprobe:abcdef01";
    let full = format!("keyv::keyv:{key}");

    // SET → enveloppe byte-exact + PX ms (oracle : SET … PX 14400000)
    l2.set(key.into(), "<svg-live/>".into(), 14400.0).await;
    let raw = raw_cmd(&addr, &["GET", &full]).await;
    // réponse RESP2 : `$<len>\r\n<octets de l'enveloppe>\r\n`
    let (head, rest) = raw.split_once("\r\n").expect("bulk head");
    assert!(head.starts_with('$'), "bulk attendu, reçu : {head}");
    let len: usize = head[1..].parse().expect("len octets");
    assert_eq!(rest[..len.min(rest.len())].len(), len, "enveloppe complète");
    let envelope: serde_json::Value = serde_json::from_str(&rest[..len]).expect("enveloppe JSON");
    assert_eq!(envelope["value"], "<svg-live/>");
    let expires = envelope["expires"].as_i64().expect("epoch ms");
    assert!(
        (expires - now_ms() - 14_400_000).abs() < 5_000,
        "expires = Date.now()+14_400_000 ms (±5 s), reçu {expires}"
    );
    let pttl: String = raw_cmd(&addr, &["PTTL", &full]).await;
    let pttl: i64 = pttl
        .trim()
        .trim_start_matches(':')
        .trim()
        .parse()
        .expect("PTTL ms");
    assert!(
        (14_390_000..=14_400_000).contains(&pttl),
        "PX ms (oracle) : PTTL={pttl}"
    );

    // hit L2 → chaîne telle quelle + TTL restant déduit de `expires`
    let hit = l2.get(key).await.expect("hit");
    assert_eq!(hit.value, "<svg-live/>");
    assert!((14_300.0..=14_400.0).contains(&hit.ttl_secs.expect("ttl")));

    // flush sur un Redis AVEC clés → SCAN + UNLINK réels (oracle e_boot_flush)
    l2.flush().await;
    let gone = raw_cmd(&addr, &["GET", &full]).await;
    assert!(
        gone.starts_with("$-1"),
        "entrée supprimée par le flush, reçu : {gone}"
    );
}
