import { Hono } from 'hono';
import { getConnInfo } from '@hono/node-server/conninfo';
import { rateLimiter } from 'hono-rate-limiter';
import type { AppConfig } from '../config';
import type { Cache } from '../cache';

import type { PatPool } from '../utils/pat-pool';
import {
  fetchWithRetry,
  GitHubNotFoundError,
  GitHubRateLimitError,
  type FetchResult,
} from '../fetchers/github';
import { THEME_NAMES } from '@gitcard/svg-renderer';

export function createDataRoute(config: AppConfig, cache: Cache, patPool: PatPool) {
  return new Hono()
    .use(
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
    )
    .get('/api/data/:username', async (c) => {
      const username = c.req.param('username');
      const dataCacheKey = `data:${username}`;

      try {
        const cachedData = await cache.get(dataCacheKey);
        if (cachedData) {
          return c.json(JSON.parse(cachedData) as FetchResult);
        }

        const data = await fetchWithRetry(username, patPool);

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
    })
    .get('/api/themes', (c) => {
      return c.json(THEME_NAMES);
    });
}
