export interface GitHubGraphQLResponse {
  data?: {
    user: GitHubUser | null;
  };
  errors?: Array<{ message: string }>;
}

export interface GitHubUser {
  name: string | null;
  login: string;
  contributionsCollection: {
    totalCommitContributions: number;
    restrictedContributionsCount: number;
    contributionCalendar: {
      totalContributions: number;
      weeks: Array<{
        contributionDays: Array<{
          contributionCount: number;
          date: string;
        }>;
      }>;
    };
  };
  repositories: {
    totalCount: number;
    nodes: Array<{
      stargazerCount: number;
      forkCount: number;
      languages: {
        edges: Array<{
          size: number;
          node: {
            name: string;
            color: string | null;
          };
        }>;
      };
    }>;
  };
  pullRequests: {
    totalCount: number;
  };
  issues: {
    totalCount: number;
  };
  followers: {
    totalCount: number;
  };
  repositoriesContributedTo: {
    totalCount: number;
  };
}
