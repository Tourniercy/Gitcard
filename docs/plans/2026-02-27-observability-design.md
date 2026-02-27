# Observability Design — Sentry + OpenTelemetry

Date: 2026-02-27

## Overview

Replace basic in-memory metrics with full observability via Sentry SaaS. Sentry is the single platform for error tracking, performance monitoring, traces, and custom metrics. No Grafana, Prometheus, or Jaeger needed.

## Architecture

```
App (Hono)
  ├── @sentry/node           → Error capture + performance monitoring
  ├── @sentry/opentelemetry  → OTel span processor (traces → Sentry)
  └── Sentry.metrics.*       → Custom counters (total cards, cache hits/misses)

Existing (unchanged):
  └── /metrics endpoint      → Stays for local dev (in-memory JSON)
```

## Env Vars

| Var                         | Required | Default        | Description                                     |
| --------------------------- | -------- | -------------- | ----------------------------------------------- |
| `SENTRY_DSN`                | No       | —              | Sentry project DSN. If absent, Sentry disabled. |
| `SENTRY_TRACES_SAMPLE_RATE` | No       | `0.1`          | Fraction of requests to trace (0.0–1.0)         |
| `SENTRY_ENVIRONMENT`        | No       | `"production"` | Environment tag in Sentry                       |

## What Sentry Captures Automatically

- Every unhandled error with full stack trace
- Request performance (duration, status) for every transaction
- Node.js runtime metrics (memory, event loop lag)

## Custom Instrumentation

| Metric                  | Type         | Labels                           | Where                |
| ----------------------- | ------------ | -------------------------------- | -------------------- |
| `cards_generated_total` | Counter      | `card_type` (stats/streak/langs) | `card-factory.ts`    |
| `cache_hits_total`      | Counter      | —                                | `cache/index.ts`     |
| `cache_misses_total`    | Counter      | —                                | `cache/index.ts`     |
| `github_api_duration`   | Distribution | —                                | `fetchers/github.ts` |

## Integration Points

| File                         | Instrumentation                                                                                                     |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `src/telemetry.ts` (new)     | `Sentry.init()` with `@sentry/opentelemetry`. Must be imported before all other modules.                            |
| `src/index.ts`               | Import telemetry first. `Sentry.captureException(err)` in global error handler.                                     |
| `src/routes/card-factory.ts` | `Sentry.startSpan()` wrapping GitHub fetch + card render. Increment `cards_generated_total`.                        |
| `src/cache/index.ts`         | `Sentry.startSpan()` for get/set. Increment `cache_hits_total` / `cache_misses_total` via `Sentry.metrics`.         |
| `src/fetchers/github.ts`     | `Sentry.startSpan()` wrapping fetch call. `Sentry.captureException()` on errors with context (username, card type). |
| `src/config.ts`              | Add `SENTRY_DSN`, `SENTRY_TRACES_SAMPLE_RATE`, `SENTRY_ENVIRONMENT` to `createEnv()` and `AppConfig`.               |

## Dependencies

### Add

- `@sentry/node`
- `@sentry/opentelemetry`

### Unchanged

- Existing in-memory `src/metrics/` module + `/metrics` endpoint stays for local dev

## Graceful Degradation

If `SENTRY_DSN` is not set, telemetry.ts is a no-op. The app works identically to before — in-memory metrics only. Zero runtime cost when Sentry is disabled.
