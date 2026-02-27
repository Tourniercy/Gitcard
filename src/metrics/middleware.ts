import { createMiddleware } from 'hono/factory';
import { recordRequest } from './store';

export const metricsMiddleware = createMiddleware(async (c, next) => {
  const start = performance.now();
  await next();
  const latencyMs = performance.now() - start;
  const route = c.req.routePath ?? c.req.path;
  recordRequest(route, latencyMs, c.res.status);
});
