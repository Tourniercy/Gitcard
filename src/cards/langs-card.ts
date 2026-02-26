import type { CardOptions, TopLangsData } from '../types.js';
import { getTheme } from '../themes/index.js';
import { createCardWrapper, createDonutSegment, encodeHTML } from './base-card.js';

const CARD_WIDTH = 495;
const CARD_HEIGHT = 195;

export function renderLangsCard(data: TopLangsData, options: CardOptions): string {
  const theme = getTheme(options.theme);

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
    .map((lang, i) => {
      const sweepAngle = (lang.percentage / 100) * Math.PI * 2;
      // Leave a tiny gap between segments
      const gap = 0.02;
      const startAngle = currentAngle + gap;
      const endAngle = currentAngle + sweepAngle - gap;
      currentAngle += sweepAngle;

      if (sweepAngle < 0.05) return ''; // Skip tiny segments

      return `<g class="fade-in" style="animation-delay: ${i * 0.1}s;">
        ${createDonutSegment(donutCx, donutCy, donutRadius, startAngle, endAngle, lang.color)}
      </g>`;
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
        <g class="fade-in" style="animation-delay: ${(i + data.languages.length) * 0.05}s;">
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
