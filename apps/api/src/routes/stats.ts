import type { Hono } from 'hono';
import type { AppConfig } from '../config';
import type { Cache } from '../cache/index';
import { renderStatsCard } from '@gitcard/svg-renderer';
import { createCardRoute } from './card-factory';

export function createStatsRoute(config: AppConfig, cache: Cache): Hono {
  return createCardRoute(config, cache, {
    path: '/stats/:username',
    cachePrefix: 'stats',
    render: (data, options) => renderStatsCard(data.stats, options),
  });
}
