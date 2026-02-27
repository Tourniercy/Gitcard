import { Hono } from 'hono';
import { bearerAuth } from 'hono/bearer-auth';
import { getSnapshot } from '../metrics/store';

export function createMetricsRoute(metricsToken?: string): Hono {
  const app = new Hono();

  if (!metricsToken) {
    app.get('/metrics', (c) => c.json({ error: 'Metrics not configured' }, 404));
    return app;
  }

  app.use('/metrics', bearerAuth({ token: metricsToken }));

  app.get('/metrics', (c) => {
    return c.json(getSnapshot());
  });

  return app;
}
