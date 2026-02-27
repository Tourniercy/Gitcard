import type { CardOptions, StreakData } from '../types';
import { getTheme } from '../themes/index';
import { createCardWrapper, encodeHTML, formatNumber } from './base-card';

const CARD_WIDTH = 495;
const CARD_HEIGHT = 195;

// Octicon SVG paths (16x16 viewBox)
const ICON_FLAME =
  'M7.998 14.5c2.832 0 5-1.98 5-4.5 0-1.463-.68-2.19-1.879-3.383l-.036-.037c-1.013-1.008-2.3-2.29-2.834-4.434-.322.256-.63.579-.864.953-.432.696-.621 1.58-.046 2.73.473.947.67 2.284-.278 3.232-.61.61-1.545.84-2.403.525a2.236 2.236 0 0 1-1.447-1.794c-.014-.158-.032-.312-.053-.462a7.333 7.333 0 0 0-.274-1.2c-.663 1.107-.994 2.258-.994 3.37 0 2.52 2.168 4.5 5 4.5Zm4.258-12.78C13.363 2.964 15.498 5.057 15.498 8c0 3.044-2.686 6-7.5 6s-7.5-2.956-7.5-6c0-2.452 1.327-4.678 3.42-6.294.535-.413 1.32-.237 1.537.383.2.572.328 1.14.39 1.572.13.918.337 1.387.727 1.387.26 0 .363-.085.477-.24.106-.146.174-.378.174-.743 0-.762-.325-1.49-.636-2.083a7.46 7.46 0 0 0-.462-.735c-.39-.525-.162-1.305.457-1.494.517-.156 1.077-.257 1.655-.257.592 0 1.158.104 1.677.265Z';
const ICON_CALENDAR =
  'M4.75 0a.75.75 0 0 1 .75.75V2h5V.75a.75.75 0 0 1 1.5 0V2h1.25c.966 0 1.75.784 1.75 1.75v10.5A1.75 1.75 0 0 1 13.25 16H2.75A1.75 1.75 0 0 1 1 14.25V3.75C1 2.784 1.784 2 2.75 2H4V.75A.75.75 0 0 1 4.75 0ZM2.5 7.5v6.75c0 .138.112.25.25.25h10.5a.25.25 0 0 0 .25-.25V7.5Zm10.75-4H2.75a.25.25 0 0 0-.25.25V6h11V3.75a.25.25 0 0 0-.25-.25Z';
const ICON_TROPHY =
  'M3.217 6.962A3.75 3.75 0 0 1 0 3.25v-.5C0 1.784.784 1 1.75 1h1.356c.227-.601.791-1 1.394-1h6.5a1.5 1.5 0 0 1 1.394 1h1.356c.966 0 1.75.784 1.75 1.75v.5a3.75 3.75 0 0 1-3.217 3.712 5.014 5.014 0 0 1-2.783 2.862V12h2.25a.75.75 0 0 1 0 1.5h-6.5a.75.75 0 0 1 0-1.5H7.5V9.824a5.014 5.014 0 0 1-2.783-2.862ZM1.75 2.5a.25.25 0 0 0-.25.25v.5a2.25 2.25 0 0 0 1.75 2.194V2.5ZM13 5.444a2.25 2.25 0 0 0 1.5-2.194v-.5a.25.25 0 0 0-.25-.25H13ZM5 1.5v5a3.5 3.5 0 0 0 6.5 0v-5a.5.5 0 0 0-.5-.5H5.5a.5.5 0 0 0-.5.5Z';

function formatDateRange(start: string, end: string): string {
  if (!start || !end) return '';
  const startDate = new Date(start);
  const endDate = new Date(end);
  const fmt = new Intl.DateTimeFormat('en', { month: 'short', day: 'numeric' });
  return `${fmt.format(startDate)} - ${fmt.format(endDate)}`;
}

function formatDateYear(dateStr: string): string {
  if (!dateStr) return '';
  const date = new Date(dateStr);
  return `Jan 1 - ${new Intl.DateTimeFormat('en', { month: 'short', day: 'numeric' }).format(date)}, ${date.getFullYear()}`;
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

  const labelY = 62 + titleOffset;
  const valueY = 105 + titleOffset;
  const unitY = 125 + titleOffset;
  const dateY = 155 + titleOffset;

  const content = `
    ${titleSection}

    <!-- Total Contributions -->
    <g class="fade-in" style="animation-delay: 0s;">
      <svg x="${col1X - 8}" y="${labelY - 16}" width="16" height="16" viewBox="0 0 16 16" fill="${accentColor}"><path fill-rule="evenodd" d="${ICON_CALENDAR}"/></svg>
      <text x="${col1X}" y="${labelY + 4}" text-anchor="middle" class="stat-label">Total</text>
      <text x="${col1X}" y="${labelY + 20}" text-anchor="middle" class="stat-label">Contributions</text>
      <text x="${col1X}" y="${valueY}" text-anchor="middle"
            font-size="24" font-weight="700" fill="${textColor}">
        ${formatNumber(data.totalContributions, options.locale)}
      </text>
      <text x="${col1X}" y="${dateY}" text-anchor="middle" class="muted">
        ${encodeHTML(formatDateYear(data.currentStreakEnd))}
      </text>
    </g>

    <!-- Divider 1 -->
    <line x1="${colWidth}" y1="${55 + titleOffset}" x2="${colWidth}" y2="${165 + titleOffset}"
          stroke="${theme.border}" stroke-width="1" opacity="0.5" />

    <!-- Current Streak (emphasized) -->
    <g class="fade-in" style="animation-delay: 0.15s;">
      <svg x="${col2X - 8}" y="${labelY - 16}" width="16" height="16" viewBox="0 0 16 16" fill="${accentColor}"><path fill-rule="evenodd" d="${ICON_FLAME}"/></svg>
      <text x="${col2X}" y="${labelY + 4}" text-anchor="middle" class="stat-label">Current Streak</text>
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
    <line x1="${colWidth * 2}" y1="${55 + titleOffset}" x2="${colWidth * 2}" y2="${165 + titleOffset}"
          stroke="${theme.border}" stroke-width="1" opacity="0.5" />

    <!-- Longest Streak -->
    <g class="fade-in" style="animation-delay: 0.3s;">
      <svg x="${col3X - 8}" y="${labelY - 16}" width="16" height="16" viewBox="0 0 16 16" fill="${accentColor}"><path fill-rule="evenodd" d="${ICON_TROPHY}"/></svg>
      <text x="${col3X}" y="${labelY + 4}" text-anchor="middle" class="stat-label">Longest</text>
      <text x="${col3X}" y="${labelY + 20}" text-anchor="middle" class="stat-label">Streak</text>
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
