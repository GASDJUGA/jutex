/**
 * Order of spelling suggestions: Hunspell lists them in no useful order.
 */

/** Most suggestions offered for a word. */
export const MAX_SUGGESTIONS = 6;

const base = (c: string) => c.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();

/**
 * How far `b` is from `a` for a typist: edit distance where swapping two neighbours or
 * changing only an accent or the case costs less than other mistakes.
 */
export function typingDistance(a: string, b: string): number {
  const x = [...a];
  const y = [...b];
  const d: number[][] = Array.from({ length: x.length + 1 }, (_, i) =>
    Array.from({ length: y.length + 1 }, (_, j) => (i === 0 ? j : j === 0 ? i : 0)),
  );
  for (let i = 1; i <= x.length; i++) {
    for (let j = 1; j <= y.length; j++) {
      const same = x[i - 1] === y[j - 1];
      const substitution = same ? 0 : base(x[i - 1]) === base(y[j - 1]) ? 0.25 : 1;
      d[i][j] = Math.min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + substitution);
      if (i > 1 && j > 1 && x[i - 1] === y[j - 2] && x[i - 2] === y[j - 1]) {
        d[i][j] = Math.min(d[i][j], d[i - 2][j - 2] + 0.5);
      }
    }
  }
  return d[x.length][y.length];
}

/** The dictionary's suggestions, closest first. */
export function rankSuggestions(word: string, suggestions: string[]): string[] {
  return suggestions
    .map((s, index) => ({ s, index, cost: typingDistance(word, s) }))
    .sort((p, q) => p.cost - q.cost || p.index - q.index)
    .map((p) => p.s)
    .slice(0, MAX_SUGGESTIONS);
}
