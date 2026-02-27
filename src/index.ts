import './telemetry';
import * as Sentry from '@sentry/node';
import { serveStatic } from '@hono/node-server/serve-static';
import { Hono } from 'hono';
import { loadConfig } from './config';
import { createCache } from './cache';
import { metricsMiddleware } from './metrics/middleware';
import { healthRoute } from './routes/health';
import { createStatsRoute } from './routes/stats';
import { createStreakRoute } from './routes/streak';
import { createTopLangsRoute } from './routes/top-langs';
import { createDataRoute } from './routes/data';

const config = loadConfig();
const cache = createCache(config.redisUrl);

const app = new Hono();

// Metrics middleware — track all requests
app.use('*', metricsMiddleware);

// Health check (no auth)
app.route('', healthRoute);

// Data API routes
app.route('', createDataRoute(config, cache));

// Card routes — all under /stats/:username
app.route('', createStatsRoute(config, cache));
app.route('', createStreakRoute(config, cache));
app.route('', createTopLangsRoute(config, cache));

// Serve frontend static files
app.use('/assets/*', serveStatic({ root: './dist/static' }));
app.get('/', serveStatic({ root: './dist/static', path: '/index.html' }));

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
