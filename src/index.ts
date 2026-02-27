import { Hono } from 'hono';
import { loadConfig } from './config';
import { createCache } from './cache';
import { metricsMiddleware } from './metrics/middleware';
import { healthRoute } from './routes/health';
import { createMetricsRoute } from './routes/metrics';
import { createStatsRoute } from './routes/stats';
import { createStreakRoute } from './routes/streak';
import { createTopLangsRoute } from './routes/top-langs';

const config = loadConfig();
const cache = createCache(config.redisUrl);

const app = new Hono();

// Metrics middleware — track all requests
app.use('*', metricsMiddleware);

// Health check (no auth)
app.route('', healthRoute);

// Metrics (bearer auth protected)
app.route('', createMetricsRoute(config.metricsToken));

// Card routes — all under /stats/:username
app.route('', createStatsRoute(config, cache));
app.route('', createStreakRoute(config, cache));
app.route('', createTopLangsRoute(config, cache));

// 404 fallback
app.notFound((c) => {
  return c.json({ error: 'Not found' }, 404);
});

// Global error handler
app.onError((err, c) => {
  console.error('Unhandled error:', err);
  return c.json({ error: 'Internal server error' }, 500);
});

export default app;
