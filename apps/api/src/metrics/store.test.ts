import { beforeEach, describe, expect, it } from 'vitest';
import { recordRequest, getSnapshot, cacheCounters, resetMetrics } from './store';

describe('metrics store', () => {
  beforeEach(() => {
    resetMetrics();
  });

  it('records a request and returns it in snapshot', () => {
    recordRequest('/:username', 50, 200);
    const snap = getSnapshot();

    expect(snap.routes['/:username']).toBeDefined();
    expect(snap.routes['/:username']!.count).toBe(1);
    expect(snap.routes['/:username']!.statusCodes[200]).toBe(1);
  });

  it('computes latency percentiles', () => {
    for (let i = 1; i <= 100; i++) {
      recordRequest('/:username', i, 200);
    }
    const snap = getSnapshot();
    const route = snap.routes['/:username']!;

    expect(route.latency.p50).toBe(50);
    expect(route.latency.p95).toBe(95);
    expect(route.latency.p99).toBe(99);
  });

  it('tracks multiple routes independently', () => {
    recordRequest('/:username', 10, 200);
    recordRequest('/:username/streak', 20, 200);
    const snap = getSnapshot();

    expect(Object.keys(snap.routes)).toHaveLength(2);
    expect(snap.routes['/:username']!.count).toBe(1);
    expect(snap.routes['/:username/streak']!.count).toBe(1);
  });

  it('computes error rate', () => {
    recordRequest('/:username', 10, 200);
    recordRequest('/:username', 10, 200);
    recordRequest('/:username', 10, 404);
    recordRequest('/:username', 10, 502);
    const snap = getSnapshot();

    expect(snap.routes['/:username']!.errorRate).toBe(0.5);
  });

  it('tracks cache hit/miss counters', () => {
    cacheCounters.hits = 0;
    cacheCounters.misses = 0;

    cacheCounters.hits += 3;
    cacheCounters.misses += 1;

    const snap = getSnapshot();
    expect(snap.cache.hits).toBe(3);
    expect(snap.cache.misses).toBe(1);
    expect(snap.cache.hitRate).toBe(0.75);
  });

  it('reports zero hit rate when no cache activity', () => {
    const snap = getSnapshot();
    expect(snap.cache.hitRate).toBe(0);
  });

  it('bounds latency samples to 1000 per route', () => {
    for (let i = 0; i < 1100; i++) {
      recordRequest('/:username', i, 200);
    }
    const snap = getSnapshot();
    expect(snap.routes['/:username']!.count).toBe(1100);
  });

  it('resets all metrics', () => {
    recordRequest('/:username', 10, 200);
    cacheCounters.hits = 5;
    resetMetrics();

    const snap = getSnapshot();
    expect(Object.keys(snap.routes)).toHaveLength(0);
    expect(snap.cache.hits).toBe(0);
  });
});
