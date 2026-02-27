import { beforeEach, describe, expect, it, vi } from 'vitest';

describe('loadConfig', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.unstubAllEnvs();
  });

  it('loads config with required PAT_1', async () => {
    vi.stubEnv('PAT_1', 'ghp_test123');
    vi.stubEnv('PORT', '4000');
    vi.stubEnv('METRICS_TOKEN', '');

    const { loadConfig } = await import('./config');
    const config = loadConfig();

    expect(config.pats).toEqual(['ghp_test123']);
    expect(config.port).toBe(4000);
    expect(config.redisUrl).toBeUndefined();
    expect(config.cacheTtl).toBe(14400);
    expect(config.logLevel).toBe('info');
    expect(config.metricsToken).toBeUndefined();
  });

  it('collects multiple PATs', async () => {
    vi.stubEnv('PAT_1', 'a');
    vi.stubEnv('PAT_2', 'b');
    vi.stubEnv('PAT_3', 'c');

    const { loadConfig } = await import('./config');
    const config = loadConfig();

    expect(config.pats).toEqual(['a', 'b', 'c']);
  });

  it('throws if PAT_1 is missing', async () => {
    vi.stubEnv('PAT_1', '');

    await expect(() => import('./config')).rejects.toThrow();
  });

  it('loads METRICS_TOKEN when set', async () => {
    vi.stubEnv('PAT_1', 'ghp_test');
    vi.stubEnv('METRICS_TOKEN', 'secret-metrics-token');

    const { loadConfig } = await import('./config');
    const config = loadConfig();

    expect(config.metricsToken).toBe('secret-metrics-token');
  });

  it('uses default values', async () => {
    vi.stubEnv('PAT_1', 'ghp_test');

    const { loadConfig } = await import('./config');
    const config = loadConfig();

    expect(config.port).toBe(3000);
    expect(config.cacheTtl).toBe(14400);
    expect(config.logLevel).toBe('info');
  });

  it('loads Sentry config when set', async () => {
    vi.stubEnv('PAT_1', 'ghp_test');
    vi.stubEnv('METRICS_TOKEN', '');
    vi.stubEnv('SENTRY_DSN', 'https://key@o123.ingest.sentry.io/456');
    vi.stubEnv('SENTRY_TRACES_SAMPLE_RATE', '0.5');
    vi.stubEnv('SENTRY_ENVIRONMENT', 'staging');

    const { loadConfig } = await import('./config');
    const config = loadConfig();

    expect(config.sentryDsn).toBe('https://key@o123.ingest.sentry.io/456');
    expect(config.sentryTracesSampleRate).toBe(0.5);
    expect(config.sentryEnvironment).toBe('staging');
  });

  it('uses Sentry defaults when not set', async () => {
    vi.stubEnv('PAT_1', 'ghp_test');
    vi.stubEnv('METRICS_TOKEN', '');
    vi.stubEnv('SENTRY_DSN', '');
    vi.stubEnv('SENTRY_TRACES_SAMPLE_RATE', '');
    vi.stubEnv('SENTRY_ENVIRONMENT', '');

    const { loadConfig } = await import('./config');
    const config = loadConfig();

    expect(config.sentryDsn).toBeUndefined();
    expect(config.sentryTracesSampleRate).toBe(0.1);
    expect(config.sentryEnvironment).toBe('production');
  });
});
