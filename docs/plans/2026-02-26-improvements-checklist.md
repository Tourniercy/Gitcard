# Improvements Checklist

Areas to investigate and potentially adopt for the next iteration of the project.

---

## 1. TypeScript Environment Libraries

**Problem:** We manually read `process.env` and validate with Zod in `config.ts`. There are purpose-built libraries for this.

**Libraries to evaluate:**

| Library | What it does |
|---|---|
| `@t3-oss/env-core` | Type-safe env validation with Zod, splits server/client vars, built for T3 stack but framework-agnostic |
| `znv` | Minimal Zod-based env parsing — `parseEnv(process.env, schema)` |
| `envalid` | Mature env validation with validators, defaults, and TypeScript types |

**What to check:**
- Does it replace our `config.ts` entirely or just simplify it?
- Does it play well with Vite's `loadEnv()` in dev mode?
- Do we still need the PAT_1..PAT_N dynamic collection, or can we rethink that?

---

## 2. Caching Library

**Problem:** We wrote a custom `Cache` interface + LRU wrapper + Redis wrapper. There may be libraries that provide a unified caching layer with multi-tier support out of the box.

**Libraries to evaluate:**

| Library | What it does |
|---|---|
| `cacheable` | Multi-tier caching (memory + Redis) with a single API, TTL, key prefixing, built-in stats |
| `keyv` | Simple key-value store with adapters for Redis, SQLite, Postgres, etc. Unified API. |
| `unstorage` | Universal storage layer from UnJS — supports memory, Redis, filesystem, Cloudflare KV, etc. |
| `hono/cache` | Hono's built-in Cache middleware — may handle response caching at the middleware level |

**What to check:**
- Can we replace `src/cache/` entirely with a library?
- Does it support TTL per key, max entries, and graceful fallback?
- Does it add unnecessary weight for our simple get/set needs?
- Is `hono/cache` middleware sufficient for our SVG response caching use case?

---

## 3. GraphQL Query Library

**Problem:** We build GraphQL queries as raw template strings in `fetchers/github.ts`. This works but has no type safety on the query itself, no validation, and error handling is manual.

**Libraries to evaluate:**

| Library | What it does |
|---|---|
| `graphql-request` | Minimal GraphQL client — typed queries, error handling, retries, headers |
| `gql.tada` | Type-safe GraphQL with automatic TypeScript type inference from queries |
| `@octokit/graphql` | GitHub's official GraphQL client — handles auth, pagination, rate limits natively |

**What to check:**
- `@octokit/graphql` might be the best fit since it's GitHub-specific and handles rate limiting, pagination, and auth headers
- Does it replace our PAT pool logic or work alongside it?
- Does `gql.tada` give us type safety on query responses without a codegen step?
- Do any of these bring in heavy dependencies we don't want?

---

## 4. Request Metrics & Analytics

**Problem:** We have no visibility into how endpoints are being used — no request counts, no latency tracking, no error rates.

**What we want:**
- Request count per endpoint (stats / streak / top-langs / health)
- Response time (p50, p95, p99)
- Cache hit/miss ratio
- Error rate by type (404, 429, 502)
- Per-username request frequency (to detect abuse)

**Approaches to evaluate:**

| Approach | Pros | Cons |
|---|---|---|
| **Hono middleware + in-memory counters** | Zero deps, simple, export via `/metrics` endpoint | Lost on restart, no persistence |
| **Prometheus + prom-client** | Industry standard, Grafana dashboards, alerting | Needs Prometheus server in docker compose |
| **Hono + `hono-pino` (structured logging)** | Structured JSON logs, parse with any log tool | No dashboards without extra tooling |
| **OpenTelemetry (`@opentelemetry/sdk-node`)** | Traces + metrics + logs, vendor-neutral | Heavy setup, overkill for small service |

**Recommended investigation order:**
1. Start with a simple Hono middleware that tracks counts + latency in-memory, exposed via `GET /metrics`
2. If we need persistence/dashboards, add Prometheus (`prom-client`) + Grafana to docker compose
3. Structured logging with `hono-pino` is orthogonal and can be added alongside either approach

**What to check:**
- Does Hono have a built-in timing/metrics middleware?
- Can we add Prometheus to docker compose without complicating the deployment?
- What's the minimal setup for a `/metrics` endpoint with request counts and latency?

---

## Priority Order

1. **Metrics** — We're blind without them. Start with simple in-memory counters.
2. **GraphQL library** — `@octokit/graphql` could simplify auth + rate limit handling significantly.
3. **Caching library** — Our current implementation works. Only switch if a library gives us meaningful gains.
4. **TypeScript env** — Low priority. Current `config.ts` is small and works fine.
