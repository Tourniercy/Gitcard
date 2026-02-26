import { beforeEach, describe, expect, it, vi } from 'vitest';

describe('loadConfig', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.unstubAllEnvs();
  });

  it('loads config with required PAT_1', async () => {
    vi.stubEnv('PAT_1', 'ghp_test123');
    vi.stubEnv('PORT', '4000');

    const { loadConfig } = await import('./config.js');
    const config = loadConfig();

    expect(config.pats).toEqual(['ghp_test123']);
    expect(config.port).toBe(4000);
    expect(config.redisUrl).toBeUndefined();
    expect(config.cacheTtl).toBe(14400);
    expect(config.logLevel).toBe('info');
  });

  it('collects multiple PATs', async () => {
    vi.stubEnv('PAT_1', 'a');
    vi.stubEnv('PAT_2', 'b');
    vi.stubEnv('PAT_3', 'c');

    const { loadConfig } = await import('./config.js');
    const config = loadConfig();

    expect(config.pats).toEqual(['a', 'b', 'c']);
  });

  it('throws if PAT_1 is missing', async () => {
    vi.stubEnv('PAT_1', '');

    const { loadConfig } = await import('./config.js');
    expect(() => loadConfig()).toThrow('At least PAT_1 must be set');
  });
});
