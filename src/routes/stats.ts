import type { Hono } from 'hono';
import type { AppConfig } from '../config.js';
import type { Cache } from '../cache/index.js';
import { renderStatsCard } from '../cards/stats-card.js';
import { createCardRoute } from './card-factory.js';

export function createStatsRoute(config: AppConfig, cache: Cache): Hono {
  return createCardRoute(config, cache, {
    path: '/:username',
    cachePrefix: 'stats',
    render: (data, options) => renderStatsCard(data.stats, options),
  });
}
