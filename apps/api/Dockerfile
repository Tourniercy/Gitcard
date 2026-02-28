# Build stage
FROM node:24-alpine AS builder

RUN corepack enable && corepack prepare pnpm@10.5.2 --activate

WORKDIR /app

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/api/package.json apps/api/
COPY packages/svg-renderer/package.json packages/svg-renderer/
RUN pnpm install --frozen-lockfile --ignore-scripts

COPY tsconfig.base.json ./
COPY packages/svg-renderer/ packages/svg-renderer/
COPY apps/api/ apps/api/

RUN pnpm --filter api build

# Production stage
FROM node:24-alpine AS runner

RUN corepack enable && corepack prepare pnpm@10.5.2 --activate

RUN addgroup -g 1001 -S app && adduser -S app -u 1001

WORKDIR /app

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/api/package.json apps/api/
COPY packages/svg-renderer/package.json packages/svg-renderer/
COPY packages/svg-renderer/src/ packages/svg-renderer/src/
RUN pnpm install --frozen-lockfile --prod --ignore-scripts --filter api

COPY --from=builder /app/apps/api/dist apps/api/dist

USER app

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:3000/health || exit 1

CMD ["node", "apps/api/dist/index.js"]
