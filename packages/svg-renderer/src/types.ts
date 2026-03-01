const LOCALE_ENTRIES = [
  { code: 'en', country: 'GB' },
  { code: 'fr', country: 'FR' },
  { code: 'de', country: 'DE' },
  { code: 'es', country: 'ES' },
  { code: 'pt', country: 'PT' },
  { code: 'pt-BR', country: 'BR' },
  { code: 'it', country: 'IT' },
  { code: 'nl', country: 'NL' },
  { code: 'ja', country: 'JP' },
  { code: 'ko', country: 'KR' },
  { code: 'zh', country: 'CN' },
  { code: 'zh-TW', country: 'TW' },
  { code: 'ru', country: 'RU' },
  { code: 'ar', country: 'SA' },
  { code: 'hi', country: 'IN' },
  { code: 'pl', country: 'PL' },
  { code: 'tr', country: 'TR' },
  { code: 'sv', country: 'SE' },
  { code: 'da', country: 'DK' },
  { code: 'nb', country: 'NO' },
] as const;

function countryToFlag(country: string): string {
  return [...country].map((c) => String.fromCodePoint(0x1f1e6 - 65 + c.charCodeAt(0))).join('');
}

export const SUPPORTED_LOCALES = LOCALE_ENTRIES.map(({ code, country }) => ({
  code,
  label: new Intl.DisplayNames([code], { type: 'language' }).of(code) ?? code,
  flag: countryToFlag(country),
}));

export const LOCALE_CODES: string[] = SUPPORTED_LOCALES.map((l) => l.code);

export interface CardOptions {
  theme: string;
  hide: string[];
  showIcons: boolean;
  hideBorder: boolean;
  hideTitle: boolean;
  bgColor?: string;
  titleColor?: string;
  textColor?: string;
  iconColor?: string;
  borderColor?: string;
  cacheSeconds: number;
  locale: string;
  layout?: string;
}

export interface GitHubStats {
  username: string;
  name: string;
  totalStars: number;
  totalForks: number;
  totalCommits: number;
  totalPRs: number;
  totalIssues: number;
  contributedTo: number;
}

export interface StreakData {
  username: string;
  totalContributions: number;
  currentStreak: number;
  longestStreak: number;
  currentStreakStart: string;
  currentStreakEnd: string;
  longestStreakStart: string;
  longestStreakEnd: string;
}

export interface LanguageData {
  name: string;
  percentage: number;
  color: string;
  size: number;
}

export interface TopLangsData {
  username: string;
  languages: LanguageData[];
}

export interface ContributionDay {
  date: string;
  count: number;
}

export interface ProfileData {
  username: string;
  name: string;
  contributionsThisYear: number;
  publicRepos: number;
  createdAt: string;
  email: string | null;
  contributionCalendar: ContributionDay[];
}
