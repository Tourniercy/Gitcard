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
