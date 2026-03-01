import * as Sentry from '@sentry/node';

import type { GitHubGraphQLResponse, GitHubUser } from './types';
import type { GitHubStats, StreakData, TopLangsData, ProfileData } from '@gitcard/svg-renderer';
import type { PatPool } from '../utils/pat-pool';

const GITHUB_GRAPHQL_URL = 'https://api.github.com/graphql';

const USER_QUERY = `
query($login: String!) {
  user(login: $login) {
    name
    login
    createdAt
    contributionsCollection {
      totalCommitContributions
      restrictedContributionsCount
      contributionCalendar {
        totalContributions
        weeks {
          contributionDays {
            contributionCount
            date
          }
        }
      }
    }
    repositories(first: 100, ownerAffiliations: OWNER, orderBy: {field: STARGAZERS, direction: DESC}) {
      totalCount
      nodes {
        stargazerCount
        forkCount
        languages(first: 10, orderBy: {field: SIZE, direction: DESC}) {
          edges {
            size
            node {
              name
              color
            }
          }
        }
      }
    }
    pullRequests(first: 1) {
      totalCount
    }
    issues(first: 1) {
      totalCount
    }
    followers {
      totalCount
    }
    repositoriesContributedTo(first: 1, contributionTypes: [COMMIT, ISSUE, PULL_REQUEST, REPOSITORY]) {
      totalCount
    }
  }
}
`;

export interface FetchResult {
  stats: GitHubStats;
  streak: StreakData;
  languages: TopLangsData;
  profile: ProfileData;
}

export class GitHubApiError extends Error {
  constructor(
    message: string,
    public status?: number,
  ) {
    super(message);
    this.name = 'GitHubApiError';
  }
}

export class GitHubNotFoundError extends GitHubApiError {
  constructor(public username: string) {
    super(`User not found: ${username}`, 404);
    this.name = 'GitHubNotFoundError';
  }
}

export class GitHubRateLimitError extends GitHubApiError {
  constructor(
    public remaining: number,
    public resetAt?: Date,
  ) {
    super('GitHub API rate limit exceeded', 429);
    this.name = 'GitHubRateLimitError';
  }
}

export async function fetchGitHubData(username: string, token: string): Promise<FetchResult> {
  return Sentry.startSpan(
    {
      name: `github.fetch ${username}`,
      op: 'http.client',
      attributes: { username },
    },
    async () => {
      const start = performance.now();

      try {
        const response = await fetch(GITHUB_GRAPHQL_URL, {
          method: 'POST',
          headers: {
            Authorization: `bearer ${token}`,
            'Content-Type': 'application/json',
            'User-Agent': 'github-stats-cards',
          },
          body: JSON.stringify({ query: USER_QUERY, variables: { login: username } }),
        });

        if (!response.ok) {
          if (response.status === 403 || response.status === 429) {
            const remaining = Number(response.headers.get('x-ratelimit-remaining') ?? 0);
            const resetEpoch = response.headers.get('x-ratelimit-reset');
            const resetAt = resetEpoch ? new Date(Number(resetEpoch) * 1000) : undefined;
            throw new GitHubRateLimitError(remaining, resetAt);
          }
          throw new GitHubApiError(
            `GitHub API error: ${response.status} ${response.statusText}`,
            response.status,
          );
        }

        const json = (await response.json()) as GitHubGraphQLResponse;

        if (json.errors?.length) {
          const messages = json.errors.map((e) => e.message).join(', ');
          if (messages.includes('Could not resolve to a User')) {
            throw new GitHubNotFoundError(username);
          }
          throw new GitHubApiError(`GraphQL errors: ${messages}`);
        }

        const user = json.data?.user;
        if (!user) {
          throw new GitHubNotFoundError(username);
        }

        const durationMs = performance.now() - start;
        Sentry.metrics.distribution('github_api_duration', durationMs, {
          unit: 'millisecond',
        });

        return {
          stats: extractStats(user, username),
          streak: extractStreak(user, username),
          languages: extractLanguages(user, username),
          profile: extractProfile(user, username),
        };
      } catch (err) {
        if (!(err instanceof GitHubNotFoundError) && !(err instanceof GitHubRateLimitError)) {
          Sentry.captureException(err, {
            tags: { username },
            extra: { operation: 'fetchGitHubData' },
          });
        }
        throw err;
      }
    },
  );
}

export async function fetchWithRetry(username: string, patPool: PatPool): Promise<FetchResult> {
  const token = patPool.getNextToken();
  try {
    return await fetchGitHubData(username, token);
  } catch (err) {
    if (err instanceof GitHubRateLimitError) {
      patPool.markExhausted(token);
      return await fetchGitHubData(username, patPool.getNextToken());
    }
    throw err;
  }
}

function extractStats(user: GitHubUser, username: string): GitHubStats {
  const totalStars = user.repositories.nodes.reduce((sum, r) => sum + r.stargazerCount, 0);
  const totalForks = user.repositories.nodes.reduce((sum, r) => sum + r.forkCount, 0);

  return {
    username,
    name: user.name ?? user.login,
    totalStars,
    totalForks,
    totalCommits:
      user.contributionsCollection.totalCommitContributions +
      user.contributionsCollection.restrictedContributionsCount,
    totalPRs: user.pullRequests.totalCount,
    totalIssues: user.issues.totalCount,
    contributedTo: user.repositoriesContributedTo.totalCount,
  };
}

function extractStreak(user: GitHubUser, username: string): StreakData {
  const calendar = user.contributionsCollection.contributionCalendar;
  const days = calendar.weeks.flatMap((w) => w.contributionDays);

  let currentStreak = 0;
  let longestStreak = 0;
  let tempStreak = 0;
  let currentStreakStart = '';
  let currentStreakEnd = '';
  let longestStreakStart = '';
  let longestStreakEnd = '';
  let tempStart = '';

  for (const day of days) {
    if (day.contributionCount > 0) {
      if (tempStreak === 0) tempStart = day.date;
      tempStreak++;
      if (tempStreak > longestStreak) {
        longestStreak = tempStreak;
        longestStreakStart = tempStart;
        longestStreakEnd = day.date;
      }
    } else {
      tempStreak = 0;
    }
  }

  // Current streak: count backwards from today
  currentStreak = 0;
  for (let i = days.length - 1; i >= 0; i--) {
    if (days[i]!.contributionCount > 0) {
      currentStreak++;
      currentStreakEnd = currentStreakEnd || days[i]!.date;
      currentStreakStart = days[i]!.date;
    } else if (currentStreak > 0) {
      break;
    }
  }

  return {
    username,
    totalContributions: calendar.totalContributions,
    currentStreak,
    longestStreak,
    currentStreakStart,
    currentStreakEnd,
    longestStreakStart,
    longestStreakEnd,
  };
}

function extractLanguages(user: GitHubUser, username: string): TopLangsData {
  const langMap = new Map<string, { size: number; color: string }>();

  for (const repo of user.repositories.nodes) {
    for (const edge of repo.languages.edges) {
      const existing = langMap.get(edge.node.name);
      if (existing) {
        existing.size += edge.size;
      } else {
        langMap.set(edge.node.name, { size: edge.size, color: edge.node.color ?? '#858585' });
      }
    }
  }

  const totalSize = Array.from(langMap.values()).reduce((sum, l) => sum + l.size, 0);

  const languages = Array.from(langMap.entries())
    .map(([name, { size, color }]) => ({
      name,
      size,
      color,
      percentage: totalSize > 0 ? (size / totalSize) * 100 : 0,
    }))
    .toSorted((a, b) => b.size - a.size)
    .slice(0, 5);

  return { username, languages };
}

function extractProfile(user: GitHubUser, username: string): ProfileData {
  const calendar = user.contributionsCollection.contributionCalendar;
  const contributionCalendar = calendar.weeks.flatMap((w) =>
    w.contributionDays.map((d) => ({ date: d.date, count: d.contributionCount })),
  );

  return {
    username,
    name: user.name ?? user.login,
    contributionsThisYear: calendar.totalContributions,
    publicRepos: user.repositories.totalCount,
    createdAt: user.createdAt,
    email: null,
    contributionCalendar,
  };
}
