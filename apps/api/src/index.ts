import './telemetry';
import * as Sentry from '@sentry/node';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { loadConfig } from './config';
import { createCache } from './cache';
import { createPatPool } from './utils/pat-pool';
import { metricsMiddleware } from './metrics/middleware';
import { healthRoute } from './routes/health';
import { createStatsRoute } from './routes/stats';
import { createStreakRoute } from './routes/streak';
import { createTopLangsRoute } from './routes/top-langs';
import { createProfileRoute } from './routes/profile';
import { createDataRoute } from './routes/data';

const config = loadConfig();
const cache = createCache(config.redisUrl);
const patPool = createPatPool(config.pats);

cache.flush().catch((err) => {
  console.error('Failed to flush cache on startup:', err);
});

const app = new Hono();

// CORS — allow cross-origin requests from the web frontend
app.use(
  '*',
  cors({
    origin: config.corsOrigin ?? '*',
    allowMethods: ['GET'],
    maxAge: 86400,
  }),
);

// Metrics middleware — track all requests
app.use('*', metricsMiddleware);

// Chain routes for Hono RPC type inference
const routes = app
  .route('', healthRoute)
  .route('', createDataRoute(config, cache, patPool))
  .route('', createStatsRoute(config, cache, patPool))
  .route('', createStreakRoute(config, cache, patPool))
  .route('', createTopLangsRoute(config, cache, patPool))
  .route('', createProfileRoute(config, cache, patPool));

// 404 fallback
app.notFound((c) => {
  return c.json({ error: 'Not found' }, 404);
});

// Global error handler
app.onError((err, c) => {
  Sentry.captureException(err);
  console.error('Unhandled error:', err);
  return c.json({ error: 'Internal server error' }, 500);
});

export default app;
export type AppType = typeof routes;
