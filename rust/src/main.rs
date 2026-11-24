//! Binaire gitcard-api (phase 4) — équivalent du boot legacy :
//! `loadConfig()` (crash zod au boot, stderr identique, exit 1) → pool PAT →
//! cache (L1 moka, L2 no-op ; Redis phase 4b) → `cache.flush()` fire-and-forget
//! (index.ts:20-22) → serve HTTP 0.0.0.0:PORT (prod @hono/node-server).
//!
//! Le bind écoute sur le port `Number(PORT)` (défaut 3000, validé 1..65535) ;
//! PORT non entier : le legacy jette `ERR_SOCKET_BAD_PORT` AU LISTEN (pas à la
//! validation) — le port arrondit via un cast, divergence théorique documentée
//! (aucune valeur non entière observable en prod).

use gitcard_p0::app::{build_router, prod_state};
use gitcard_p0::cache::{AppCache, NoopL2};
use gitcard_p0::config::{load_config, OsEnv};
use gitcard_p0::pat_pool::PatPool;
use std::sync::Arc;
use std::time::SystemTime;

#[tokio::main]
async fn main() {
    let config = match load_config(&OsEnv) {
        Ok(c) => c,
        Err(e) => {
            // legacy : `❌ Invalid environment variables: […]` sur stderr PUIS
            // throw `Error: Invalid environment variables` → exit 1 (la pile
            // V8 et la ligne `Node.js v26.13.1` ne sont pas reproduites —
            // écart documenté ; le bloc sémantique est identique à l'octet).
            eprintln!("{}", e.stderr_block());
            std::process::exit(1);
        }
    };

    let pool = match PatPool::new(
        config.pats.clone(),
        Arc::new(|| {
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
        }),
    ) {
        Ok(p) => p,
        Err(e) => {
            // `At least one PAT is required` — inatteignable après la
            // validation zod de PAT_1 (le legacy crasherait pareil au boot)
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let state = prod_state(
        AppCache::new(NoopL2),
        pool,
        config.cache_ttl,
        config.cors_origin.clone(),
    );

    // `cache.flush().catch(err => console.error('Failed to flush cache on startup:', err))`
    // — fire-and-forget au legacy ; no-op observable en phase 4 (L2 absent).
    let flush_state = state.clone();
    tokio::spawn(async move { flush_state.cache.flush().await });

    let port = config.port as u16;
    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };
    let app = build_router(state);
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
