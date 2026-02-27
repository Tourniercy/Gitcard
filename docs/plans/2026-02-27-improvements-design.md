# Improvements Design — Iteration 2

Date: 2026-02-27

## Overview

Four improvement areas for the github-stats-cards project, building on the completed scaffold.

---

## 1. Request Metrics (In-Memory + Protected /metrics)

### Architecture

```
src/
├── metrics/
│   ├── store.ts          # In-memory counters (module singleton)
│   └── middleware.ts      # Hono middleware that records per-request data
├── routes/
│   └── metrics.ts        # GET /metrics (bearer auth protected)
```

### What it tracks

- Request count per route pattern (`/:username`, `/:username/streak`, etc.)
- Latency per route (p50, p95, p99) — rolling window of last 1000 samples per route
- Status code distribution per route
- Error rate per route (4xx + 5xx / total)
- Cache hit/miss counters + hit rate
- Uptime + memory usage

### Protection

Hono's built-in `bearerAuth` middleware from `hono/bearer-auth`. New env var: `METRICS_TOKEN` (optional — if not set, `/metrics` returns 404).

### Response format

JSON. Easy to read with `curl`, easy to parse later when we add Prometheus.

### Future upgrade path

Swap the custom store for `@hono/prometheus` + `prom-client`. Change output format from JSON to Prometheus text. The middleware pattern stays the same.

---

## 2. Caching (cacheable)

### Migration

Replace `lru-cache` + `ioredis` + custom glue with `cacheable` + `@keyv/redis`.

```
Before:                          After:
├── cache/                       ├── cache/
│   ├── index.ts (interface)     │   └── index.ts (Cache interface + cacheable factory)
│   ├── memory.ts (lru-cache)
│   └── redis.ts (ioredis)
```

### Key decisions

- **3 files → 1 file.** The `Cache` interface stays; the factory uses `cacheable` internally.
- **Deps:** Remove `lru-cache` + `ioredis`. Add `cacheable` + `@keyv/redis`.
- **Redis client:** Switches from `ioredis` to `@redis/client` (used internally by `@keyv/redis`). Our usage is simple get/set with TTL — clean swap.
- **Graceful fallback:** `nonBlocking: true` — Redis failures silently fall back to memory.
- **TTL:** `cacheable` uses milliseconds; our interface uses seconds. Adapter multiplies by 1000.
- **Miss value:** `cacheable` returns `undefined`; our interface returns `null`. Adapter normalizes with `?? null`.
- **Cache hit/miss tracking:** Cache factory increments metrics counters from `metrics/store.ts`.

### What stays the same

- `Cache` interface contract: `get(key): Promise<string | null>`, `set(key, value, ttlSeconds): Promise<void>`
- Cache key format: `card:{type}:{username}:{paramsHash}`
- All route handlers unchanged — they consume the `Cache` interface, not the implementation

---

## 3. GraphQL Fetcher (Improve Error Handling)

### Decision: keep raw fetch

Research showed `@octokit/graphql` cannot expose response headers (`x-ratelimit-remaining`, `x-ratelimit-reset`). The maintainers explicitly declined to add this. Our PAT pool relies on these headers. Raw `fetch()` stays.

### Changes

- **Typed error classes:** `GitHubNotFoundError`, `GitHubRateLimitError`, `GitHubApiError` — each carries structured data (status code, rate limit reset time, message)
- **Rate limit info extraction:** Parse `x-ratelimit-remaining` and `x-ratelimit-reset` from response headers, pass to PAT pool for smarter exhaustion tracking
- **Cleaner response validation:** Better null/undefined checks on GraphQL response data

### What stays the same

- Raw `fetch()` — no new dependency
- Single GraphQL query per request
- Query string template literals
- PAT pool integration pattern
- All three extract functions (`extractStats`, `extractStreak`, `extractLanguages`)

---

## 4. Config (@t3-oss/env-core)

### Migration

Replace hand-rolled Zod validation with `@t3-oss/env-core`'s `createEnv()`.

### Key decisions

- **Static vars** (`PORT`, `LOG_LEVEL`, `REDIS_URL`, `CACHE_TTL`, `METRICS_TOKEN`) validated via `createEnv()` with Zod schemas
- **`emptyStringAsUndefined: true`** — treats `PORT=` same as missing, so `.default()` works correctly
- **`PAT_1` enforced** in the static schema (guarantees at least one PAT)
- **`METRICS_TOKEN`** — new optional env var for the metrics endpoint
- **`collectPats()` loop stays** — `@t3-oss/env-core` can't handle dynamic `PAT_N` patterns
- **`runtimeEnv: process.env`** — works in both Vite dev and production

### Deps

Add `@t3-oss/env-core`. Zod stays (peer dep).

---

## Dependency Changes Summary

### Add

- `cacheable`
- `@keyv/redis`
- `@t3-oss/env-core`

### Remove

- `lru-cache`
- `ioredis`

### No new deps for

- Metrics (custom middleware, `hono/bearer-auth` is built into Hono)
- GraphQL fetcher (stays raw `fetch()`)

---

## Implementation Priority

1. **Config** — add `METRICS_TOKEN` env var (other tasks depend on it)
2. **Metrics** — new files, no existing code modified except `index.ts` route registration
3. **Caching** — replace implementation behind stable interface
4. **GraphQL fetcher** — error handling cleanup, no interface changes
