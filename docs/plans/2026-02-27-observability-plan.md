# Sentry + OpenTelemetry Observability — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add full observability via Sentry SaaS — error tracking, performance monitoring, distributed traces, and custom metrics (cards generated, cache hits/misses, GitHub API duration).

**Architecture:** Sentry v8+ with built-in OpenTelemetry (Mode A — Sentry manages OTel internally). A side-effect `telemetry.ts` module is imported first in `index.ts` to call `Sentry.init()` before any other code. Custom spans wrap card generation, caching, and GitHub API calls. Custom metrics count cards generated and cache operations. Existing in-memory `/metrics` endpoint stays for local dev.

**Tech Stack:** `@sentry/node` (only package needed — OTel is built-in in v8+)

**Design doc:** `docs/plans/2026-02-27-observability-design.md`

---

### Task 1: Install @sentry/node

**Files:**

- Modify: `package.json`

**Step 1: Install the dependency**

Run: `pnpm add @sentry/node`

**Step 2: Verify installation**

Run: `pnpm ls @sentry/node`
Expected: Shows `@sentry/node` version 8.x+ in the dependency tree.

**Step 3: Commit**

```bash
git add package.json pnpm-lock.yaml
git commit -m "chore: add @sentry/node dependency"
```

---

### Task 2: Add Sentry env vars to config.ts

**Files:**

- Modify: `src/config.ts`
- Modify: `src/config.test.ts`

**Step 1: Write the failing test**

Add a new test to `src/config.test.ts`:

```typescript
it('loads Sentry config when set', async () => {
  vi.stubEnv('PAT_1', 'ghp_test');
  vi.stubEnv('METRICS_TOKEN', '');
  vi.stubEnv('SENTRY_DSN', 'https://key@sentry.io/123');
  vi.stubEnv('SENTRY_TRACES_SAMPLE_RATE', '0.5');
  vi.stubEnv('SENTRY_ENVIRONMENT', 'staging');

  const { loadConfig } = await import('./config');
  const config = loadConfig();

  expect(config.sentryDsn).toBe('https://key@sentry.io/123');
  expect(config.sentryTracesSampleRate).toBe(0.5);
  expect(config.sentryEnvironment).toBe('staging');
});

it('uses Sentry defaults when not set', async () => {
  vi.stubEnv('PAT_1', 'ghp_test');
  vi.stubEnv('METRICS_TOKEN', '');

  const { loadConfig } = await import('./config');
  const config = loadConfig();

  expect(config.sentryDsn).toBeUndefined();
  expect(config.sentryTracesSampleRate).toBe(0.1);
  expect(config.sentryEnvironment).toBe('production');
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm test -- src/config.test.ts`
Expected: FAIL — `sentryDsn`, `sentryTracesSampleRate`, `sentryEnvironment` don't exist on config.

**Step 3: Implement config changes**

In `src/config.ts`, add to the `createEnv` server schema:

```typescript
SENTRY_DSN: z.string().url().optional(),
SENTRY_TRACES_SAMPLE_RATE: z.coerce.number().min(0).max(1).default(0.1),
SENTRY_ENVIRONMENT: z.string().min(1).default('production'),
```

Add to the `AppConfig` interface:

```typescript
sentryDsn?: string;
sentryTracesSampleRate: number;
sentryEnvironment: string;
```

Add to the `loadConfig()` return object:

```typescript
sentryDsn: env.SENTRY_DSN,
sentryTracesSampleRate: env.SENTRY_TRACES_SAMPLE_RATE,
sentryEnvironment: env.SENTRY_ENVIRONMENT,
```

**Step 4: Run test to verify it passes**

Run: `pnpm test -- src/config.test.ts`
Expected: All tests PASS.

**Step 5: Commit**

```bash
git add src/config.ts src/config.test.ts
git commit -m "feat: add Sentry env vars to config"
```

---

### Task 3: Create src/telemetry.ts

**Files:**

- Create: `src/telemetry.ts`
- Create: `src/telemetry.test.ts`

**Context:** This module is a side-effect import. It reads `process.env` directly (not through `config.ts`) because it must execute before any other module. If `SENTRY_DSN` is absent, it's a complete no-op.

**Step 1: Write the failing test**

Create `src/telemetry.test.ts`:

```typescript
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock @sentry/node before importing telemetry
vi.mock('@sentry/node', () => ({
  init: vi.fn(),
  isInitialized: vi.fn(() => false),
}));

describe('telemetry', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.unstubAllEnvs();
  });

  it('calls Sentry.init when SENTRY_DSN is set', async () => {
    vi.stubEnv('SENTRY_DSN', 'https://key@sentry.io/123');
    vi.stubEnv('SENTRY_TRACES_SAMPLE_RATE', '0.5');
    vi.stubEnv('SENTRY_ENVIRONMENT', 'staging');

    const Sentry = await import('@sentry/node');
    await import('./telemetry');

    expect(Sentry.init).toHaveBeenCalledWith(
      expect.objectContaining({
        dsn: 'https://key@sentry.io/123',
        tracesSampleRate: 0.5,
        environment: 'staging',
      }),
    );
  });

  it('does not call Sentry.init when SENTRY_DSN is absent', async () => {
    vi.stubEnv('SENTRY_DSN', '');

    const Sentry = await import('@sentry/node');
    await import('./telemetry');

    expect(Sentry.init).not.toHaveBeenCalled();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm test -- src/telemetry.test.ts`
Expected: FAIL — `src/telemetry.ts` does not exist.

**Step 3: Implement telemetry.ts**

Create `src/telemetry.ts`:

```typescript
import * as Sentry from '@sentry/node';

const dsn = process.env.SENTRY_DSN;

if (dsn) {
  Sentry.init({
    dsn,
    tracesSampleRate: Number(process.env.SENTRY_TRACES_SAMPLE_RATE ?? '0.1'),
    environment: process.env.SENTRY_ENVIRONMENT ?? 'production',
  });
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm test -- src/telemetry.test.ts`
Expected: All tests PASS.

**Step 5: Commit**

```bash
git add src/telemetry.ts src/telemetry.test.ts
git commit -m "feat: add telemetry module with Sentry init"
```

---

### Task 4: Wire telemetry into index.ts and global error handler

**Files:**

- Modify: `src/index.ts`

**Context:** `import './telemetry'` must be the very first import in `index.ts`. The global error handler should call `Sentry.captureException()` to send unhandled errors to Sentry.

**Step 1: Update index.ts**

Add `import './telemetry';` as the very first line (before all other imports).

Add `import * as Sentry from '@sentry/node';` after telemetry.

Update the global error handler:

```typescript
app.onError((err, c) => {
  Sentry.captureException(err);
  console.error('Unhandled error:', err);
  return c.json({ error: 'Internal server error' }, 500);
});
```

The full import order at the top of `index.ts` should be:

```typescript
import './telemetry';
import * as Sentry from '@sentry/node';
import { Hono } from 'hono';
import { loadConfig } from './config';
// ... rest of imports
```

**Step 2: Verify typecheck passes**

Run: `pnpm typecheck`
Expected: No errors.

**Step 3: Run all tests**

Run: `pnpm test`
Expected: All tests PASS.

**Step 4: Commit**

```bash
git add src/index.ts
git commit -m "feat: wire telemetry into app entry and error handler"
```

---

### Task 5: Add Sentry instrumentation to card-factory.ts

**Files:**

- Modify: `src/routes/card-factory.ts`

**Context:** Wrap the entire card generation flow (fetch + render) in a `Sentry.startSpan()`. Increment `cards_generated_total` counter with a `card_type` tag. This gives us per-card-type generation counts and latency in Sentry.

**Step 1: Add Sentry span and metrics**

Add `import * as Sentry from '@sentry/node';` at the top.

Wrap the fetch-and-render logic inside the route handler with `Sentry.startSpan()`:

```typescript
app.get(routeConfig.path, async (c) => {
  const username = c.req.param('username') as string;
  const options = parseCardOptions(c.req.query());

  const paramsHash = createHash('md5').update(JSON.stringify(options)).digest('hex').slice(0, 8);
  const cacheKey = `card:${routeConfig.cachePrefix}:${username}:${paramsHash}`;

  // Check cache
  const cached = await cache.get(cacheKey);
  if (cached) {
    return svgResponse(c, cached, options.cacheSeconds);
  }

  // Fetch and render (wrapped in Sentry span)
  return Sentry.startSpan(
    {
      name: `card.generate ${routeConfig.cachePrefix}`,
      op: 'card.generate',
      attributes: { username, card_type: routeConfig.cachePrefix },
    },
    async () => {
      try {
        const token = patPool.getNextToken();
        let data: FetchResult;

        try {
          data = await fetchGitHubData(username, token);
        } catch (err) {
          if (err instanceof GitHubRateLimitError) {
            patPool.markExhausted(token);
            const retryToken = patPool.getNextToken();
            data = await fetchGitHubData(username, retryToken);
          } else {
            throw err;
          }
        }

        const svg = routeConfig.render(data, options);
        await cache.set(cacheKey, svg, options.cacheSeconds);

        Sentry.metrics.increment('cards_generated_total', 1, {
          tags: { card_type: routeConfig.cachePrefix },
        });

        return svgResponse(c, svg, options.cacheSeconds);
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
    },
  );
});
```

> **Note:** Use `Sentry.metrics.increment()` for the counter. Check the installed @sentry/node version — if v8, use `Sentry.metrics.increment()`. If v10+, use `Sentry.metrics.count()`. Consult Context7 for the exact API on the installed version.

**Step 2: Verify typecheck passes**

Run: `pnpm typecheck`
Expected: No errors.

**Step 3: Run tests**

Run: `pnpm test`
Expected: All tests PASS.

**Step 4: Commit**

```bash
git add src/routes/card-factory.ts
git commit -m "feat: add Sentry span and card generation counter to card-factory"
```

---

### Task 6: Add Sentry instrumentation to cache/index.ts

**Files:**

- Modify: `src/cache/index.ts`

**Context:** Wrap cache get/set in `Sentry.startSpan()`. Increment `cache_hits_total` / `cache_misses_total` counters via Sentry metrics (alongside the existing in-memory `cacheCounters`).

**Step 1: Add Sentry spans and metrics**

Add `import * as Sentry from '@sentry/node';` at the top.

Update the `get` method:

```typescript
async get(key: string): Promise<string | null> {
  return Sentry.startSpan(
    { name: 'cache.get', op: 'cache.get', attributes: { 'cache.key': key } },
    async () => {
      const value = await cache.get<string>(key);
      if (value !== undefined) {
        cacheCounters.hits++;
        Sentry.metrics.increment('cache_hits_total');
        return value;
      }
      cacheCounters.misses++;
      Sentry.metrics.increment('cache_misses_total');
      return null;
    },
  );
},
```

Update the `set` method:

```typescript
async set(key: string, value: string, ttlSeconds: number): Promise<void> {
  return Sentry.startSpan(
    { name: 'cache.set', op: 'cache.set', attributes: { 'cache.key': key } },
    async () => {
      await cache.set(key, value, ttlSeconds * 1000);
    },
  );
},
```

**Step 2: Verify typecheck passes**

Run: `pnpm typecheck`
Expected: No errors.

**Step 3: Run tests**

Run: `pnpm test`
Expected: All tests PASS.

**Step 4: Commit**

```bash
git add src/cache/index.ts
git commit -m "feat: add Sentry spans and cache metrics to cache module"
```

---

### Task 7: Add Sentry instrumentation to fetchers/github.ts

**Files:**

- Modify: `src/fetchers/github.ts`

**Context:** Wrap the `fetchGitHubData` function body in `Sentry.startSpan()`. On errors, call `Sentry.captureException()` with username and card context as tags. Record `github_api_duration` as a Sentry distribution metric. Note: Sentry auto-instruments native `fetch()`, so the HTTP call itself is already traced. We add a parent span for the overall GitHub fetch operation and capture errors with context.

**Step 1: Add Sentry span and error capture**

Add `import * as Sentry from '@sentry/node';` at the top.

Wrap the body of `fetchGitHubData`:

```typescript
export async function fetchGitHubData(username: string, token: string): Promise<FetchResult> {
  return Sentry.startSpan(
    {
      name: `github.fetch ${username}`,
      op: 'http.client',
      attributes: { username },
    },
    async () => {
      const start = performance.now();

      try {
        const response = await fetch(GITHUB_GRAPHQL_URL, {
          method: 'POST',
          headers: {
            Authorization: `bearer ${token}`,
            'Content-Type': 'application/json',
            'User-Agent': 'github-stats-cards',
          },
          body: JSON.stringify({ query: USER_QUERY, variables: { login: username } }),
        });

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

        const json = (await response.json()) as GitHubGraphQLResponse;

        if (json.errors?.length) {
          throw new GitHubApiError(
            `GraphQL errors: ${json.errors.map((e) => e.message).join(', ')}`,
          );
        }

        const user = json.data?.user;
        if (!user) {
          throw new GitHubNotFoundError(username);
        }

        const durationMs = performance.now() - start;
        Sentry.metrics.distribution('github_api_duration', durationMs, {
          unit: 'millisecond',
        });

        return {
          stats: extractStats(user, username),
          streak: extractStreak(user, username),
          languages: extractLanguages(user, username),
        };
      } catch (err) {
        Sentry.captureException(err, {
          tags: { username },
          extra: { operation: 'fetchGitHubData' },
        });
        throw err;
      }
    },
  );
}
```

**Step 2: Verify typecheck passes**

Run: `pnpm typecheck`
Expected: No errors.

**Step 3: Run tests**

Run: `pnpm test`
Expected: All tests PASS.

**Step 4: Commit**

```bash
git add src/fetchers/github.ts
git commit -m "feat: add Sentry span, error capture, and duration metric to GitHub fetcher"
```

---

### Task 8: Update Docker config and .env.example

**Files:**

- Modify: `docker-compose.yml`
- Modify: `.env.example`

**Step 1: Update docker-compose.yml**

Add Sentry env vars to the `app` service `environment` section:

```yaml
- SENTRY_DSN=${SENTRY_DSN:-}
- SENTRY_TRACES_SAMPLE_RATE=${SENTRY_TRACES_SAMPLE_RATE:-0.1}
- SENTRY_ENVIRONMENT=${SENTRY_ENVIRONMENT:-production}
```

**Step 2: Update .env.example**

Add at the end:

```
# Sentry (optional — if not set, Sentry is disabled)
# SENTRY_DSN=https://key@o123.ingest.sentry.io/456
# SENTRY_TRACES_SAMPLE_RATE=0.1
# SENTRY_ENVIRONMENT=production
```

**Step 3: Commit**

```bash
git add docker-compose.yml .env.example
git commit -m "chore: add Sentry env vars to Docker config and .env.example"
```

---

### Task 9: Final verification

**Step 1: Run format**

Run: `pnpm format`

**Step 2: Run lint**

Run: `pnpm lint`
Expected: No errors.

**Step 3: Run typecheck**

Run: `pnpm typecheck`
Expected: No errors.

**Step 4: Run all tests**

Run: `pnpm test`
Expected: All tests PASS.

**Step 5: Verify build**

Run: `pnpm build`
Expected: Build succeeds, output in `dist/`.

**Step 6: Commit any formatting fixes**

```bash
git add -A
git commit -m "chore: format and lint fixes"
```

(Only if there are changes to commit.)

**Step 7: Verify graceful degradation**

Without `SENTRY_DSN` set, the app should start and work identically to before. The telemetry module is a no-op.

Run: `pnpm dev` (briefly check that `/health` responds 200)

---

## Summary of Sentry Instrumentation

| File                         | What Sentry Captures                                                                                                                                                        |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/telemetry.ts`           | `Sentry.init()` — enables error tracking, performance, traces                                                                                                               |
| `src/index.ts`               | `Sentry.captureException()` — catches all unhandled errors                                                                                                                  |
| `src/routes/card-factory.ts` | `Sentry.startSpan()` — card generation duration. `Sentry.metrics.increment('cards_generated_total')` — per-card-type counter                                                |
| `src/cache/index.ts`         | `Sentry.startSpan()` — cache get/set latency. `Sentry.metrics.increment('cache_hits_total'/'cache_misses_total')` — cache counters                                          |
| `src/fetchers/github.ts`     | `Sentry.startSpan()` — GitHub API call span. `Sentry.metrics.distribution('github_api_duration')` — API latency. `Sentry.captureException()` — errors with username context |

## What You Get on Sentry Dashboard

- **Errors**: Every unhandled error + GitHub API errors with username context
- **Performance**: Request latency per route, card generation latency, GitHub API latency
- **Traces**: Full request lifecycle (request → cache check → GitHub fetch → card render → response)
- **Custom Metrics**: `cards_generated_total` (by card type), `cache_hits_total`, `cache_misses_total`, `github_api_duration`
