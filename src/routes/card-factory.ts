import { Hono } from 'hono';
import { createHash } from 'node:crypto';
import type { AppConfig } from '../config';
import type { Cache } from '../cache/index';
import type { CardOptions } from '../types';
import { createPatPool } from '../utils/pat-pool';
import { parseCardOptions } from '../utils/query-params';
import {
  fetchGitHubData,
  GitHubNotFoundError,
  GitHubRateLimitError,
} from '../fetchers/github';
import type { FetchResult } from '../fetchers/github';
import { svgResponse, errorSvg } from './card-response';

interface CardRouteConfig {
  path: string;
  cachePrefix: string;
  render: (data: FetchResult, options: CardOptions) => string;
}

export function createCardRoute(
  config: AppConfig,
  cache: Cache,
  routeConfig: CardRouteConfig,
): Hono {
  const app = new Hono();
  const patPool = createPatPool(config.pats);

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

    // Fetch and render
    try {
      const token = patPool.getNextToken();
      let data: FetchResult;

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

      const svg = routeConfig.render(data, options);
      await cache.set(cacheKey, svg, options.cacheSeconds);
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
  });

  return app;
}
