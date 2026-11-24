# syntax=docker/dockerfile:1
# Image de prod du port Rust de « gitcard-api » (phase 6a) — multi-étapes :
# builder rust:1-alpine (target x86_64-unknown-linux-musl par défaut → binaire
# statique), runtime alpine:3.20 non-root (uid 10001) + HEALTHCHECK GET /health
# identique au legacy `apps/api/Dockerfile` (30s/5s/10s/3, wget spider).
# CONTEXT = RACINE du dépôt : `docker buildx build -f rust/Dockerfile <racine>`.
# Le filtre de contexte EFFECTIF est `rust/Dockerfile.dockerignore`
# (BuildKit lit `<Dockerfile>.dockerignore` à côté du Dockerfile choisi).
# Le meta-test `rust/tests/dockerfile_includes.rs` garantit que tout chemin
# embarqué par `include_str!` hors de `rust/` est couvert par un COPY et non
# exclu par le filtre de contexte — les 3 goldens embarqués :
#   octicons.rs:67  → tests/golden/meta/octicons-extract.json
#   github.rs:33    → tests/golden/graphql/user-query.txt
#   intl.rs:17      → tests/golden/intl-tables.json
# (octicons.rs:8 embarque rust/src/octicons.json, couvert par COPY rust/src.)

FROM rust:1-alpine AS builder
ENV CARGO_BUILD_JOBS=2 \
    CARGO_INCREMENTAL=0 \
    CARGO_TERM_COLOR=never \
    HOME=/tmp
WORKDIR /w
# Cache des dépendances avant le code (cargo fetch exige une cible : stub).
COPY rust/Cargo.toml rust/Cargo.lock ./rust/
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=gitcard-p6-reg \
    mkdir -p rust/src && printf 'fn main() {}\n' > rust/src/main.rs && printf '' > rust/src/lib.rs \
    && cd rust && cargo fetch --locked
COPY rust/src ./rust/src
# Goldens embarqués par `include_str!` depuis rust/src (source UNIQUE partagée
# avec les tests, jamais dupliquée sous rust/) — copiés APRÈS `COPY rust/src`
# pour que la couche stub-fetch reste en cache. Sans ces COPY, `cargo build`
# échoue (`couldn't read ../../tests/golden/...`) — le meta-test est ROUGE
# sur ce cas (preuve : DOCKERFILE_PATH sur une variante sans ces lignes).
COPY tests/golden/meta/octicons-extract.json tests/golden/meta/octicons-extract.json
COPY tests/golden/graphql/user-query.txt tests/golden/graphql/user-query.txt
COPY tests/golden/intl-tables.json tests/golden/intl-tables.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=gitcard-p6-reg \
    --mount=type=cache,target=/w/rust/target,id=gitcard-p6-target \
    cd rust && cargo build --release --locked --target x86_64-unknown-linux-musl \
    && cp target/x86_64-unknown-linux-musl/release/gitcard-p0 /usr/local/bin/rilcy-gitcard-api

FROM alpine:3.20
RUN apk add --no-cache ca-certificates curl tzdata \
    && adduser -D -u 10001 app
COPY --from=builder /usr/local/bin/rilcy-gitcard-api /usr/local/bin/rilcy-gitcard-api
ENV PORT=3000
USER app
EXPOSE 3000
# HEALTHCHECK identique au legacy (GET /health, 30s/5s/10s/3, wget spider).
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://127.0.0.1:3000/health || exit 1
ENTRYPOINT ["/usr/local/bin/rilcy-gitcard-api"]
