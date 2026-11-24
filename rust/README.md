# GitCard API — Rust port (`rust/`)

This directory contains the **production API** of GitCard: the binary
`rilcy-gitcard-api` (crate `gitcard-p0`), a strict parity port of the legacy
Node/Hono API in `apps/api`. The legacy service is **not deployed** anymore; it
is kept only as the **parity oracle** that regenerates the byte-exact
expectations in `tests/golden/` (see [Oracles & goldens](#oracles--goldens)).
The Rust port has served production traffic via Coolify since **2026-11-24
20:00**.

## Binary & image

- Target: `x86_64-unknown-linux-musl` (the default target of `rust:1-alpine`)
  → static binary, copied to `/usr/local/bin/rilcy-gitcard-api`
  (`rust/Dockerfile:39-40`).
- Multi-stage image (`rust/Dockerfile`):
  - builder `rust:1-alpine` with `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`,
    `HOME=/tmp` and registry/target cache mounts;
  - runtime `alpine:3.20` + `ca-certificates`, `curl`, `tzdata`, **non-root**
    user `app` (uid `10001`), `ENTRYPOINT ["/usr/local/bin/rilcy-gitcard-api"]`,
    `HEALTHCHECK` = `wget --spider http://127.0.0.1:3000/health`
    (30s / 5s / 10s / 3 retries) — same check as the legacy service.
- **The build context is the repository root**, not `rust/`:
  `docker build -f rust/Dockerfile .` from the root. The effective context
  filter is `rust/Dockerfile.dockerignore`: BuildKit reads
  `<Dockerfile>.dockerignore` sitting **next to the chosen Dockerfile**, not the
  root `.dockerignore`. That filter excludes everything heavy (`apps`,
  `packages`, `node_modules`, `rust/target`, most of `tests/golden`) and
  re-includes exactly the 3 goldens embedded by `include_str!` (see below).
- Coolify deployment uses `dockerfile_location=/rust/Dockerfile` (context =
  repo root). The watched paths (`watch_paths`, one entry per line) are the
  files that end up **in the image** — the port sources, the manifest and its
  lockfile, the Dockerfile and its dedicated dockerignore, and the goldens
  (including the 3 embedded by `include_str!`):

  ```
  rust/src/**
  rust/Cargo.toml
  rust/Cargo.lock
  rust/Dockerfile
  rust/Dockerfile.dockerignore
  tests/golden/**
  ```

  A documentation change under `rust/` (this README) or to the Rust tests
  (`rust/tests/**`) touches none of these watched paths, so it does not
  redeploy.
- Container limit **128 Mio without swap** (a full L1 cache measures ≈ 97 Mio
  RSS per the `p4b_l1_bound` probe, plus margin), health check `GET /health`.
  Resting RSS ≈ 3–5 Mio; final image ≈ 24.6 Mo.

## Developer commands — always in a container

`cargo`/`rustc` must **never** run on the host (memory-capped host). All
commands below run in the capped `rust:1-alpine` container, with:

- the **repository root** mounted on `/w` and workdir `/w/rust` (the build and
  tests embed 3 goldens located **outside** `rust/` via `include_str!`;
  mounting only `rust/` breaks them);
- persistent volumes `hermes-cargo-reg` (cargo registry) and
  `hermes-cargo-rustup` (rustup toolchain);
- no swap (`--memory-swap` == `--memory`), 2 CPUs, `CARGO_BUILD_JOBS=2`,
  `CARGO_INCREMENTAL=0`;
- **absolute** `-v` paths — a relative path is silently interpreted as a named
  volume (run from the repo root, or replace `$PWD` with the absolute path);
- no bash in alpine: `sh` POSIX only; at most one cargo build at a time.

First run only (network on, then everything else offline):

```sh
docker run --rm --network bridge --memory=1200m --memory-swap=1200m --cpus=2 \
  -v "$PWD":/w -v hermes-cargo-reg:/usr/local/cargo/registry -v hermes-cargo-rustup:/usr/local/rustup \
  -e HOME=/tmp -e CARGO_TERM_COLOR=never -w /w/rust rust:1-alpine \
  sh -c 'rustup component add rustfmt clippy && cargo fetch --locked'
```

(`rustfmt` and `clippy` are absent from `rust:1-alpine`; add them in this same
networked run.)

Then, all offline (`--network none`):

```sh
# Format check
docker run --rm --network none --memory=1200m --memory-swap=1200m --cpus=2 \
  -v "$PWD":/w -v hermes-cargo-reg:/usr/local/cargo/registry -v hermes-cargo-rustup:/usr/local/rustup \
  -e HOME=/tmp -e CARGO_BUILD_JOBS=2 -e CARGO_INCREMENTAL=0 -e CARGO_TERM_COLOR=never \
  -w /w/rust rust:1-alpine cargo fmt --check

# Lints (deny warnings)
docker run --rm --network none --memory=1200m --memory-swap=1200m --cpus=2 \
  -v "$PWD":/w -v hermes-cargo-reg:/usr/local/cargo/registry -v hermes-cargo-rustup:/usr/local/rustup \
  -e HOME=/tmp -e CARGO_BUILD_JOBS=2 -e CARGO_INCREMENTAL=0 -e CARGO_TERM_COLOR=never \
  -w /w/rust rust:1-alpine cargo clippy -j 2 --all-targets -- -D warnings

# Test suite
docker run --rm --network none --memory=1200m --memory-swap=1200m --cpus=2 \
  -v "$PWD":/w -v hermes-cargo-reg:/usr/local/cargo/registry -v hermes-cargo-rustup:/usr/local/rustup \
  -e HOME=/tmp -e CARGO_BUILD_JOBS=2 -e CARGO_INCREMENTAL=0 -e CARGO_PROFILE_DEV_DEBUG=0 \
  -e CARGO_PROFILE_TEST_DEBUG=0 -e CARGO_TERM_COLOR=never -w /w/rust rust:1-alpine \
  cargo test -j 2

# Release build (what the Dockerfile runs, minus the musl target flag:
# rust:1-alpine targets x86_64-unknown-linux-musl by default)
docker run --rm --network none --memory=1200m --memory-swap=1200m --cpus=2 \
  -v "$PWD":/w -v hermes-cargo-reg:/usr/local/cargo/registry -v hermes-cargo-rustup:/usr/local/rustup \
  -e HOME=/tmp -e CARGO_BUILD_JOBS=2 -e CARGO_INCREMENTAL=0 -e CARGO_TERM_COLOR=never \
  -w /w/rust rust:1-alpine cargo build --release --locked -j 2
```

Expected test result: **106 passed, 0 failed, 2 ignored** (the two manual
probes below); golden corpus **512/512** byte-exact. Peak memory stays under
the 1200 Mio cap (cold `cargo test` ≈ 863 Mio, cold release build ≈ 728 Mio).

## Manual probes — the 2 `#[ignore]` tests

These tests are skipped by `cargo test` and must only be run **by a human, on
purpose**. An agent must never enable live probes / live gates.

1. `rss_borne_defaut_boot_puis_10k_entrees`
   (`rust/tests/p4b_l1_bound.rs:156-167`) — memory probe: reads the container
   RSS (`VmRSS`) at boot, fills the L1 with 10,000 × ~6 KiB entries under the
   default 24 Mio bound, reads RSS again. This is the `docker stats`-
   equivalent evidence behind the 128 Mio production limit. Run:

   ```sh
   # inside the capped test container (same flags as `cargo test` above)
   cargo test --test p4b_l1_bound -- --ignored --nocapture
   # prints RSS_BOOT_KIB=… RSS_AFTER_KIB=…
   ```

2. `redis_live_envelope_ttl_hit_et_flush`
   (`rust/tests/redis_live.rs:73-126`) — integration against a **real Redis**
   on a dedicated **internal** docker network (no external access possible):
   verifies the byte-exact `keyv` envelope `{"value","expires"}`, `PTTL` ≈
   14,400,000 ms (i.e. `SET … PX` in ms), that an L2 hit returns the stored
   string with the remaining TTL, and that the boot flush is `SCAN` +
   `UNLINK` (never `FLUSHDB`). Run (recipe from the test header):

   ```sh
   docker network create --internal gc-test
   docker run -d --name gc-redis --network gc-test redis:7.4.12-alpine3.22
   docker run --rm --network gc-test --memory=1200m --memory-swap=1200m --cpus=2 \
     -v "$PWD":/w -v hermes-cargo-reg:/usr/local/cargo/registry -v hermes-cargo-rustup:/usr/local/rustup \
     -e HOME=/tmp -e CARGO_BUILD_JOBS=2 -e CARGO_INCREMENTAL=0 -e CARGO_TERM_COLOR=never \
     -e REDIS_LIVE_URL=redis://gc-redis:6379 -w /w/rust rust:1-alpine \
     cargo test --test redis_live -- --ignored
   ```

   (registry volume must be warm — run the networked `cargo fetch` once first).

## Environment variables

Read once at boot by `rust/src/config.rs`, which reproduces the legacy
validation (t3-env + zod v4): an empty string counts as unset; an invalid
value prints the legacy block `❌ Invalid environment variables: […]` on stderr
and exits with code 1 (`rust/src/main.rs:24-34`).

| Variable | Required | Default | Role |
| --- | --- | --- | --- |
| `PAT_1` | **Yes** | – | First GitHub personal access token (GraphQL). Missing or empty → boot failure (exit 1). |
| `PAT_2` … `PAT_N` | No | – | Extra tokens, read sequentially (`PAT_2`, `PAT_3`, …); collection stops at the first absent **or empty** variable. The pool rotates automatically. |
| `PORT` | No | `3000` | Listen port (bound on `0.0.0.0`). Coerced number, 1–65535; a non-integer value is rejected at listen time with the legacy `RangeError [ERR_SOCKET_BAD_PORT]` (exit 1). |
| `LOG_LEVEL` | No | `info` | One of `debug`, `info`, `warn`, `error` (validated enum). |
| `REDIS_URL` | No | – (L2 disabled) | Redis URL for the L2 cache. Must be a WHATWG-parseable URL; a non-`redis` scheme fails the boot like the legacy client. |
| `CACHE_TTL` | No | `14400` | TTL in seconds of the `data:{username}` cache entry (minimum 60). |
| `CORS_ORIGIN` | No | – (ACAO `*`) | When set, `Access-Control-Allow-Origin` is emitted only for a matching `Origin` and `vary: Origin` is added; when unset, every response gets `access-control-allow-origin: *`. |
| `SENTRY_DSN` | No | – (Sentry off) | Sentry DSN (WHATWG URL). Without it, Sentry is a no-op. |
| `SENTRY_TRACES_SAMPLE_RATE` | No | `0.1` | HTTP transaction sampling rate, 0–1. |
| `SENTRY_ENVIRONMENT` | No | `production` | Sentry environment tag. |
| `L1_MAX_MIB` | No | `24` (`0` = unbounded) | Byte-weight bound of the in-memory L1 cache: full entries are evicted LRU-by-weight instead of growing without limit (the legacy was unbounded). Port-specific protective variable, read outside the zod schema; an invalid value falls back to the default. |

## HTTP surface

Routes (`rust/src/app.rs:770-806`) — GET only (any other method → 404 JSON
`{"error":"Not found"}`; `HEAD` is dispatched to the GET handler with an empty
body and no `content-length`; `OPTIONS` is answered 204 before routing on any
path):

| Method & path | Role |
| --- | --- |
| `GET /health`, `GET /healthz` | Health probe → `{"status":"ok","timestamp":"<ISO 8601, ms UTC>"}` |
| `GET /api/themes` | JSON array of the built-in theme names |
| `GET /api/data/{username}` | Raw GitHub GraphQL data as JSON (cached under `data:{username}`); **rate-limited** (see below) |
| `GET /stats/{username}` | Stats card (SVG) |
| `GET /stats/{username}/streak` | Streak card (SVG) |
| `GET /stats/{username}/top-langs` | Top-languages card (SVG) |
| `GET /stats/{username}/profile` | Profile card (SVG) |
| any other path | 404 JSON `{"error":"Not found"}` |

Card failures mirror the legacy: unknown user → 404 error SVG, GitHub rate
limit → 429 SVG, all PATs rate-limited → 429 SVG, other errors → 502 SVG.
Error SVGs are **never cached**. SVG responses carry
`Cache-Control: public, max-age=N, s-maxage=N, stale-while-revalidate=86400`
plus a strong `ETag` (MD5 hex of the body, quoted).

### Rate limiter (only on `/api/data*`)

Port of the legacy `hono-rate-limiter` (`rust/src/ratelimit.rs:39-42`,
`rust/src/app.rs:296-386`):

- window **60 s**, limit **30** hits per key; key = `cf-connecting-ip`
  header, else the client IP, else `unknown`;
- every response on that path carries draft-6 headers
  (`ratelimit-limit`, `ratelimit-policy: 30;w=60`, `ratelimit-remaining`,
  `ratelimit-reset` — absent when 0);
- the 31st hit in a window → 429 `text/plain` "Too many requests, please try
  again later." + `Retry-After`.

### Cache (L1 memory + L2 Redis)

- **L1**: in-process `moka` cache with per-entry TTL
  (`CACHE_TTL` for data, the card's `cacheSeconds` for SVGs) and a byte-weight
  bound (`L1_MAX_MIB`, default 24 Mio, `0` = unbounded = legacy parity):
  eviction is LRU by weight instead of unbounded growth
  (`rust/src/cache.rs:326-345`).
- **L2**: Redis when `REDIS_URL` is set, else a no-op. Commands are pinned
  byte-exact against the legacy: `GET`, `SET … PX`, `UNLINK`,
  `SCAN … MATCH keyv::* … COUNT … TYPE` — never `FLUSHDB`/`FLUSHALL`
  (`rust/src/cache.rs:128-290`).
- At boot the cache is flushed fire-and-forget (Redis SCAN/UNLINK never delay
  the listen; `rust/src/main.rs:91-95`).
- Cache keys: `data:{username}` (GitHub payload) and
  `svg:{prefix}:{username}:{hash8}` (rendered card, 8-hex hash of the query
  options). Error SVGs are never cached.

### Logging

**No per-request logging** (parity with the legacy). Errors go to **stderr
only** (`console.error` equivalents: handler failures, cache fetch errors,
boot crashes). Sentry (optional, `SENTRY_DSN`) adds a panic hook, sampled HTTP
transactions and a capture of every 5xx as `[web] METHOD /path -> code`
(without query string); without a DSN all Sentry layers are no-ops and
responses are unchanged.

## Oracles & goldens

`tests/golden/` holds byte-exact expectations **produced by running the legacy
Node app offline** — never by guessing. The `scripts/gen-*.mjs` runners execute
the legacy (Node v26.13.1) inside a `node:26.13.1-trixie-slim` container with
`--network none`, never via `pnpm`, never on the host:

- `scripts/gen-golden.mjs` — golden corpus v1, Intl tables, libm sweep,
  octicons (fixtures 100% fictional);
- `scripts/gen-graphql.mjs` — GraphQL client expectations (GitHub `fetch`
  stubbed with local fixtures);
- `scripts/gen-http-golden.mjs` — HTTP oracles (raw-socket captures of the
  legacy server on 127.0.0.1); modes selected by `GEN_HTTP_MODE`;
- `scripts/gen-ratelimit-golden.mjs` — rate-limiter headers and 429 behaviour;
- `scripts/gen-redis-golden.mjs` — byte-exact Redis command log against a fake
  RESP server;
- `scripts/probe-minors.mjs` — legacy verdicts for edge cases (non-integer
  `PORT`, port already in use);
- `scripts/flood.mjs` — load generator (phase 6b memory measurements, internal
  network only).

Three goldens are **embedded in the binary** with `include_str!` because the
code needs them at runtime
(`tests/golden/meta/octicons-extract.json`,
`tests/golden/graphql/user-query.txt`, `tests/golden/intl-tables.json` —
single source shared with the tests, never duplicated under `rust/`). The
meta-test `rust/tests/dockerfile_includes.rs` guarantees that every embedded
path outside `rust/` is covered by a `COPY` in `rust/Dockerfile` and not
excluded by `rust/Dockerfile.dockerignore` — the image build breaks otherwise.

## Production (Coolify)

- Deployed since **2026-11-24 20:00**; `dockerfile_location=/rust/Dockerfile`
  (build context = repository root).
- Container: **128 Mio, no swap** (`--memory-swap` = `--memory`) — a full L1
  cache measures ≈ 97 Mio RSS (probe `p4b_l1_bound`), the limit leaves the
  margin; resting RSS ≈ 3–5 Mio.
- Health check: `GET /health` (mirrored by the image `HEALTHCHECK`).
- `watch_paths` (one entry per line): `rust/src/**`, `rust/Cargo.toml`,
  `rust/Cargo.lock`, `rust/Dockerfile`, `rust/Dockerfile.dockerignore`,
  `tests/golden/**` — the files that end up **in the image**, so a change to
  the port sources, the manifest/lockfile, the Dockerfile or the goldens
  (including the 3 embedded by `include_str!`) rebuilds the image; a
  documentation change under `rust/` (this README) and the Rust tests
  (`rust/tests/**`) do not redeploy.
- Same listen port, health path and environment variable names as the legacy
  service: the Coolify environment is unchanged.
