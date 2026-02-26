import type { Hono } from 'hono';
import type { AppConfig } from '../config';
import type { Cache } from '../cache/index';
import { renderStatsCard } from '../cards/stats-card';
import { createCardRoute } from './card-factory';

export function createStatsRoute(config: AppConfig, cache: Cache): Hono {
  return createCardRoute(config, cache, {
    path: '/:username',
    cachePrefix: 'stats',
    render: (data, options) => renderStatsCard(data.stats, options),
  });
}
