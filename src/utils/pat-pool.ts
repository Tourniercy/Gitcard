interface PatPool {
  getNextToken(): string;
  markExhausted(token: string): void;
}

const COOLDOWN_MS = 60 * 60 * 1000; // 1 hour

export function createPatPool(tokens: string[]): PatPool {
  if (tokens.length === 0) {
    throw new Error('At least one PAT is required');
  }

  let index = 0;
  const exhaustedUntil = new Map<string, number>();

  function isAvailable(token: string): boolean {
    const until = exhaustedUntil.get(token);
    if (until === undefined) return true;
    if (Date.now() >= until) {
      exhaustedUntil.delete(token);
      return true;
    }
    return false;
  }

  return {
    getNextToken(): string {
      const startIndex = index;
      do {
        const token = tokens[index % tokens.length]!;
        index = (index + 1) % tokens.length;
        if (isAvailable(token)) return token;
      } while (index !== startIndex);

      throw new Error('All PAT tokens are rate-limited');
    },

    markExhausted(token: string): void {
      exhaustedUntil.set(token, Date.now() + COOLDOWN_MS);
    },
  };
}
