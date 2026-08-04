import { Hono, type Context } from 'hono';

export const healthRoute = new Hono();

const health = (c: Context) => c.json({ status: 'ok', timestamp: new Date().toISOString() });

healthRoute.get('/health', health);
// Alias so every rilcy app answers on the same path, which keeps the container
// health checks and the external uptime monitors uniform across projects.
healthRoute.get('/healthz', health);
