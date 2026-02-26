import { Hono } from 'hono';
import { loadConfig } from './config.js';
import { createCache } from './cache/index.js';
import { healthRoute } from './routes/health.js';
import { createStatsRoute } from './routes/stats.js';
import { createStreakRoute } from './routes/streak.js';
import { createTopLangsRoute } from './routes/top-langs.js';

const config = loadConfig();
const cache = createCache(config.redisUrl);

const app = new Hono();

// Health check
app.route('', healthRoute);

// Card routes — order matters: more specific paths first
app.route('', createStreakRoute(config, cache));
app.route('', createTopLangsRoute(config, cache));
app.route('', createStatsRoute(config, cache));

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
