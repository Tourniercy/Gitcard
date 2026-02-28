import type { IconName } from '@primer/octicons';
import type { CardOptions, ContributionDay, ProfileData } from '../types';
import { getTheme } from '../themes/index';
import { createCardWrapper, encodeHTML, formatNumber, octiconSvg } from './base-card';

const CARD_WIDTH = 700;
const CARD_HEIGHT = 200;

// Chart area bounds
const CHART_LEFT = 320;
const CHART_RIGHT = 670;
const CHART_TOP = 45;
const CHART_BOTTOM = 155;

interface WeekBucket {
  label: string;
  count: number;
}

function aggregateWeekly(days: ContributionDay[]): WeekBucket[] {
  const buckets: WeekBucket[] = [];
  for (let i = 0; i < days.length; i += 7) {
    const week = days.slice(i, i + 7);
    const total = week.reduce((s, d) => s + d.count, 0);
    const mid = week[Math.floor(week.length / 2)];
    buckets.push({ label: mid?.date ?? '', count: total });
  }
  return buckets;
}

/**
 * Monotone cubic interpolation (Fritsch-Carlson) producing an SVG path.
 * Returns a path string starting from the first point.
 */
function monotoneCubicPath(points: { x: number; y: number }[]): string {
  const n = points.length;
  if (n < 2) return '';
  if (n === 2) return `L${points[1]!.x},${points[1]!.y}`;

  // Step 1: Compute slopes of secant lines
  const deltas: number[] = [];
  const slopes: number[] = [];
  for (let i = 0; i < n - 1; i++) {
    const dx = points[i + 1]!.x - points[i]!.x;
    const dy = points[i + 1]!.y - points[i]!.y;
    deltas.push(dx);
    slopes.push(dy / dx);
  }

  // Step 2: Compute tangents using Fritsch-Carlson method
  const tangents: number[] = [slopes[0]!];
  for (let i = 1; i < n - 1; i++) {
    const s0 = slopes[i - 1]!;
    const s1 = slopes[i]!;
    if (s0 * s1 <= 0) {
      tangents.push(0);
    } else {
      tangents.push(
        (3 * (deltas[i - 1]! + deltas[i]!)) /
          ((2 * deltas[i]! + deltas[i - 1]!) / s0 + (deltas[i]! + 2 * deltas[i - 1]!) / s1),
      );
    }
  }
  tangents.push(slopes[n - 2]!);

  // Step 3: Build cubic bezier segments
  const parts: string[] = [];
  for (let i = 0; i < n - 1; i++) {
    const dx = deltas[i]! / 3;
    const cp1x = points[i]!.x + dx;
    const cp1y = points[i]!.y + tangents[i]! * dx;
    const cp2x = points[i + 1]!.x - dx;
    const cp2y = points[i + 1]!.y - tangents[i + 1]! * dx;
    parts.push(`C${cp1x},${cp1y} ${cp2x},${cp2y} ${points[i + 1]!.x},${points[i + 1]!.y}`);
  }
  return parts.join(' ');
}

function accountAge(createdAt: string): string {
  const created = new Date(createdAt);
  const now = new Date();
  const years = now.getFullYear() - created.getFullYear();
  if (years < 1) {
    const months =
      (now.getFullYear() - created.getFullYear()) * 12 + now.getMonth() - created.getMonth();
    return `${months}mo`;
  }
  return `${years}y`;
}

function renderAreaChart(
  weeks: WeekBucket[],
  ringColor: string,
  mutedColor: string,
  textColor: string,
): string {
  if (weeks.length === 0) return '';

  const maxCount = Math.max(...weeks.map((w) => w.count), 1);

  // Build points
  const points = weeks.map((w, i) => ({
    x: CHART_LEFT + (i / (weeks.length - 1)) * (CHART_RIGHT - CHART_LEFT),
    y: CHART_BOTTOM - (w.count / maxCount) * (CHART_BOTTOM - CHART_TOP),
  }));

  // Build area path
  const first = points[0]!;
  const last = points[points.length - 1]!;
  const linePath = `M${first.x},${first.y} ${monotoneCubicPath(points)}`;
  const areaPath = `${linePath} L${last.x},${CHART_BOTTOM} L${first.x},${CHART_BOTTOM} Z`;

  // X-axis tick labels (every ~2 months = ~8 weeks)
  const xTicks: string[] = [];
  const tickInterval = Math.max(1, Math.floor(weeks.length / 6));
  for (let i = 0; i < weeks.length; i += tickInterval) {
    const w = weeks[i]!;
    const x = CHART_LEFT + (i / (weeks.length - 1)) * (CHART_RIGHT - CHART_LEFT);
    const d = new Date(w.label);
    const month = d.toLocaleString('en', { month: 'short' });
    xTicks.push(
      `<text x="${x}" y="${CHART_BOTTOM + 14}" text-anchor="middle" font-size="9" fill="${mutedColor}">${month}</text>`,
    );
  }

  // Y-axis ticks
  const mid = Math.round(maxCount / 2);
  const yTicks = [
    { val: 0, y: CHART_BOTTOM },
    { val: mid, y: CHART_BOTTOM - (mid / maxCount) * (CHART_BOTTOM - CHART_TOP) },
    { val: maxCount, y: CHART_TOP },
  ];
  const yTickSvg = yTicks.map(
    (t) =>
      `<text x="${CHART_LEFT - 6}" y="${t.y + 3}" text-anchor="end" font-size="9" fill="${mutedColor}">${t.val}</text>`,
  );

  // Grid lines
  const gridLines = yTicks.map(
    (t) =>
      `<line x1="${CHART_LEFT}" y1="${t.y}" x2="${CHART_RIGHT}" y2="${t.y}" stroke="${mutedColor}" stroke-opacity="0.15" stroke-width="0.5" />`,
  );

  return `
    ${gridLines.join('')}
    <path d="${areaPath}" fill="${ringColor}" fill-opacity="0.15" />
    <path d="${linePath}" fill="none" stroke="${ringColor}" stroke-width="1.5" />
    ${xTicks.join('')}
    ${yTickSvg.join('')}
    <text x="${(CHART_LEFT + CHART_RIGHT) / 2}" y="${CHART_BOTTOM + 28}" text-anchor="middle" font-size="10" fill="${textColor}">contributions in the last year</text>
  `;
}

export function renderProfileCard(data: ProfileData, options: CardOptions): string {
  const theme = getTheme(options.theme);
  const iconColor = options.iconColor ? `#${options.iconColor}` : theme.icon;
  const textColor = options.textColor ? `#${options.textColor}` : theme.text;

  const titleSection = options.hideTitle
    ? ''
    : `<text x="25" y="30" class="title">${encodeHTML(data.name || data.username)}</text>`;
  const titleOffset = options.hideTitle ? -10 : 0;

  const rows: { icon: IconName; label: string; value: string }[] = [
    {
      icon: 'history',
      label: 'Contributions',
      value: formatNumber(data.contributionsThisYear, options.locale),
    },
    { icon: 'repo', label: 'Public Repos', value: formatNumber(data.publicRepos, options.locale) },
    { icon: 'calendar', label: 'Account Age', value: accountAge(data.createdAt) },
  ];

  if (data.email) {
    rows.push({ icon: 'mail', label: 'Email', value: data.email });
  }

  const ROW_HEIGHT = 26;
  const startY = 55 + titleOffset;

  const rowsSvg = rows
    .map((row, i) => {
      const y = startY + i * ROW_HEIGHT;
      const iconPart = options.showIcons ? octiconSvg(row.icon, 25, y - 13, 16, iconColor) : '';
      const labelX = options.showIcons ? 47 : 25;
      return `
        <g>
          ${iconPart}
          <text x="${labelX}" y="${y}" class="stat-label">${encodeHTML(row.label)}</text>
          <text x="290" y="${y}" class="stat-value" text-anchor="end">${encodeHTML(row.value)}</text>
        </g>
      `;
    })
    .join('');

  // Vertical divider
  const divider = `<line x1="305" y1="15" x2="305" y2="185" stroke="${theme.border}" stroke-width="1" />`;

  // Contribution area chart
  const weeks = aggregateWeekly(data.contributionCalendar);
  const chart = renderAreaChart(weeks, theme.ring, theme.muted, textColor);

  const content = `
    ${titleSection}
    ${rowsSvg}
    ${divider}
    ${chart}
  `;

  return createCardWrapper(CARD_WIDTH, CARD_HEIGHT, content, theme, options);
}
