import type { IconName } from '@primer/octicons';
import type { CardOptions, GitHubStats } from '../types';
import { getTheme } from '../themes/index';
import {
  createCardWrapper,
  createRingChart,
  encodeHTML,
  formatNumber,
  octiconSvg,
} from './base-card';

const CARD_WIDTH = 495;
const CARD_HEIGHT = 195;

interface StatRow {
  key: string;
  label: string;
  value: number;
  icon: IconName;
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
    { key: 'stars', label: 'Total Stars', value: stats.totalStars, icon: 'star' },
    { key: 'forks', label: 'Total Forks', value: stats.totalForks, icon: 'repo-forked' },
    { key: 'commits', label: 'Total Commits', value: stats.totalCommits, icon: 'git-commit' },
    { key: 'prs', label: 'Total PRs', value: stats.totalPRs, icon: 'git-pull-request' },
    { key: 'issues', label: 'Total Issues', value: stats.totalIssues, icon: 'issue-opened' },
    { key: 'contribs', label: 'Contributed To', value: stats.contributedTo, icon: 'repo' },
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
        ? octiconSvg(stat.icon, statsStartX, y - 13, 16, iconColor)
        : '';
      const labelX = options.showIcons ? statsStartX + 22 : statsStartX;
      return `
        <g>
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
