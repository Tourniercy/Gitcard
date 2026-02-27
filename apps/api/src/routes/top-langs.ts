import type { Hono } from 'hono';
import type { AppConfig } from '../config';
import type { Cache } from '../cache/index';
import { renderLangsCard } from '../cards/langs-card';
import { createCardRoute } from './card-factory';

export function createTopLangsRoute(config: AppConfig, cache: Cache): Hono {
  return createCardRoute(config, cache, {
    path: '/stats/:username/top-langs',
    cachePrefix: 'langs',
    render: (data, options) => renderLangsCard(data.languages, options),
  });
}
