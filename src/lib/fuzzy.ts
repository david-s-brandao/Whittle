/** Subsequence fuzzy match. Returns a score (higher is better) or null if no match. */
export function fuzzyScore(query: string, text: string): number | null {
  const q = query.toLowerCase()
  const t = text.toLowerCase()
  if (!q) return 0
  let score = 0
  let ti = 0
  let prev = -2
  for (const ch of q) {
    const idx = t.indexOf(ch, ti)
    if (idx < 0) return null
    score += 1
    if (idx === prev + 1) score += 3 // consecutive
    if (idx === 0 || /[\s/_.-]/.test(t[idx - 1])) score += 2 // word start
    prev = idx
    ti = idx + 1
  }
  return score - t.length * 0.01
}

export function fuzzyFilter<T>(items: T[], query: string, text: (i: T) => string): T[] {
  if (!query.trim()) return items
  return items
    .map((item) => ({ item, s: fuzzyScore(query, text(item)) }))
    .filter((x): x is { item: T; s: number } => x.s !== null)
    .sort((a, b) => b.s - a.s)
    .map((x) => x.item)
}
