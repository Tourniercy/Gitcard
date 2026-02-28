import type { Hono } from 'hono';
import type { AppConfig } from '../config';
import type { Cache } from '../cache/index';
import { renderStreakCard } from '@gitcard/svg-renderer';
import { createCardRoute } from './card-factory';

export function createStreakRoute(config: AppConfig, cache: Cache): Hono {
  return createCardRoute(config, cache, {
    path: '/stats/:username/streak',
    cachePrefix: 'streak',
    render: (data, options) => renderStreakCard(data.streak, options),
  });
}
