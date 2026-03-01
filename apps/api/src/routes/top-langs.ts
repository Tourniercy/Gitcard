import type { Hono } from 'hono';
import type { AppConfig } from '../config';
import type { Cache } from '../cache/index';
import type { PatPool } from '../utils/pat-pool';
import { renderLangsCard } from '@gitcard/svg-renderer';
import { createCardRoute } from './card-factory';

export function createTopLangsRoute(config: AppConfig, cache: Cache, patPool: PatPool): Hono {
  return createCardRoute(config, cache, patPool, {
    path: '/stats/:username/top-langs',
    cachePrefix: 'langs',
    render: (data, options) => renderLangsCard(data.languages, options),
  });
}
