import type { CardOptions, StreakData } from '../types';
import { getTheme } from '../themes/index';
import { createCardWrapper, encodeHTML, formatNumber, octiconSvg } from './base-card';

const CARD_WIDTH = 495;
const CARD_HEIGHT = 195;

function formatDateRange(start: string, end: string): string {
  if (!start || !end) return '';
  const startDate = new Date(start);
  const endDate = new Date(end);
  const fmt = new Intl.DateTimeFormat('en', { month: 'short', day: 'numeric', year: 'numeric' });
  return `${fmt.format(startDate)} - ${fmt.format(endDate)}`;
}

export function renderStreakCard(data: StreakData, options: CardOptions): string {
  const theme = getTheme(options.theme);
  const accentColor = options.iconColor ? `#${options.iconColor}` : theme.ring;
  const textColor = options.textColor ? `#${options.textColor}` : theme.text;
  const mutedColor = theme.muted;

  const titleSection = options.hideTitle
    ? ''
    : `<text x="${CARD_WIDTH / 2}" y="28" class="title" text-anchor="middle">Contribution Streak</text>`;

  const titleOffset = options.hideTitle ? -10 : 0;

  const colWidth = CARD_WIDTH / 3;
  const col1X = colWidth / 2;
  const col2X = colWidth + colWidth / 2;
  const col3X = colWidth * 2 + colWidth / 2;

  const iconY = 44 + titleOffset;
  const labelY = 72 + titleOffset;
  const valueY = 110 + titleOffset;
  const unitY = 128 + titleOffset;
  const dateY = 158 + titleOffset;

  const content = `
    ${titleSection}

    <!-- Total Contributions -->
    <g class="fade-in" style="animation-delay: 0s;">
      ${octiconSvg('calendar', col1X - 8, iconY, 16, accentColor)}
      <text x="${col1X}" y="${labelY}" text-anchor="middle" class="stat-label">Contributions</text>
      <text x="${col1X}" y="${valueY}" text-anchor="middle"
            font-size="24" font-weight="700" fill="${textColor}">
        ${formatNumber(data.totalContributions, options.locale)}
      </text>
      <text x="${col1X}" y="${dateY}" text-anchor="middle" class="muted">
        ${encodeHTML(formatDateRange(data.currentStreakStart, data.currentStreakEnd))}
      </text>
    </g>

    <!-- Divider 1 -->
    <line x1="${colWidth}" y1="${55 + titleOffset}" x2="${colWidth}" y2="${170 + titleOffset}"
          stroke="${theme.border}" stroke-width="1" opacity="0.5" />

    <!-- Current Streak (emphasized) -->
    <g class="fade-in" style="animation-delay: 0.15s;">
      ${octiconSvg('flame', col2X - 8, iconY, 16, accentColor)}
      <text x="${col2X}" y="${labelY}" text-anchor="middle" class="stat-label">Current Streak</text>
      <text x="${col2X}" y="${valueY + 4}" text-anchor="middle"
            font-size="32" font-weight="800" fill="${accentColor}">
        ${data.currentStreak}
      </text>
      <text x="${col2X}" y="${unitY + 6}" text-anchor="middle"
            font-size="13" fill="${mutedColor}">days</text>
      <text x="${col2X}" y="${dateY}" text-anchor="middle" class="muted">
        ${encodeHTML(formatDateRange(data.currentStreakStart, data.currentStreakEnd))}
      </text>
    </g>

    <!-- Divider 2 -->
    <line x1="${colWidth * 2}" y1="${55 + titleOffset}" x2="${colWidth * 2}" y2="${170 + titleOffset}"
          stroke="${theme.border}" stroke-width="1" opacity="0.5" />

    <!-- Longest Streak -->
    <g class="fade-in" style="animation-delay: 0.3s;">
      ${octiconSvg('trophy', col3X - 8, iconY, 16, accentColor)}
      <text x="${col3X}" y="${labelY}" text-anchor="middle" class="stat-label">Longest Streak</text>
      <text x="${col3X}" y="${valueY}" text-anchor="middle"
            font-size="24" font-weight="700" fill="${textColor}">
        ${data.longestStreak}
      </text>
      <text x="${col3X}" y="${unitY}" text-anchor="middle"
            font-size="12" fill="${mutedColor}">days</text>
      <text x="${col3X}" y="${dateY}" text-anchor="middle" class="muted">
        ${encodeHTML(formatDateRange(data.longestStreakStart, data.longestStreakEnd))}
      </text>
    </g>
  `;

  return createCardWrapper(CARD_WIDTH, CARD_HEIGHT, content, theme, options);
}
