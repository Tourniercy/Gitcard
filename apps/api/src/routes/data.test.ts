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

describe('GET /api/data/:username', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.resetModules();
  });

  it('returns JSON with stats, streak, and languages for a valid user', async () => {
    const { createDataRoute } = await import('./data');

    const cache = createMockCache();
    const config = createTestConfig();
    const route = createDataRoute(config, cache);

    const app = new Hono();
    app.route('', route);

    const res = await app.request('/api/data/testuser');

    expect(res.status).toBe(200);
    const json = await res.json();
    expect(json).toHaveProperty('stats');
    expect(json).toHaveProperty('streak');
    expect(json).toHaveProperty('languages');
    expect(json.stats.username).toBe('testuser');
    expect(json.stats.totalStars).toBe(50);
    expect(json.streak.totalContributions).toBe(500);
    expect(json.languages.languages).toHaveLength(2);
  });

  it('returns 404 for non-existent user', async () => {
    // Override fetch to return user-not-found response
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () =>
          Promise.resolve({
            data: { user: null },
            errors: [{ message: "Could not resolve to a User with the login of 'ghost'" }],
          }),
        headers: new Headers(),
      }),
    );

    const { createDataRoute } = await import('./data');

    const cache = createMockCache();
    const config = createTestConfig();
    const route = createDataRoute(config, cache);

    const app = new Hono();
    app.route('', route);

    const res = await app.request('/api/data/ghost');

    expect(res.status).toBe(404);
    const json = await res.json();
    expect(json.error).toBe('User not found: ghost');
  });
});

describe('GET /api/themes', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.resetModules();
  });

  it('returns array containing default, dark, dracula', async () => {
    const { createDataRoute } = await import('./data');

    const cache = createMockCache();
    const config = createTestConfig();
    const route = createDataRoute(config, cache);

    const app = new Hono();
    app.route('', route);

    const res = await app.request('/api/themes');

    expect(res.status).toBe(200);
    const json = await res.json();
    expect(Array.isArray(json)).toBe(true);
    expect(json).toContain('default');
    expect(json).toContain('dark');
    expect(json).toContain('dracula');
  });
});
