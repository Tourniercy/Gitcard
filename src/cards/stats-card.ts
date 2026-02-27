import type { CardOptions, GitHubStats } from '../types';
import { getTheme } from '../themes/index';
import { createCardWrapper, createRingChart, encodeHTML, formatNumber } from './base-card';

const CARD_WIDTH = 495;
const CARD_HEIGHT = 195;

interface StatRow {
  key: string;
  label: string;
  value: number;
  icon: string;
}

// Octicon SVG paths (16x16 viewBox)
const ICONS: Record<string, string> = {
  star: 'M8 .25a.75.75 0 0 1 .673.418l1.882 3.815 4.21.612a.75.75 0 0 1 .416 1.279l-3.046 2.97.719 4.192a.75.75 0 0 1-1.088.791L8 12.347l-3.766 1.98a.75.75 0 0 1-1.088-.79l.72-4.194L.818 6.374a.75.75 0 0 1 .416-1.28l4.21-.611L7.327.668A.75.75 0 0 1 8 .25z',
  fork: 'M5 3.25a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0zm0 2.122a2.25 2.25 0 1 0-1.5 0v.878A2.25 2.25 0 0 0 5.75 8.5h1.5v2.128a2.251 2.251 0 1 0 1.5 0V8.5h1.5a2.25 2.25 0 0 0 2.25-2.25v-.878a2.25 2.25 0 1 0-1.5 0v.878a.75.75 0 0 1-.75.75h-4.5A.75.75 0 0 1 5 6.25v-.878zm3.75 7.378a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0zm3-8.75a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0z',
  commit:
    'M11.93 8.5a4.002 4.002 0 0 1-7.86 0H.75a.75.75 0 0 1 0-1.5h3.32a4.002 4.002 0 0 1 7.86 0h3.32a.75.75 0 0 1 0 1.5Zm-1.43-.5a2.5 2.5 0 1 0-5 0 2.5 2.5 0 0 0 5 0Z',
  pr: 'M1.5 3.25a2.25 2.25 0 1 1 3 2.122v5.256a2.251 2.251 0 1 1-1.5 0V5.372A2.25 2.25 0 0 1 1.5 3.25Zm5.677-.177L9.573.677A.25.25 0 0 1 10 .854V2.5h1A2.5 2.5 0 0 1 13.5 5v5.628a2.251 2.251 0 1 1-1.5 0V5a1 1 0 0 0-1-1h-1v1.646a.25.25 0 0 1-.427.177L7.177 3.427a.25.25 0 0 1 0-.354ZM3.75 2.5a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Zm0 9.5a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Zm8.25.75a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Z',
  issue:
    'M8 9.5a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3Z M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0ZM1.5 8a6.5 6.5 0 1 0 13 0 6.5 6.5 0 0 0-13 0Z',
  contrib:
    'M2 2.5A2.5 2.5 0 0 1 4.5 0h8.75a.75.75 0 0 1 .75.75v12.5a.75.75 0 0 1-.75.75h-2.5a.75.75 0 0 1 0-1.5h1.75v-2h-8a1 1 0 0 0-.714 1.7.75.75 0 1 1-1.072 1.05A2.495 2.495 0 0 1 2 11.5Zm10.5-1h-8a1 1 0 0 0-1 1v6.708A2.486 2.486 0 0 1 4.5 9h8Z',
};

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
    { key: 'forks', label: 'Total Forks', value: stats.totalForks, icon: 'fork' },
    { key: 'commits', label: 'Total Commits', value: stats.totalCommits, icon: 'commit' },
    { key: 'prs', label: 'Total PRs', value: stats.totalPRs, icon: 'pr' },
    { key: 'issues', label: 'Total Issues', value: stats.totalIssues, icon: 'issue' },
    { key: 'contribs', label: 'Contributed To', value: stats.contributedTo, icon: 'contrib' },
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
      const iconPath = ICONS[stat.icon];
      const iconPart =
        options.showIcons && iconPath
          ? `<svg x="${statsStartX}" y="${y - 13}" width="16" height="16" viewBox="0 0 16 16" fill="${iconColor}"><path fill-rule="evenodd" d="${iconPath}"/></svg>`
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
