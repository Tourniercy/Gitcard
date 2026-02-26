import type { Hono } from 'hono';
import type { AppConfig } from '../config.js';
import type { Cache } from '../cache/index.js';
import { renderStreakCard } from '../cards/streak-card.js';
import { createCardRoute } from './card-factory.js';

export function createStreakRoute(config: AppConfig, cache: Cache): Hono {
  return createCardRoute(config, cache, {
    path: '/:username/streak',
    cachePrefix: 'streak',
    render: (data, options) => renderStreakCard(data.streak, options),
  });
}
