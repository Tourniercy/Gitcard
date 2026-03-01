import { beforeEach, describe, expect, it, vi } from 'vitest';

// Stub fetch globally before any imports that might use it
vi.stubGlobal(
  'fetch',
  vi.fn().mockResolvedValue({
    ok: true,
    json: () =>
      Promise.resolve({
        data: {
          user: {
            name: 'Test User',
            login: 'testuser',
            contributionsCollection: {
              totalCommitContributions: 100,
              restrictedContributionsCount: 10,
              contributionCalendar: {
                totalContributions: 500,
                weeks: [
                  {
                    contributionDays: [
                      { contributionCount: 5, date: '2026-02-25' },
                      { contributionCount: 3, date: '2026-02-26' },
                      { contributionCount: 1, date: '2026-02-27' },
                    ],
                  },
                ],
              },
            },
            repositories: {
              totalCount: 20,
              nodes: [
                {
                  stargazerCount: 50,
                  forkCount: 10,
                  languages: {
                    edges: [
                      { size: 1000, node: { name: 'TypeScript', color: '#3178c6' } },
                      { size: 500, node: { name: 'Python', color: '#3572A5' } },
                    ],
                  },
                },
              ],
            },
            pullRequests: { totalCount: 30 },
            issues: { totalCount: 15 },
            followers: { totalCount: 200 },
            repositoriesContributedTo: { totalCount: 5 },
          },
        },
      }),
    headers: new Headers(),
  }),
);

// Mock Sentry to avoid side effects
vi.mock('@sentry/node', () => ({
  startSpan: vi.fn((_opts: unknown, fn: () => unknown) => fn()),
  metrics: { count: vi.fn(), distribution: vi.fn() },
  captureException: vi.fn(),
}));

import { Hono } from 'hono';
import type { Cache } from '../cache/index';
import type { AppConfig } from '../config';
import type { FetchResult } from '../fetchers/github';
import type { CardOptions } from '@gitcard/svg-renderer';
import { createPatPool } from '../utils/pat-pool';

function createMockCache(): Cache & {
  store: Map<string, string>;
} {
  const store = new Map<string, string>();
  return {
    store,
    async get(key: string): Promise<string | null> {
      return store.get(key) ?? null;
    },
    async set(key: string, value: string, _ttlSeconds: number): Promise<void> {
      store.set(key, value);
    },
    async flush(): Promise<void> {
      store.clear();
    },
  };
}

function createTestConfig(): AppConfig {
  return {
    pats: ['ghp_test_token_1'],
    port: 3000,
    cacheTtl: 14400,
    logLevel: 'info',
    redisUrl: undefined,
    sentryDsn: undefined,
    sentryTracesSampleRate: 0,
    sentryEnvironment: 'test',
  };
}

describe('card-factory two-layer cache', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.resetModules();
  });

  it('caches GitHub data and reuses it on SVG cache miss with different options', async () => {
    const { createCardRoute } = await import('./card-factory');

    const cache = createMockCache();
    const config = createTestConfig();
    const renderFn = vi.fn((_data: FetchResult, _options: CardOptions) => '<svg>mock</svg>');

    const route = createCardRoute(config, cache, createPatPool(config.pats), {
      path: '/stats/:username',
      cachePrefix: 'stats',
      render: renderFn,
    });

    const app = new Hono();
    app.route('', route);

    // First request — should fetch from GitHub and render
    const res1 = await app.request('/stats/testuser');
    expect(res1.status).toBe(200);
    expect(await res1.text()).toContain('<svg');
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(renderFn).toHaveBeenCalledTimes(1);

    // Second request with different theme — should NOT fetch from GitHub again
    // (data cache hit), but should render again (SVG cache miss due to different params)
    const res2 = await app.request('/stats/testuser?theme=dark');
    expect(res2.status).toBe(200);
    expect(await res2.text()).toContain('<svg');
    expect(fetch).toHaveBeenCalledTimes(1); // Still 1 — data was cached
    expect(renderFn).toHaveBeenCalledTimes(2); // Render called again with new options

    // Verify data cache key exists
    expect(cache.store.has('data:testuser')).toBe(true);
  });

  it('returns cached SVG on full cache hit without rendering', async () => {
    const { createCardRoute } = await import('./card-factory');

    const cache = createMockCache();
    const config = createTestConfig();
    const renderFn = vi.fn((_data: FetchResult, _options: CardOptions) => '<svg>mock</svg>');

    const route = createCardRoute(config, cache, createPatPool(config.pats), {
      path: '/stats/:username',
      cachePrefix: 'stats',
      render: renderFn,
    });

    const app = new Hono();
    app.route('', route);

    // First request — populates both caches
    const res1 = await app.request('/stats/testuser');
    expect(res1.status).toBe(200);
    expect(renderFn).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledTimes(1);

    // Second identical request — should hit SVG cache, no render or fetch
    const res2 = await app.request('/stats/testuser');
    expect(res2.status).toBe(200);
    expect(await res2.text()).toContain('<svg');
    expect(fetch).toHaveBeenCalledTimes(1); // Still 1
    expect(renderFn).toHaveBeenCalledTimes(1); // Still 1 — SVG was cached
  });
});
