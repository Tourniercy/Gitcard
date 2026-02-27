import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@sentry/node', () => ({
  init: vi.fn(),
  isInitialized: vi.fn(() => false),
}));

describe('telemetry', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    vi.unstubAllEnvs();
  });

  it('calls Sentry.init when SENTRY_DSN is set', async () => {
    vi.stubEnv('SENTRY_DSN', 'https://key@sentry.io/123');
    vi.stubEnv('SENTRY_TRACES_SAMPLE_RATE', '0.5');
    vi.stubEnv('SENTRY_ENVIRONMENT', 'staging');

    const Sentry = await import('@sentry/node');
    await import('./telemetry');

    expect(Sentry.init).toHaveBeenCalledWith(
      expect.objectContaining({
        dsn: 'https://key@sentry.io/123',
        tracesSampleRate: 0.5,
        environment: 'staging',
      }),
    );
  });

  it('does not call Sentry.init when SENTRY_DSN is absent', async () => {
    vi.stubEnv('SENTRY_DSN', '');

    const Sentry = await import('@sentry/node');
    await import('./telemetry');

    expect(Sentry.init).not.toHaveBeenCalled();
  });
});
