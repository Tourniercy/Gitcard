import type { Hono } from 'hono';
import type { AppConfig } from '../config.js';
import type { Cache } from '../cache/index.js';
import { renderLangsCard } from '../cards/langs-card.js';
import { createCardRoute } from './card-factory.js';

export function createTopLangsRoute(config: AppConfig, cache: Cache): Hono {
  return createCardRoute(config, cache, {
    path: '/:username/top-langs',
    cachePrefix: 'langs',
    render: (data, options) => renderLangsCard(data.languages, options),
  });
}
