import type { Hono } from 'hono';
import type { AppConfig } from '../config';
import type { Cache } from '../cache/index';
import type { PatPool } from '../utils/pat-pool';
import { renderProfileCard } from '@gitcard/svg-renderer';
import { createCardRoute } from './card-factory';

export function createProfileRoute(config: AppConfig, cache: Cache, patPool: PatPool): Hono {
  return createCardRoute(config, cache, patPool, {
    path: '/stats/:username/profile',
    cachePrefix: 'profile',
    render: (data, options) => renderProfileCard(data.profile, options),
  });
}
