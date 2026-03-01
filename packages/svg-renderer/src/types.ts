const LOCALE_CODES_LIST = [
  'en',
  'fr',
  'de',
  'es',
  'pt',
  'pt-BR',
  'it',
  'nl',
  'ja',
  'ko',
  'zh',
  'zh-TW',
  'ru',
  'ar',
  'hi',
  'pl',
  'tr',
  'sv',
  'da',
  'nb',
] as const;

export const SUPPORTED_LOCALES = LOCALE_CODES_LIST.map((code) => ({
  code,
  label: new Intl.DisplayNames([code], { type: 'language' }).of(code) ?? code,
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
