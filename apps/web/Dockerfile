# Build stage
FROM node:24-alpine AS builder

RUN corepack enable && corepack prepare pnpm@10.5.2 --activate

WORKDIR /app

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/web/package.json apps/web/
COPY packages/svg-renderer/package.json packages/svg-renderer/
RUN pnpm install --frozen-lockfile --ignore-scripts

COPY tsconfig.base.json ./
COPY packages/svg-renderer/ packages/svg-renderer/
COPY apps/web/ apps/web/

ARG VITE_API_URL=https://api.gitcard.rilcy.app
ENV VITE_API_URL=$VITE_API_URL

RUN pnpm --filter web build

# Production stage — serve static files with nginx
FROM nginx:alpine AS runner

COPY apps/web/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/apps/web/dist /usr/share/nginx/html

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:80/ || exit 1
