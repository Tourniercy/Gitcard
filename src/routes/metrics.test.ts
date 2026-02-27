import { describe, expect, it, beforeEach } from 'vitest';
import { Hono } from 'hono';
import { createMetricsRoute } from './metrics';
import { resetMetrics, recordRequest, cacheCounters } from '../metrics/store';

describe('metrics route', () => {
  const token = 'test-secret-token';
  let app: Hono;

  beforeEach(() => {
    resetMetrics();
    app = new Hono();
    app.route('', createMetricsRoute(token));
  });

  it('returns 401 without auth header', async () => {
    const res = await app.request('/metrics');
    expect(res.status).toBe(401);
  });

  it('returns 401 with wrong token', async () => {
    const res = await app.request('/metrics', {
      headers: { Authorization: 'Bearer wrong-token' },
    });
    expect(res.status).toBe(401);
  });

  it('returns metrics with valid token', async () => {
    recordRequest('/:username', 50, 200);
    cacheCounters.hits = 10;
    cacheCounters.misses = 5;

    const res = await app.request('/metrics', {
      headers: { Authorization: `Bearer ${token}` },
    });

    expect(res.status).toBe(200);
    const body = await res.json();

    expect(body.routes['/:username']).toBeDefined();
    expect(body.routes['/:username'].count).toBe(1);
    expect(body.cache.hits).toBe(10);
    expect(body.cache.misses).toBe(5);
    expect(body.cache.hitRate).toBeCloseTo(0.6667, 2);
    expect(body.uptime).toBeGreaterThan(0);
    expect(body.memoryMb).toBeGreaterThan(0);
  });
});

describe('metrics route disabled', () => {
  it('returns 404 when no token configured', async () => {
    const app = new Hono();
    app.route('', createMetricsRoute(undefined));

    const res = await app.request('/metrics');
    expect(res.status).toBe(404);
  });
});
