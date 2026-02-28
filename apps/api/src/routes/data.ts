import { Hono } from 'hono';
import { getConnInfo } from '@hono/node-server/conninfo';
import { rateLimiter } from 'hono-rate-limiter';
import type { AppConfig } from '../config';
import type { Cache } from '../cache/index';
import { createPatPool } from '../utils/pat-pool';
import { fetchGitHubData, GitHubNotFoundError, GitHubRateLimitError } from '../fetchers/github';
import { THEME_NAMES } from '@gitcard/svg-renderer';

export function createDataRoute(config: AppConfig, cache: Cache): Hono {
  const app = new Hono();
  const patPool = createPatPool(config.pats);

  // Rate limit: 30 requests per minute per IP
  // Priority: cf-connecting-ip (Cloudflare, unfakeable) → socket IP (unfakeable)
  app.use(
    '/api/data/*',
    rateLimiter({
      windowMs: 60_000,
      limit: 30,
      keyGenerator: (c) => {
        const cfIp = c.req.header('cf-connecting-ip');
        if (cfIp) return cfIp;
        try {
          const info = getConnInfo(c);
          return info.remote.address ?? 'unknown';
        } catch {
          return 'unknown';
        }
      },
    }),
  );

  app.get('/api/data/:username', async (c) => {
    const username = c.req.param('username');
    const dataCacheKey = `data:${username}`;

    try {
      // Check data cache first
      const cachedData = await cache.get(dataCacheKey);
      if (cachedData) {
        return c.json(JSON.parse(cachedData));
      }

      // Fetch from GitHub
      const token = patPool.getNextToken();

      let data;
      try {
        data = await fetchGitHubData(username, token);
      } catch (err) {
        // If rate-limited, mark token exhausted and retry with next token
        if (err instanceof GitHubRateLimitError) {
          patPool.markExhausted(token);
          const retryToken = patPool.getNextToken();
          data = await fetchGitHubData(username, retryToken);
        } else {
          throw err;
        }
      }

      // Cache the result
      await cache.set(dataCacheKey, JSON.stringify(data), config.cacheTtl);

      return c.json(data);
    } catch (err) {
      if (err instanceof GitHubNotFoundError) {
        return c.json({ error: `User not found: ${username}` }, 404);
      }
      if (err instanceof GitHubRateLimitError) {
        return c.json({ error: 'GitHub API rate limit exceeded' }, 429);
      }
      console.error(`Error fetching data for ${username}:`, err);
      return c.json({ error: 'Internal server error' }, 500);
    }
  });

  app.get('/api/themes', (c) => {
    return c.json(THEME_NAMES);
  });

  return app;
}
