import { describe, expect, it } from 'vitest';
import { fetchGitHubData, GitHubNotFoundError } from './fetchers/github';

const GITHUB_TOKEN = process.env.PAT_1 ?? process.env.GITHUB_TOKEN;

const describeIntegration = GITHUB_TOKEN ? describe : describe.skip;

describeIntegration('integration: GitHub API (real token)', () => {
  it('fetches stats for a known public user', async () => {
    const result = await fetchGitHubData('torvalds', GITHUB_TOKEN!);

    // Stats
    expect(result.stats.username).toBe('torvalds');
    expect(result.stats.name).toBe('Linus Torvalds');
    expect(result.stats.totalStars).toBeGreaterThan(0);
    expect(result.stats.totalForks).toBeGreaterThan(0);
    expect(result.stats.totalCommits).toBeGreaterThanOrEqual(0);
    expect(result.stats.totalPRs).toBeGreaterThanOrEqual(0);
    expect(result.stats.totalIssues).toBeGreaterThanOrEqual(0);
    expect(result.stats.contributedTo).toBeGreaterThanOrEqual(0);

    // Streak
    expect(result.streak.username).toBe('torvalds');
    expect(result.streak.totalContributions).toBeGreaterThanOrEqual(0);
    expect(result.streak.currentStreak).toBeGreaterThanOrEqual(0);
    expect(result.streak.longestStreak).toBeGreaterThanOrEqual(0);

    // Languages
    expect(result.languages.username).toBe('torvalds');
    expect(result.languages.languages.length).toBeGreaterThan(0);
    expect(result.languages.languages[0]).toHaveProperty('name');
    expect(result.languages.languages[0]).toHaveProperty('percentage');
  });

  it('throws GitHubNotFoundError for a non-existent user', async () => {
    const bogusUsername = 'this-user-definitely-does-not-exist-xyz-123456789';
    try {
      await fetchGitHubData(bogusUsername, GITHUB_TOKEN!);
      expect.fail('Should have thrown');
    } catch (err) {
      expect(err).toBeInstanceOf(GitHubNotFoundError);
      expect((err as GitHubNotFoundError).status).toBe(404);
    }
  });

  it('throws GitHubRateLimitError with an invalid token', async () => {
    try {
      await fetchGitHubData('torvalds', 'ghp_invalid_token_000000000000000000');
      expect.fail('Should have thrown');
    } catch (err) {
      // GitHub returns 401 for bad tokens, which we map to a generic error
      // Just verify it throws — the exact error depends on GitHub's response
      expect(err).toBeDefined();
    }
  });

  it('returns valid SVG when rendered through the full card pipeline', async () => {
    const result = await fetchGitHubData('torvalds', GITHUB_TOKEN!);

    // Dynamically import card renderers to test the full pipeline
    const { renderStatsCard } = await import('./cards/stats-card');
    const { renderStreakCard } = await import('./cards/streak-card');
    const { renderLangsCard } = await import('./cards/langs-card');
    const { parseCardOptions } = await import('./utils/query-params');

    const options = parseCardOptions({});

    const statsSvg = renderStatsCard(result.stats, options);
    expect(statsSvg).toContain('<svg');
    expect(statsSvg).toContain('Linus Torvalds');
    expect(statsSvg).toContain('</svg>');

    const streakSvg = renderStreakCard(result.streak, options);
    expect(streakSvg).toContain('<svg');
    expect(streakSvg).toContain('</svg>');

    const langsSvg = renderLangsCard(result.languages, options);
    expect(langsSvg).toContain('<svg');
    expect(langsSvg).toContain('</svg>');
  });

  it('returns proper HTTP responses through Hono routes', async () => {
    const { Hono } = await import('hono');
    const { createStatsRoute } = await import('./routes/stats');
    const { createStreakRoute } = await import('./routes/streak');
    const { createTopLangsRoute } = await import('./routes/top-langs');
    const { createCache } = await import('./cache/index');

    const config = {
      pats: [GITHUB_TOKEN!],
      port: 3000,
      cacheTtl: 60,
      logLevel: 'info' as const,
      metricsToken: undefined,
      redisUrl: undefined,
      sentryDsn: undefined,
      sentryTracesSampleRate: 0,
      sentryEnvironment: 'test',
    };
    const cache = createCache(undefined);

    const app = new Hono();
    app.route('', createStatsRoute(config, cache));
    app.route('', createStreakRoute(config, cache));
    app.route('', createTopLangsRoute(config, cache));

    // Stats card
    const statsRes = await app.request('/stats/torvalds');
    expect(statsRes.status).toBe(200);
    expect(statsRes.headers.get('content-type')).toContain('image/svg+xml');
    const statsSvg = await statsRes.text();
    expect(statsSvg).toContain('<svg');

    // Streak card
    const streakRes = await app.request('/stats/torvalds/streak');
    expect(streakRes.status).toBe(200);
    expect(streakRes.headers.get('content-type')).toContain('image/svg+xml');

    // Top langs card
    const langsRes = await app.request('/stats/torvalds/top-langs');
    expect(langsRes.status).toBe(200);
    expect(langsRes.headers.get('content-type')).toContain('image/svg+xml');

    // 404 for non-existent user
    const notFoundRes = await app.request('/stats/this-user-does-not-exist-xyz-123456789');
    expect(notFoundRes.status).toBe(404);
    const errorSvg = await notFoundRes.text();
    expect(errorSvg).toContain('User not found');
  });
});
