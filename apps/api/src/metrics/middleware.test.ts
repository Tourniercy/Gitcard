import { describe, expect, it, beforeEach } from 'vitest';
import { Hono } from 'hono';
import { metricsMiddleware } from './middleware';
import { getSnapshot, resetMetrics } from './store';

describe('metricsMiddleware', () => {
  beforeEach(() => {
    resetMetrics();
  });

  it('records request count and status', async () => {
    const app = new Hono();
    app.use('*', metricsMiddleware);
    app.get('/health', (c) => c.json({ status: 'ok' }));

    const res = await app.request('/health');
    expect(res.status).toBe(200);

    const snap = getSnapshot();
    expect(snap.routes['/health']).toBeDefined();
    expect(snap.routes['/health']!.count).toBe(1);
    expect(snap.routes['/health']!.statusCodes[200]).toBe(1);
  });

  it('records latency', async () => {
    const app = new Hono();
    app.use('*', metricsMiddleware);
    app.get('/slow', async (c) => {
      await new Promise((r) => setTimeout(r, 10));
      return c.json({ ok: true });
    });

    await app.request('/slow');
    const snap = getSnapshot();
    expect(snap.routes['/slow']!.latency.p50).toBeGreaterThan(0);
  });

  it('records error status codes', async () => {
    const app = new Hono();
    app.use('*', metricsMiddleware);
    app.get('/fail', (c) => c.json({ error: 'not found' }, 404));

    await app.request('/fail');
    const snap = getSnapshot();
    expect(snap.routes['/fail']!.statusCodes[404]).toBe(1);
    expect(snap.routes['/fail']!.errorRate).toBe(1);
  });
});
