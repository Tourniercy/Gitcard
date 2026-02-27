# Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Upgrade the github-stats-cards project with request metrics, cacheable-based caching, improved fetcher error handling, and @t3-oss/env-core config validation.

**Architecture:** Add a metrics middleware layer that tracks request counts/latency/cache performance, exposed via a bearer-auth protected `/metrics` endpoint. Replace the custom cache layer (lru-cache + ioredis) with `cacheable` + `@keyv/redis`. Improve the GitHub fetcher with typed error classes and rate limit header extraction. Migrate config to `@t3-oss/env-core`.

**Tech Stack:** Hono 4, cacheable, @keyv/redis, @t3-oss/env-core, Zod, vitest, hono/bearer-auth

**Context7 Requirement:** Before writing or modifying any code that uses a library, consult Context7 MCP to verify the API. Training data is outdated for cacheable, @t3-oss/env-core, and recent Hono changes.

---

## Task 1: Install New Dependencies

**Files:**

- Modify: `package.json`

**Step 1: Install new deps**

Run:

```bash
pnpm add cacheable @keyv/redis @t3-oss/env-core
```

**Step 2: Remove old deps**

Run:

```bash
pnpm remove lru-cache ioredis
```

**Step 3: Verify install**

Run: `pnpm typecheck`
Expected: Errors in `src/cache/memory.ts` and `src/cache/redis.ts` (expected — they import removed packages). No other unexpected errors.

**Step 4: Commit**

```bash
git add package.json pnpm-lock.yaml
git commit -m "chore: swap lru-cache + ioredis for cacheable, add @t3-oss/env-core"
```

---

## Task 2: Migrate Config to @t3-oss/env-core

**Files:**

- Modify: `src/config.ts`
- Modify: `src/config.test.ts`
- Modify: `.env.example`

**Step 1: Write the updated tests**

Replace `src/config.test.ts` with:

```typescript
import { beforeEach, describe, expect, it, vi } from 'vitest';

describe('loadConfig', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.unstubAllEnvs();
  });

  it('loads config with required PAT_1', async () => {
    vi.stubEnv('PAT_1', 'ghp_test123');
    vi.stubEnv('PORT', '4000');

    const { loadConfig } = await import('./config');
    const config = loadConfig();

    expect(config.pats).toEqual(['ghp_test123']);
    expect(config.port).toBe(4000);
    expect(config.redisUrl).toBeUndefined();
    expect(config.cacheTtl).toBe(14400);
    expect(config.logLevel).toBe('info');
    expect(config.metricsToken).toBeUndefined();
  });

  it('collects multiple PATs', async () => {
    vi.stubEnv('PAT_1', 'a');
    vi.stubEnv('PAT_2', 'b');
    vi.stubEnv('PAT_3', 'c');

    const { loadConfig } = await import('./config');
    const config = loadConfig();

    expect(config.pats).toEqual(['a', 'b', 'c']);
  });

  it('throws if PAT_1 is missing', async () => {
    vi.stubEnv('PAT_1', '');

    const { loadConfig } = await import('./config');
    expect(() => loadConfig()).toThrow();
  });

  it('loads METRICS_TOKEN when set', async () => {
    vi.stubEnv('PAT_1', 'ghp_test');
    vi.stubEnv('METRICS_TOKEN', 'secret-metrics-token');

    const { loadConfig } = await import('./config');
    const config = loadConfig();

    expect(config.metricsToken).toBe('secret-metrics-token');
  });

  it('uses default values', async () => {
    vi.stubEnv('PAT_1', 'ghp_test');

    const { loadConfig } = await import('./config');
    const config = loadConfig();

    expect(config.port).toBe(3000);
    expect(config.cacheTtl).toBe(14400);
    expect(config.logLevel).toBe('info');
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/config.test.ts`
Expected: FAIL — `metricsToken` property doesn't exist yet on `AppConfig`.

**Step 3: Rewrite config.ts with @t3-oss/env-core**

> **Context7:** Check `@t3-oss/env-core` for `createEnv()` API — `server`, `runtimeEnv`, `emptyStringAsUndefined`.

Replace `src/config.ts` with:

```typescript
import { createEnv } from '@t3-oss/env-core';
import { z } from 'zod';

const env = createEnv({
  server: {
    PORT: z.coerce.number().min(1).max(65535).default(3000),
    LOG_LEVEL: z.enum(['debug', 'info', 'warn', 'error']).default('info'),
    REDIS_URL: z.string().url().optional(),
    CACHE_TTL: z.coerce.number().min(60).default(14400),
    METRICS_TOKEN: z.string().min(1).optional(),
    PAT_1: z.string().min(1, 'At least PAT_1 must be set'),
  },
  runtimeEnv: process.env,
  emptyStringAsUndefined: true,
});

function collectPats(): string[] {
  const pats: string[] = [];
  for (let i = 1; ; i++) {
    const pat = process.env[`PAT_${i}`];
    if (!pat) break;
    pats.push(pat);
  }
  return pats;
}

export interface AppConfig {
  pats: string[];
  port: number;
  redisUrl?: string;
  cacheTtl: number;
  logLevel: 'debug' | 'info' | 'warn' | 'error';
  metricsToken?: string;
}

export function loadConfig(): AppConfig {
  return {
    pats: collectPats(),
    port: env.PORT,
    redisUrl: env.REDIS_URL,
    cacheTtl: env.CACHE_TTL,
    logLevel: env.LOG_LEVEL,
    metricsToken: env.METRICS_TOKEN,
  };
}
```

**Step 4: Update .env.example**

Add to `.env.example`:

```
# Metrics (optional — if not set, /metrics endpoint returns 404)
# METRICS_TOKEN=your-secret-token-here
```

**Step 5: Run tests to verify they pass**

Run: `pnpm vitest run src/config.test.ts`
Expected: All 5 tests PASS.

**Step 6: Run full check**

Run: `pnpm typecheck`
Expected: May have errors in cache files (expected — those are rewritten in Task 5). Config-related files should be clean.

**Step 7: Commit**

```bash
git add src/config.ts src/config.test.ts .env.example
git commit -m "refactor: migrate config to @t3-oss/env-core, add METRICS_TOKEN"
```

---

## Task 3: Metrics Store

**Files:**

- Create: `src/metrics/store.ts`
- Create: `src/metrics/store.test.ts`

**Step 1: Write the failing tests**

Create `src/metrics/store.test.ts`:

```typescript
import { beforeEach, describe, expect, it } from 'vitest';
import { recordRequest, getSnapshot, cacheCounters, resetMetrics } from './store';

describe('metrics store', () => {
  beforeEach(() => {
    resetMetrics();
  });

  it('records a request and returns it in snapshot', () => {
    recordRequest('/:username', 50, 200);
    const snap = getSnapshot();

    expect(snap.routes['/:username']).toBeDefined();
    expect(snap.routes['/:username']!.count).toBe(1);
    expect(snap.routes['/:username']!.statusCodes[200]).toBe(1);
  });

  it('computes latency percentiles', () => {
    for (let i = 1; i <= 100; i++) {
      recordRequest('/:username', i, 200);
    }
    const snap = getSnapshot();
    const route = snap.routes['/:username']!;

    expect(route.latency.p50).toBe(50);
    expect(route.latency.p95).toBe(95);
    expect(route.latency.p99).toBe(99);
  });

  it('tracks multiple routes independently', () => {
    recordRequest('/:username', 10, 200);
    recordRequest('/:username/streak', 20, 200);
    const snap = getSnapshot();

    expect(Object.keys(snap.routes)).toHaveLength(2);
    expect(snap.routes['/:username']!.count).toBe(1);
    expect(snap.routes['/:username/streak']!.count).toBe(1);
  });

  it('computes error rate', () => {
    recordRequest('/:username', 10, 200);
    recordRequest('/:username', 10, 200);
    recordRequest('/:username', 10, 404);
    recordRequest('/:username', 10, 502);
    const snap = getSnapshot();

    expect(snap.routes['/:username']!.errorRate).toBe(0.5);
  });

  it('tracks cache hit/miss counters', () => {
    cacheCounters.hits = 0;
    cacheCounters.misses = 0;

    cacheCounters.hits += 3;
    cacheCounters.misses += 1;

    const snap = getSnapshot();
    expect(snap.cache.hits).toBe(3);
    expect(snap.cache.misses).toBe(1);
    expect(snap.cache.hitRate).toBe(0.75);
  });

  it('reports zero hit rate when no cache activity', () => {
    const snap = getSnapshot();
    expect(snap.cache.hitRate).toBe(0);
  });

  it('bounds latency samples to 1000 per route', () => {
    for (let i = 0; i < 1100; i++) {
      recordRequest('/:username', i, 200);
    }
    const snap = getSnapshot();
    // p50 should reflect the more recent samples (100-1099), not the earliest
    expect(snap.routes['/:username']!.count).toBe(1100);
  });

  it('resets all metrics', () => {
    recordRequest('/:username', 10, 200);
    cacheCounters.hits = 5;
    resetMetrics();

    const snap = getSnapshot();
    expect(Object.keys(snap.routes)).toHaveLength(0);
    expect(snap.cache.hits).toBe(0);
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/metrics/store.test.ts`
Expected: FAIL — module not found.

**Step 3: Implement the metrics store**

Create `src/metrics/store.ts`:

```typescript
interface RouteMetrics {
  count: number;
  latencies: number[];
  statusCodes: Record<number, number>;
}

interface RouteSnapshot {
  count: number;
  latency: { p50: number; p95: number; p99: number };
  statusCodes: Record<number, number>;
  errorRate: number;
}

export interface MetricsSnapshot {
  routes: Record<string, RouteSnapshot>;
  cache: { hits: number; misses: number; hitRate: number };
  uptime: number;
  memoryMb: number;
}

const MAX_LATENCY_SAMPLES = 1000;

const store = new Map<string, RouteMetrics>();

export const cacheCounters = { hits: 0, misses: 0 };

export function recordRequest(route: string, latencyMs: number, status: number): void {
  let m = store.get(route);
  if (!m) {
    m = { count: 0, latencies: [], statusCodes: {} };
    store.set(route, m);
  }
  m.count++;
  if (m.latencies.length >= MAX_LATENCY_SAMPLES) {
    m.latencies.shift();
  }
  m.latencies.push(latencyMs);
  m.statusCodes[status] = (m.statusCodes[status] ?? 0) + 1;
}

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0;
  const idx = Math.floor((sorted.length - 1) * p);
  return sorted[idx]!;
}

export function getSnapshot(): MetricsSnapshot {
  const routes: Record<string, RouteSnapshot> = {};

  for (const [route, m] of store.entries()) {
    const sorted = [...m.latencies].toSorted((a, b) => a - b);
    const errors = Object.entries(m.statusCodes)
      .filter(([s]) => Number(s) >= 400)
      .reduce((sum, [, n]) => sum + n, 0);

    routes[route] = {
      count: m.count,
      latency: {
        p50: percentile(sorted, 0.5),
        p95: percentile(sorted, 0.95),
        p99: percentile(sorted, 0.99),
      },
      statusCodes: m.statusCodes,
      errorRate: m.count > 0 ? errors / m.count : 0,
    };
  }

  const total = cacheCounters.hits + cacheCounters.misses;

  return {
    routes,
    cache: {
      hits: cacheCounters.hits,
      misses: cacheCounters.misses,
      hitRate: total > 0 ? cacheCounters.hits / total : 0,
    },
    uptime: process.uptime(),
    memoryMb: Math.round((process.memoryUsage().rss / 1024 / 1024) * 100) / 100,
  };
}

export function resetMetrics(): void {
  store.clear();
  cacheCounters.hits = 0;
  cacheCounters.misses = 0;
}
```

**Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/metrics/store.test.ts`
Expected: All 8 tests PASS.

**Step 5: Commit**

```bash
git add src/metrics/store.ts src/metrics/store.test.ts
git commit -m "feat: add in-memory metrics store with request/cache tracking"
```

---

## Task 4: Metrics Middleware

**Files:**

- Create: `src/metrics/middleware.ts`
- Create: `src/metrics/middleware.test.ts`

**Step 1: Write the failing tests**

Create `src/metrics/middleware.test.ts`:

```typescript
import { describe, expect, it, beforeEach } from 'vitest';
import { Hono } from 'hono';
import { metricsMiddleware } from './middleware';
import { getSnapshot, resetMetrics } from './store';

describe('metricsMiddleware', () => {
  beforeEach(() => {
    resetMetrics();
  });

  it('records request count and status', async () => {
    const app = new Hono();
    app.use('*', metricsMiddleware);
    app.get('/health', (c) => c.json({ status: 'ok' }));

    const res = await app.request('/health');
    expect(res.status).toBe(200);

    const snap = getSnapshot();
    expect(snap.routes['/health']).toBeDefined();
    expect(snap.routes['/health']!.count).toBe(1);
    expect(snap.routes['/health']!.statusCodes[200]).toBe(1);
  });

  it('records latency', async () => {
    const app = new Hono();
    app.use('*', metricsMiddleware);
    app.get('/slow', async (c) => {
      await new Promise((r) => setTimeout(r, 10));
      return c.json({ ok: true });
    });

    await app.request('/slow');
    const snap = getSnapshot();
    expect(snap.routes['/slow']!.latency.p50).toBeGreaterThan(0);
  });

  it('records error status codes', async () => {
    const app = new Hono();
    app.use('*', metricsMiddleware);
    app.get('/fail', (c) => c.json({ error: 'not found' }, 404));

    await app.request('/fail');
    const snap = getSnapshot();
    expect(snap.routes['/fail']!.statusCodes[404]).toBe(1);
    expect(snap.routes['/fail']!.errorRate).toBe(1);
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/metrics/middleware.test.ts`
Expected: FAIL — module not found.

**Step 3: Implement the middleware**

> **Context7:** Check Hono for `createMiddleware` from `hono/factory`.

Create `src/metrics/middleware.ts`:

```typescript
import { createMiddleware } from 'hono/factory';
import { recordRequest } from './store';

export const metricsMiddleware = createMiddleware(async (c, next) => {
  const start = performance.now();
  await next();
  const latencyMs = performance.now() - start;
  const route = c.req.routePath ?? c.req.path;
  recordRequest(route, latencyMs, c.res.status);
});
```

**Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/metrics/middleware.test.ts`
Expected: All 3 tests PASS.

**Step 5: Commit**

```bash
git add src/metrics/middleware.ts src/metrics/middleware.test.ts
git commit -m "feat: add Hono metrics middleware for request tracking"
```

---

## Task 5: Metrics Route (Protected)

**Files:**

- Create: `src/routes/metrics.ts`
- Create: `src/routes/metrics.test.ts`

**Step 1: Write the failing tests**

> **Context7:** Check Hono for `bearerAuth` from `hono/bearer-auth`.

Create `src/routes/metrics.test.ts`:

```typescript
import { describe, expect, it, beforeEach } from 'vitest';
import { Hono } from 'hono';
import { createMetricsRoute } from './metrics';
import { resetMetrics, recordRequest, cacheCounters } from '../metrics/store';

describe('metrics route', () => {
  const token = 'test-secret-token';
  let app: Hono;

  beforeEach(() => {
    resetMetrics();
    app = new Hono();
    app.route('', createMetricsRoute(token));
  });

  it('returns 401 without auth header', async () => {
    const res = await app.request('/metrics');
    expect(res.status).toBe(401);
  });

  it('returns 401 with wrong token', async () => {
    const res = await app.request('/metrics', {
      headers: { Authorization: 'Bearer wrong-token' },
    });
    expect(res.status).toBe(401);
  });

  it('returns metrics with valid token', async () => {
    recordRequest('/:username', 50, 200);
    cacheCounters.hits = 10;
    cacheCounters.misses = 5;

    const res = await app.request('/metrics', {
      headers: { Authorization: `Bearer ${token}` },
    });

    expect(res.status).toBe(200);
    const body = await res.json();

    expect(body.routes['/:username']).toBeDefined();
    expect(body.routes['/:username'].count).toBe(1);
    expect(body.cache.hits).toBe(10);
    expect(body.cache.misses).toBe(5);
    expect(body.cache.hitRate).toBeCloseTo(0.6667, 2);
    expect(body.uptime).toBeGreaterThan(0);
    expect(body.memoryMb).toBeGreaterThan(0);
  });
});

describe('metrics route disabled', () => {
  it('returns 404 when no token configured', async () => {
    const app = new Hono();
    app.route('', createMetricsRoute(undefined));

    const res = await app.request('/metrics');
    expect(res.status).toBe(404);
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/routes/metrics.test.ts`
Expected: FAIL — module not found.

**Step 3: Implement the metrics route**

Create `src/routes/metrics.ts`:

```typescript
import { Hono } from 'hono';
import { bearerAuth } from 'hono/bearer-auth';
import { getSnapshot } from '../metrics/store';

export function createMetricsRoute(metricsToken?: string): Hono {
  const app = new Hono();

  if (!metricsToken) {
    app.get('/metrics', (c) => c.json({ error: 'Metrics not configured' }, 404));
    return app;
  }

  app.use('/metrics', bearerAuth({ token: metricsToken }));

  app.get('/metrics', (c) => {
    return c.json(getSnapshot());
  });

  return app;
}
```

**Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/routes/metrics.test.ts`
Expected: All 4 tests PASS.

**Step 5: Commit**

```bash
git add src/routes/metrics.ts src/routes/metrics.test.ts
git commit -m "feat: add bearer-auth protected /metrics endpoint"
```

---

## Task 6: Migrate Cache to cacheable

**Files:**

- Rewrite: `src/cache/index.ts`
- Delete: `src/cache/memory.ts`
- Delete: `src/cache/redis.ts`
- Rewrite: `src/cache/memory.test.ts` → `src/cache/cache.test.ts`

**Step 1: Write the new cache tests**

> **Context7:** Check `cacheable` for constructor options, `get()`, `set()` API, TTL units, `nonBlocking` option.

Create `src/cache/cache.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { createCache } from './index';

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe('cache (cacheable)', () => {
  it('returns null for missing key', async () => {
    const cache = createCache();
    expect(await cache.get('missing')).toBeNull();
  });

  it('stores and retrieves a value', async () => {
    const cache = createCache();
    await cache.set('key', 'value', 3600);
    expect(await cache.get('key')).toBe('value');
  });

  it('expires entries after TTL elapses', async () => {
    const cache = createCache();
    await cache.set('key', 'value', 1); // 1 second TTL
    expect(await cache.get('key')).toBe('value');
    await delay(1100);
    expect(await cache.get('key')).toBeNull();
  });

  it('overwrites existing key', async () => {
    const cache = createCache();
    await cache.set('key', 'old', 3600);
    await cache.set('key', 'new', 3600);
    expect(await cache.get('key')).toBe('new');
  });
});
```

**Step 2: Rewrite src/cache/index.ts**

Replace `src/cache/index.ts` with:

```typescript
import { Cacheable } from 'cacheable';
import KeyvRedis from '@keyv/redis';
import { cacheCounters } from '../metrics/store';

export interface Cache {
  get(key: string): Promise<string | null>;
  set(key: string, value: string, ttlSeconds: number): Promise<void>;
}

export function createCache(redisUrl?: string): Cache {
  const options: ConstructorParameters<typeof Cacheable>[0] = {
    nonBlocking: true,
  };

  if (redisUrl) {
    options.secondary = new KeyvRedis(redisUrl);
  }

  const cache = new Cacheable(options);

  return {
    async get(key: string): Promise<string | null> {
      const value = await cache.get<string>(key);
      if (value !== undefined) {
        cacheCounters.hits++;
        return value;
      }
      cacheCounters.misses++;
      return null;
    },

    async set(key: string, value: string, ttlSeconds: number): Promise<void> {
      await cache.set(key, value, ttlSeconds * 1000);
    },
  };
}
```

**Step 3: Delete old files**

Delete `src/cache/memory.ts` and `src/cache/redis.ts`.
Delete `src/cache/memory.test.ts`.

**Step 4: Run the new cache tests**

Run: `pnpm vitest run src/cache/cache.test.ts`
Expected: All 4 tests PASS.

**Step 5: Run typecheck**

Run: `pnpm typecheck`
Expected: No errors related to cache imports. The `Cache` interface is unchanged so all consumers (`card-factory.ts`, `index.ts`) should be clean.

**Step 6: Commit**

```bash
git add src/cache/index.ts src/cache/cache.test.ts
git rm src/cache/memory.ts src/cache/redis.ts src/cache/memory.test.ts
git commit -m "refactor: replace lru-cache + ioredis with cacheable"
```

---

## Task 7: Typed Error Classes for GitHub Fetcher

**Files:**

- Modify: `src/fetchers/github.ts`
- Modify: `src/fetchers/github.test.ts`

**Step 1: Write the failing tests for new error types**

Add to `src/fetchers/github.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { GitHubApiError, GitHubNotFoundError, GitHubRateLimitError } from './github';

describe('GitHub error classes', () => {
  it('GitHubNotFoundError has correct properties', () => {
    const err = new GitHubNotFoundError('octocat');
    expect(err).toBeInstanceOf(GitHubApiError);
    expect(err.name).toBe('GitHubNotFoundError');
    expect(err.status).toBe(404);
    expect(err.username).toBe('octocat');
    expect(err.message).toContain('octocat');
  });

  it('GitHubRateLimitError has correct properties', () => {
    const resetAt = new Date('2026-02-27T12:00:00Z');
    const err = new GitHubRateLimitError(0, resetAt);
    expect(err).toBeInstanceOf(GitHubApiError);
    expect(err.name).toBe('GitHubRateLimitError');
    expect(err.status).toBe(429);
    expect(err.remaining).toBe(0);
    expect(err.resetAt).toEqual(resetAt);
  });

  it('GitHubRateLimitError works without resetAt', () => {
    const err = new GitHubRateLimitError(0);
    expect(err.resetAt).toBeUndefined();
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/fetchers/github.test.ts`
Expected: FAIL — `GitHubNotFoundError` and `GitHubRateLimitError` not exported.

**Step 3: Add typed error classes to github.ts**

Add after the existing `GitHubApiError` class in `src/fetchers/github.ts`:

```typescript
export class GitHubNotFoundError extends GitHubApiError {
  constructor(public username: string) {
    super(`User not found: ${username}`, 404);
    this.name = 'GitHubNotFoundError';
  }
}

export class GitHubRateLimitError extends GitHubApiError {
  constructor(
    public remaining: number,
    public resetAt?: Date,
  ) {
    super('GitHub API rate limit exceeded', 429);
    this.name = 'GitHubRateLimitError';
  }
}
```

**Step 4: Update fetchGitHubData to use the new error classes and extract rate limit headers**

In `src/fetchers/github.ts`, update the `fetchGitHubData` function:

Replace the HTTP error check block:

```typescript
if (!response.ok) {
  throw new GitHubApiError(
    `GitHub API error: ${response.status} ${response.statusText}`,
    response.status,
  );
}
```

With:

```typescript
if (!response.ok) {
  if (response.status === 403 || response.status === 429) {
    const remaining = Number(response.headers.get('x-ratelimit-remaining') ?? 0);
    const resetEpoch = response.headers.get('x-ratelimit-reset');
    const resetAt = resetEpoch ? new Date(Number(resetEpoch) * 1000) : undefined;
    throw new GitHubRateLimitError(remaining, resetAt);
  }
  throw new GitHubApiError(
    `GitHub API error: ${response.status} ${response.statusText}`,
    response.status,
  );
}
```

Replace the user-not-found check:

```typescript
if (!user) {
  throw new GitHubApiError(`User not found: ${username}`, 404);
}
```

With:

```typescript
if (!user) {
  throw new GitHubNotFoundError(username);
}
```

**Step 5: Run tests to verify they pass**

Run: `pnpm vitest run src/fetchers/github.test.ts`
Expected: All tests PASS (existing + new).

**Step 6: Commit**

```bash
git add src/fetchers/github.ts src/fetchers/github.test.ts
git commit -m "feat: add typed GitHub error classes with rate limit header extraction"
```

---

## Task 8: Update Card Factory to Use Typed Errors

**Files:**

- Modify: `src/routes/card-factory.ts`

**Step 1: Update imports**

In `src/routes/card-factory.ts`, update the import:

From:

```typescript
import { fetchGitHubData, GitHubApiError } from '../fetchers/github';
```

To:

```typescript
import {
  fetchGitHubData,
  GitHubApiError,
  GitHubNotFoundError,
  GitHubRateLimitError,
} from '../fetchers/github';
```

**Step 2: Simplify error handling with typed errors**

Replace the catch block in `card-factory.ts` (the outer try-catch, lines 60-85):

```typescript
    } catch (err) {
      if (err instanceof GitHubNotFoundError) {
        const svg = errorSvg(`User not found: ${username}`);
        c.status(404);
        return svgResponse(c, svg, 300);
      }
      if (err instanceof GitHubRateLimitError) {
        const svg = errorSvg('GitHub API rate limit exceeded. Please try again later.');
        c.status(429);
        return svgResponse(c, svg, 60);
      }
      if (err instanceof Error && err.message === 'All PAT tokens are rate-limited') {
        const svg = errorSvg('All API tokens are rate-limited. Please try again later.');
        c.status(429);
        return svgResponse(c, svg, 60);
      }

      console.error(`Error generating ${routeConfig.cachePrefix} card for ${username}:`, err);
      const svg = errorSvg('An error occurred while generating the card.');
      c.status(502);
      return svgResponse(c, svg, 60);
    }
```

Also update the inner catch to use `GitHubRateLimitError`:

Replace:

```typescript
        if (err instanceof GitHubApiError && (err.status === 403 || err.status === 429)) {
```

With:

```typescript
        if (err instanceof GitHubRateLimitError) {
```

**Step 3: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS — no type errors.

**Step 4: Commit**

```bash
git add src/routes/card-factory.ts
git commit -m "refactor: use typed GitHub error classes in card factory"
```

---

## Task 9: Wire Everything Into the App Entry Point

**Files:**

- Modify: `src/index.ts`

**Step 1: Add metrics middleware and route**

Replace `src/index.ts` with:

```typescript
import { Hono } from 'hono';
import { loadConfig } from './config';
import { createCache } from './cache';
import { metricsMiddleware } from './metrics/middleware';
import { healthRoute } from './routes/health';
import { createMetricsRoute } from './routes/metrics';
import { createStatsRoute } from './routes/stats';
import { createStreakRoute } from './routes/streak';
import { createTopLangsRoute } from './routes/top-langs';

const config = loadConfig();
const cache = createCache(config.redisUrl);

const app = new Hono();

// Metrics middleware — track all requests
app.use('*', metricsMiddleware);

// Health check (no auth)
app.route('', healthRoute);

// Metrics (bearer auth protected)
app.route('', createMetricsRoute(config.metricsToken));

// Card routes — order matters: more specific paths first
app.route('', createStreakRoute(config, cache));
app.route('', createTopLangsRoute(config, cache));
app.route('', createStatsRoute(config, cache));

// 404 fallback
app.notFound((c) => {
  return c.json({ error: 'Not found' }, 404);
});

// Global error handler
app.onError((err, c) => {
  console.error('Unhandled error:', err);
  return c.json({ error: 'Internal server error' }, 500);
});

export default app;
```

**Step 2: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS.

**Step 3: Run all tests**

Run: `pnpm vitest run`
Expected: All tests PASS.

**Step 4: Commit**

```bash
git add src/index.ts
git commit -m "feat: wire metrics middleware and protected /metrics route into app"
```

---

## Task 10: Update Docker Config and .env.example

**Files:**

- Modify: `docker-compose.yml`
- Modify: `.env.example`

**Step 1: Add METRICS_TOKEN to docker-compose.yml**

In the `app` service `environment` section of `docker-compose.yml`, add:

```yaml
- METRICS_TOKEN=${METRICS_TOKEN:-}
```

**Step 2: Verify .env.example is complete**

`.env.example` should already have `METRICS_TOKEN` from Task 2. Verify it's present.

**Step 3: Commit**

```bash
git add docker-compose.yml .env.example
git commit -m "chore: add METRICS_TOKEN to docker compose environment"
```

---

## Task 11: Final Verification

**Step 1: Run full test suite**

Run: `pnpm vitest run`
Expected: All tests PASS.

**Step 2: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS.

**Step 3: Run linter**

Run: `pnpm lint`
Expected: PASS (no new warnings).

**Step 4: Run formatter check**

Run: `pnpm format:check`
If formatting issues: run `pnpm format` then commit the formatting changes.

**Step 5: Test build**

Run: `pnpm build`
Expected: Build succeeds.

**Step 6: Verify dev server starts**

Run: `pnpm dev` (manual check — Ctrl+C after confirming it starts without errors)

---

## Summary of Changes

| Area    | What changes                                       | New deps                   | Removed deps           |
| ------- | -------------------------------------------------- | -------------------------- | ---------------------- |
| Config  | `@t3-oss/env-core` + `METRICS_TOKEN` env var       | `@t3-oss/env-core`         | —                      |
| Metrics | New `src/metrics/` module + `GET /metrics` route   | — (hono built-ins)         | —                      |
| Caching | `cacheable` replaces custom cache layer            | `cacheable`, `@keyv/redis` | `lru-cache`, `ioredis` |
| Fetcher | Typed error classes + rate limit header extraction | —                          | —                      |

---

## Future Upgrade: OpenTelemetry (Traces + Metrics)

> **Not part of this iteration.** This section documents how to upgrade from in-memory metrics to OpenTelemetry when the project needs persistent observability.

### Why upgrade

The in-memory metrics store (Tasks 3-5) works for a single instance but resets on restart and has no dashboards. When the service is stable and seeing real traffic, upgrade to OpenTelemetry for:

- **Distributed traces:** Spans per route, per GitHub API call, per cache lookup
- **Prometheus-compatible metrics:** Export to Prometheus/Grafana for dashboards and alerting
- **Vendor-neutral:** Switch between Jaeger, Grafana Tempo, Datadog, etc. without code changes

### Deps to add

```bash
pnpm add @opentelemetry/sdk-node @opentelemetry/auto-instrumentations-node @opentelemetry/exporter-prometheus @opentelemetry/exporter-trace-otlp-http
```

### Architecture

```
src/
├── telemetry/
│   └── setup.ts           # OTel SDK initialization (must run before app imports)
├── metrics/
│   ├── store.ts           # Keep as fallback, or replace with OTel meter
│   └── middleware.ts       # Replace with OTel span creation
```

### Key integration points

1. **`src/telemetry/setup.ts`** — Initialize OTel SDK with:
   - `PrometheusExporter` for metrics (replaces in-memory store)
   - `OTLPTraceExporter` for traces (sends to Jaeger/Tempo)
   - `getNodeAutoInstrumentations()` for automatic HTTP/fetch instrumentation
2. **`src/metrics/middleware.ts`** — Replace `recordRequest()` with OTel span creation:
   - Start span on request, set attributes (route, method)
   - End span on response, record status + latency
3. **`src/cache/index.ts`** — Add cache hit/miss as OTel counter metric
4. **`src/fetchers/github.ts`** — Add child span for GitHub API calls (duration, status, token index)
5. **Docker compose** — Add Prometheus + Grafana + Jaeger containers

### Migration path

The in-memory metrics middleware and `/metrics` route can coexist with OTel during migration. Once OTel is stable, remove the custom store and point `/metrics` at the Prometheus exporter.

---

## Future Upgrade: Sentry for Error Tracking

> **Not part of this iteration.** This section documents how to add Sentry for error monitoring.

### Why add Sentry

Currently errors are logged to `console.error` and tracked as status code counters. Sentry provides:

- **Error grouping and deduplication** — see unique errors, not log noise
- **Stack traces with source maps** — pinpoint errors in production builds
- **Release tracking** — correlate errors with deployments
- **Alerting** — get notified on new or spiking errors

### Deps to add

```bash
pnpm add @sentry/node
```

### Key integration points

1. **`src/index.ts`** — Initialize Sentry before app creation:
   ```typescript
   import * as Sentry from '@sentry/node';
   Sentry.init({ dsn: config.sentryDsn, tracesSampleRate: 0.1 });
   ```
2. **`src/config.ts`** — Add `SENTRY_DSN` env var (optional)
3. **Global error handler** — Capture unhandled errors:
   ```typescript
   app.onError((err, c) => {
     Sentry.captureException(err);
     return c.json({ error: 'Internal server error' }, 500);
   });
   ```
4. **`src/fetchers/github.ts`** — Capture GitHub API errors with context:
   ```typescript
   Sentry.captureException(err, { tags: { username, tokenIndex } });
   ```
5. **`src/routes/card-factory.ts`** — Capture card generation errors with route context

### Integration with OpenTelemetry

If both OTel and Sentry are active, Sentry can ingest OTel traces via `@sentry/opentelemetry`. This avoids double-instrumenting:

```bash
pnpm add @sentry/opentelemetry
```

Configure Sentry as an OTel span processor rather than using its own instrumentation.
