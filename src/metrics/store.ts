interface RouteMetrics {
  count: number;
  latencies: number[];
  statusCodes: Record<number, number>;
}

interface RouteSnapshot {
  count: number;
  latency: { p50: number; p95: number; p99: number };
  statusCodes: Record<number, number>;
  errorRate: number;
}

export interface MetricsSnapshot {
  routes: Record<string, RouteSnapshot>;
  cache: { hits: number; misses: number; hitRate: number };
  uptime: number;
  memoryMb: number;
}

const MAX_LATENCY_SAMPLES = 1000;

const store = new Map<string, RouteMetrics>();

export const cacheCounters = { hits: 0, misses: 0 };

export function recordRequest(route: string, latencyMs: number, status: number): void {
  let m = store.get(route);
  if (!m) {
    m = { count: 0, latencies: [], statusCodes: {} };
    store.set(route, m);
  }
  m.count++;
  if (m.latencies.length >= MAX_LATENCY_SAMPLES) {
    m.latencies.shift();
  }
  m.latencies.push(latencyMs);
  m.statusCodes[status] = (m.statusCodes[status] ?? 0) + 1;
}

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0;
  const idx = Math.floor((sorted.length - 1) * p);
  return sorted[idx]!;
}

export function getSnapshot(): MetricsSnapshot {
  const routes: Record<string, RouteSnapshot> = {};

  for (const [route, m] of store.entries()) {
    const sorted = [...m.latencies].toSorted((a, b) => a - b);
    const errors = Object.entries(m.statusCodes)
      .filter(([s]) => Number(s) >= 400)
      .reduce((sum, [, n]) => sum + n, 0);

    routes[route] = {
      count: m.count,
      latency: {
        p50: percentile(sorted, 0.5),
        p95: percentile(sorted, 0.95),
        p99: percentile(sorted, 0.99),
      },
      statusCodes: m.statusCodes,
      errorRate: m.count > 0 ? errors / m.count : 0,
    };
  }

  const total = cacheCounters.hits + cacheCounters.misses;

  return {
    routes,
    cache: {
      hits: cacheCounters.hits,
      misses: cacheCounters.misses,
      hitRate: total > 0 ? cacheCounters.hits / total : 0,
    },
    uptime: process.uptime(),
    memoryMb: Math.round((process.memoryUsage().rss / 1024 / 1024) * 100) / 100,
  };
}

export function resetMetrics(): void {
  store.clear();
  cacheCounters.hits = 0;
  cacheCounters.misses = 0;
}
