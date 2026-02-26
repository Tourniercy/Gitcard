import { describe, expect, it } from 'vitest';
import { Hono } from 'hono';
import { healthRoute } from './health.js';

describe('GET /health', () => {
  it('returns 200 with status ok', async () => {
    const app = new Hono();
    app.route('', healthRoute);

    const res = await app.request('/health');

    expect(res.status).toBe(200);
    const json = await res.json();
    expect(json.status).toBe('ok');
    expect(json.timestamp).toBeDefined();
  });
});
