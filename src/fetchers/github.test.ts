import { afterEach, describe, expect, it, vi } from 'vitest';
import { fetchGitHubData } from './github.js';

const mockUser = {
  name: 'Test User',
  login: 'testuser',
  contributionsCollection: {
    totalCommitContributions: 100,
    restrictedContributionsCount: 10,
    contributionCalendar: {
      totalContributions: 500,
      weeks: [],
    },
  },
  repositories: {
    totalCount: 20,
    nodes: [
      {
        stargazerCount: 50,
        forkCount: 10,
        languages: {
          edges: [
            { size: 1000, node: { name: 'TypeScript', color: '#3178c6' } },
            { size: 500, node: { name: 'Python', color: '#3572A5' } },
          ],
        },
      },
    ],
  },
  pullRequests: { totalCount: 30 },
  issues: { totalCount: 15 },
  followers: { totalCount: 200 },
  repositoriesContributedTo: { totalCount: 5 },
};

describe('fetchGitHubData', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('fetches and transforms user stats', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ data: { user: mockUser } }),
        headers: new Headers(),
      }),
    );

    const result = await fetchGitHubData('testuser', 'fake-token');

    expect(result.stats.username).toBe('testuser');
    expect(result.stats.totalStars).toBe(50);
    expect(result.stats.totalForks).toBe(10);
    expect(result.stats.totalCommits).toBe(110);
    expect(result.stats.totalPRs).toBe(30);
    expect(result.stats.totalIssues).toBe(15);
    expect(result.stats.contributedTo).toBe(5);
  });

  it('aggregates language data', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ data: { user: mockUser } }),
        headers: new Headers(),
      }),
    );

    const result = await fetchGitHubData('testuser', 'fake-token');

    expect(result.languages.languages).toHaveLength(2);
    expect(result.languages.languages[0]!.name).toBe('TypeScript');
    expect(result.languages.languages[0]!.percentage).toBeCloseTo(66.67, 1);
  });

  it('throws on user not found', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ data: { user: null } }),
        headers: new Headers(),
      }),
    );

    await expect(fetchGitHubData('ghost', 'fake-token')).rejects.toThrow('User not found: ghost');
  });

  it('throws on API error', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 403,
        statusText: 'Forbidden',
        headers: new Headers(),
      }),
    );

    await expect(fetchGitHubData('user', 'fake-token')).rejects.toThrow();
  });
});
