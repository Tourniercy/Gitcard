import type { CardOptions, GitHubStats } from '../types.js';
import { getTheme } from '../themes/index.js';
import { createCardWrapper, createRingChart, encodeHTML, formatNumber } from './base-card.js';

const CARD_WIDTH = 495;
const CARD_HEIGHT = 195;

interface StatRow {
  key: string;
  label: string;
  value: number;
  icon: string;
}

function calculateGrade(stats: GitHubStats): { grade: string; percentage: number } {
  const score =
    stats.totalCommits * 1 +
    stats.totalPRs * 3 +
    stats.totalIssues * 2 +
    stats.totalStars * 1 +
    stats.totalForks * 0.5 +
    stats.contributedTo * 5;

  if (score >= 5000) return { grade: 'A+', percentage: 100 };
  if (score >= 2500) return { grade: 'A', percentage: 85 };
  if (score >= 1000) return { grade: 'B+', percentage: 70 };
  if (score >= 500) return { grade: 'B', percentage: 55 };
  return { grade: 'C', percentage: 40 };
}

export function renderStatsCard(stats: GitHubStats, options: CardOptions): string {
  const theme = getTheme(options.theme);
  const iconColor = options.iconColor ? `#${options.iconColor}` : theme.icon;
  const { grade, percentage } = calculateGrade(stats);

  const allStats: StatRow[] = [
    { key: 'stars', label: 'Total Stars', value: stats.totalStars, icon: '\u2605' },
    { key: 'forks', label: 'Total Forks', value: stats.totalForks, icon: '\u0192' },
    { key: 'commits', label: 'Total Commits', value: stats.totalCommits, icon: '\u25CB' },
    { key: 'prs', label: 'Total PRs', value: stats.totalPRs, icon: '\u25B3' },
    { key: 'issues', label: 'Total Issues', value: stats.totalIssues, icon: '!' },
    { key: 'contribs', label: 'Contributed To', value: stats.contributedTo, icon: '\u25C7' },
  ];

  const visibleStats = allStats.filter((s) => !options.hide.includes(s.key));

  const titleSection = options.hideTitle
    ? ''
    : `<text x="25" y="30" class="title">${encodeHTML(stats.name)}&#39;s GitHub Stats</text>`;

  const titleOffset = options.hideTitle ? 0 : 10;
  const ringCx = 75;
  const ringCy = 105 + titleOffset;
  const ringRadius = 40;

  const ringSection = createRingChart(
    ringCx,
    ringCy,
    ringRadius,
    percentage,
    theme.ring,
    theme.muted,
    grade,
  );

  const statsStartX = 160;
  const statsStartY = 55 + titleOffset;
  const rowHeight = 22;

  const statsSection = visibleStats
    .map((stat, i) => {
      const y = statsStartY + i * rowHeight;
      const iconPart = options.showIcons
        ? `<text x="${statsStartX}" y="${y}" font-size="14" fill="${iconColor}">${stat.icon}</text>`
        : '';
      const labelX = options.showIcons ? statsStartX + 22 : statsStartX;
      return `
        <g class="fade-in" style="animation-delay: ${i * 0.1}s;">
          ${iconPart}
          <text x="${labelX}" y="${y}" class="stat-label">${encodeHTML(stat.label)}</text>
          <text x="${CARD_WIDTH - 25}" y="${y}" class="stat-value" text-anchor="end">${formatNumber(stat.value, options.locale)}</text>
        </g>
      `;
    })
    .join('');

  const content = `
    ${titleSection}
    ${ringSection}
    ${statsSection}
  `;

  return createCardWrapper(CARD_WIDTH, CARD_HEIGHT, content, theme, options);
}
