import { useQuery } from '@tanstack/react-query';
import type { GitHubStats, StreakData, TopLangsData, ProfileData } from '@gitcard/svg-renderer';
import { api } from '@/lib/api';

export interface GitHubData {
  stats: GitHubStats;
  streak: StreakData;
  languages: TopLangsData;
  profile: ProfileData;
}

export function useGitHubData(username: string) {
  return useQuery<GitHubData>({
    queryKey: ['github-data', username],
    queryFn: async () => {
      const res = await api.api.data[':username'].$get({
        param: { username },
      });
      if (!res.ok) {
        if (res.status === 404) throw new Error('User not found');
        throw new Error(`API error: ${res.status}`);
      }
      return (await res.json()) as GitHubData;
    },
    enabled: !!username,
  });
}
