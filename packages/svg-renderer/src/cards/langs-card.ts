import type { Theme } from '../themes/index';
import type { CardOptions, TopLangsData } from '../types';
import { getTheme } from '../themes/index';
import { createCardWrapper, createDonutSegment, encodeHTML } from './base-card';

const CARD_WIDTH = 495;
const CARD_HEIGHT = 195;

const COMPACT_WIDTH = 300;

function renderCompactLangsCard(data: TopLangsData, options: CardOptions, theme: Theme): string {
  const titleSection = options.hideTitle
    ? ''
    : `<text x="25" y="30" class="title">Most Used Languages</text>`;

  const barY = options.hideTitle ? 20 : 48;
  const barHeight = 8;
  const barLeft = 25;
  const barWidth = COMPACT_WIDTH - 50;

  // Stacked progress bar segments
  let barX = barLeft;
  const barSegments = data.languages
    .map((lang) => {
      const w = (lang.percentage / 100) * barWidth;
      const segment = `<rect x="${barX}" y="${barY}" width="${w}" height="${barHeight}" fill="${lang.color}">
        <animate attributeName="width" from="0" to="${w}" dur="0.6s" fill="freeze" />
      </rect>`;
      barX += w;
      return segment;
    })
    .join('');

  // Clip path for rounded corners on the bar
  const barClipId = 'compact-bar-clip';
  const barClip = `<clipPath id="${barClipId}"><rect x="${barLeft}" y="${barY}" width="${barWidth}" height="${barHeight}" rx="5" ry="5" /></clipPath>`;

  // 2-column legend below bar
  const legendStartY = barY + barHeight + 18;
  const legendRowHeight = 25;
  const colWidth = (COMPACT_WIDTH - 50) / 2;

  const legendItems = data.languages
    .map((lang, i) => {
      const col = i % 2;
      const row = Math.floor(i / 2);
      const x = barLeft + col * colWidth;
      const y = legendStartY + row * legendRowHeight;
      const delay = (450 + i * 150).toFixed(0);
      return `
        <g opacity="0">
          <circle cx="${x}" cy="${y - 4}" r="5" fill="${lang.color}" />
          <text x="${x + 14}" y="${y}" class="lang-name" font-size="11" fill="${options.textColor ? `#${options.textColor}` : theme.text}">${encodeHTML(lang.name)} ${lang.percentage.toFixed(2)}%</text>
          <animate attributeName="opacity" from="0" to="1" dur="0.3s" begin="${delay}ms" fill="freeze" />
        </g>
      `;
    })
    .join('');

  const legendRows = Math.ceil(data.languages.length / 2);
  const dynamicHeight = legendStartY + legendRows * legendRowHeight + 10;

  const emptyMessage =
    data.languages.length === 0
      ? `<text x="${COMPACT_WIDTH / 2}" y="${dynamicHeight / 2}" text-anchor="middle" class="muted">No language data available</text>`
      : '';

  const content = `
    <defs>${barClip}</defs>
    ${titleSection}
    <g clip-path="url(#${barClipId})">${barSegments}</g>
    ${legendItems}
    ${emptyMessage}
  `;

  return createCardWrapper(COMPACT_WIDTH, dynamicHeight, content, theme, options);
}

export function renderLangsCard(data: TopLangsData, options: CardOptions): string {
  const theme = getTheme(options.theme);

  if (options.layout === 'compact') return renderCompactLangsCard(data, options, theme);

  const titleSection = options.hideTitle
    ? ''
    : `<text x="25" y="30" class="title">Most Used Languages</text>`;

  const titleOffset = options.hideTitle ? -10 : 0;
  const donutCx = 90;
  const donutCy = 110 + titleOffset;
  const donutRadius = 55;

  // Build donut segments
  let currentAngle = 0;
  const donutSegments = data.languages
    .map((lang) => {
      const sweepAngle = (lang.percentage / 100) * Math.PI * 2;
      // Leave a tiny gap between segments
      const gap = 0.02;
      const startAngle = currentAngle + gap;
      const endAngle = currentAngle + sweepAngle - gap;
      currentAngle += sweepAngle;

      if (sweepAngle < 0.05) return ''; // Skip tiny segments

      return createDonutSegment(donutCx, donutCy, donutRadius, startAngle, endAngle, lang.color);
    })
    .join('');

  // Build legend
  const legendX = 185;
  const legendStartY = 60 + titleOffset;
  const legendRowHeight = 24;

  const legendItems = data.languages
    .map((lang, i) => {
      const y = legendStartY + i * legendRowHeight;
      return `
        <g>
          <circle cx="${legendX}" cy="${y - 4}" r="5" fill="${lang.color}" />
          <text x="${legendX + 14}" y="${y}" class="stat-label">${encodeHTML(lang.name)}</text>
          <text x="${CARD_WIDTH - 25}" y="${y}" class="stat-value" text-anchor="end">${lang.percentage.toFixed(1)}%</text>
        </g>
      `;
    })
    .join('');

  const emptyMessage =
    data.languages.length === 0
      ? `<text x="${CARD_WIDTH / 2}" y="${CARD_HEIGHT / 2}" text-anchor="middle" class="muted">No language data available</text>`
      : '';

  const content = `
    ${titleSection}
    ${donutSegments}
    ${legendItems}
    ${emptyMessage}
  `;

  return createCardWrapper(CARD_WIDTH, CARD_HEIGHT, content, theme, options);
}
