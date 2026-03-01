import type { Hono } from 'hono';
import type { AppConfig } from '../config';
import type { Cache } from '../cache/index';
import type { PatPool } from '../utils/pat-pool';
import { renderStreakCard } from '@gitcard/svg-renderer';
import { createCardRoute } from './card-factory';

export function createStreakRoute(config: AppConfig, cache: Cache, patPool: PatPool): Hono {
  return createCardRoute(config, cache, patPool, {
    path: '/stats/:username/streak',
    cachePrefix: 'streak',
    render: (data, options) => renderStreakCard(data.streak, options),
  });
}
