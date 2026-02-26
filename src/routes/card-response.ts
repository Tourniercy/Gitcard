import type { Context } from 'hono';
import { createHash } from 'node:crypto';

export function svgResponse(c: Context, svg: string, cacheSeconds: number): Response {
  const etag = createHash('md5').update(svg).digest('hex');

  c.header('Content-Type', 'image/svg+xml; charset=utf-8');
  c.header(
    'Cache-Control',
    `public, max-age=${cacheSeconds}, s-maxage=${cacheSeconds}, stale-while-revalidate=86400`,
  );
  c.header('ETag', `"${etag}"`);

  return c.body(svg);
}

export function errorSvg(message: string, width = 495, height = 120): string {
  return `
<svg width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" xmlns="http://www.w3.org/2000/svg">
  <rect width="${width}" height="${height}" rx="8" fill="#fef2f2" stroke="#fca5a5" />
  <text x="${width / 2}" y="${height / 2}" text-anchor="middle" dominant-baseline="central"
        font-family="'Segoe UI', system-ui, sans-serif" font-size="14" fill="#dc2626">
    ${message}
  </text>
</svg>`.trim();
}
