import { useState, useEffect, useRef } from 'react';
import type { GitHubStats, StreakData, TopLangsData } from '@gitcard/svg-renderer';

export interface GitHubData {
  stats: GitHubStats;
  streak: StreakData;
  languages: TopLangsData;
}

interface UseGitHubDataReturn {
  data: GitHubData | null;
  isLoading: boolean;
  error: string | null;
}

const API_BASE = import.meta.env.VITE_API_URL ?? '';

export function useGitHubData(username: string): UseGitHubDataReturn {
  const [data, setData] = useState<GitHubData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cacheRef = useRef<Map<string, GitHubData>>(new Map());

  useEffect(() => {
    if (!username) {
      setData(null);
      setError(null);
      return;
    }

    const cached = cacheRef.current.get(username);
    if (cached) {
      setData(cached);
      setError(null);
      return;
    }

    let cancelled = false;
    setIsLoading(true);
    setError(null);

    fetch(`${API_BASE}/api/data/${encodeURIComponent(username)}`)
      .then(async (res) => {
        if (!res.ok) {
          if (res.status === 404) throw new Error('User not found');
          throw new Error(`API error: ${res.status}`);
        }
        return res.json() as Promise<GitHubData>;
      })
      .then((result) => {
        if (!cancelled) {
          cacheRef.current.set(username, result);
          setData(result);
          setIsLoading(false);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Unknown error');
          setData(null);
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [username]);

  return { data, isLoading, error };
}
