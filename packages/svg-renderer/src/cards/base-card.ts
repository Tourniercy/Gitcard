import octicons from '@primer/octicons';
import type { IconName } from '@primer/octicons';
import type { Theme } from '../themes/index';
import type { CardOptions } from '../types';
import { encodeHTML } from '../utils/sanitize';

export { encodeHTML } from '../utils/sanitize';

const FONT_FAMILY = "'Segoe UI', system-ui, -apple-system, sans-serif";

/**
 * Render a GitHub Octicon as an inline `<svg>` element for embedding inside card SVGs.
 */
export function octiconSvg(
  name: IconName,
  x: number,
  y: number,
  size: 16 | 24,
  fill: string,
): string {
  const variant = octicons[name].heights[size]!;
  return `<svg x="${x}" y="${y}" width="${size}" height="${size}" viewBox="0 0 ${size} ${variant.width}" fill="${fill}">${variant.path}</svg>`;
}

export function formatNumber(n: number, locale: string): string {
  return new Intl.NumberFormat(locale).format(n);
}

export function createArcPath(
  cx: number,
  cy: number,
  radius: number,
  startAngle: number,
  endAngle: number,
): string {
  const x1 = cx + radius * Math.cos(startAngle);
  const y1 = cy + radius * Math.sin(startAngle);
  const x2 = cx + radius * Math.cos(endAngle);
  const y2 = cy + radius * Math.sin(endAngle);
  const largeArc = endAngle - startAngle > Math.PI ? 1 : 0;

  return `M ${x1} ${y1} A ${radius} ${radius} 0 ${largeArc} 1 ${x2} ${y2}`;
}

let cardIdCounter = 0;

export function createCardWrapper(
  width: number,
  height: number,
  content: string,
  theme: Theme,
  options: CardOptions,
): string {
  const bg = options.bgColor ? `#${options.bgColor}` : theme.background;
  const borderColor = options.borderColor ? `#${options.borderColor}` : theme.border;
  const showBorder = !options.hideBorder;
  const radius = 8;
  const uid = cardIdCounter++;

  return `
<svg width="${width}" height="${height}" viewBox="0 0 ${width} ${height}"
     xmlns="http://www.w3.org/2000/svg">
  <defs>
    <clipPath id="clip-${uid}">
      <rect width="${width}" height="${height}" rx="${radius}" ry="${radius}" />
    </clipPath>
  </defs>

  <style>
    * { font-family: ${FONT_FAMILY}; }
    .title { font-size: 14px; font-weight: 600; fill: ${options.titleColor ? `#${options.titleColor}` : theme.title}; }
    .stat-label { font-size: 12px; fill: ${options.textColor ? `#${options.textColor}` : theme.muted}; }
    .stat-value { font-size: 14px; font-weight: 700; fill: ${options.textColor ? `#${options.textColor}` : theme.text}; }
    .muted { font-size: 11px; fill: ${theme.muted}; }
  </style>

  <g clip-path="url(#clip-${uid})">
    <rect width="${width}" height="${height}" rx="${radius}" ry="${radius}"
          fill="${bg}" />
    ${showBorder ? `<rect width="${width}" height="${height}" rx="${radius}" ry="${radius}" fill="none" stroke="${borderColor}" stroke-width="1" />` : ''}

    ${content}
  </g>
</svg>`.trim();
}

export function createRingChart(
  cx: number,
  cy: number,
  radius: number,
  percentage: number,
  color: string,
  bgColor: string,
  label: string,
): string {
  const circumference = 2 * Math.PI * radius;
  const offset = circumference * (1 - percentage / 100);

  return `
    <circle cx="${cx}" cy="${cy}" r="${radius}"
            fill="none" stroke="${bgColor}" stroke-width="6" opacity="0.2" />
    <circle cx="${cx}" cy="${cy}" r="${radius}"
            fill="none" stroke="${color}" stroke-width="6"
            stroke-dasharray="${circumference}"
            stroke-dashoffset="${offset}"
            stroke-linecap="round"
            transform="rotate(-90 ${cx} ${cy})"
            />
    <text x="${cx}" y="${cy}" text-anchor="middle" dominant-baseline="central"
          font-size="18" font-weight="700" fill="${color}">
      ${encodeHTML(label)}
    </text>
  `;
}

export function createDonutSegment(
  cx: number,
  cy: number,
  radius: number,
  startAngle: number,
  endAngle: number,
  color: string,
): string {
  const innerRadius = radius * 0.6;
  const d = createDonutArcPath(cx, cy, radius, innerRadius, startAngle, endAngle);
  return `<path d="${d}" fill="${color}" />`;
}

function createDonutArcPath(
  cx: number,
  cy: number,
  outerR: number,
  innerR: number,
  startAngle: number,
  endAngle: number,
): string {
  const startOuter = polarToCartesian(cx, cy, outerR, startAngle);
  const endOuter = polarToCartesian(cx, cy, outerR, endAngle);
  const startInner = polarToCartesian(cx, cy, innerR, endAngle);
  const endInner = polarToCartesian(cx, cy, innerR, startAngle);
  const largeArc = endAngle - startAngle > Math.PI ? 1 : 0;

  return [
    `M ${startOuter.x} ${startOuter.y}`,
    `A ${outerR} ${outerR} 0 ${largeArc} 1 ${endOuter.x} ${endOuter.y}`,
    `L ${startInner.x} ${startInner.y}`,
    `A ${innerR} ${innerR} 0 ${largeArc} 0 ${endInner.x} ${endInner.y}`,
    'Z',
  ].join(' ');
}

function polarToCartesian(
  cx: number,
  cy: number,
  radius: number,
  angle: number,
): { x: number; y: number } {
  return {
    x: cx + radius * Math.cos(angle - Math.PI / 2),
    y: cy + radius * Math.sin(angle - Math.PI / 2),
  };
}
