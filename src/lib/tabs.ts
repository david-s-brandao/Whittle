/** Pure helpers for the tab strip, kept separate so they can be tested. */

export interface SavedTabs {
  open: string[]
  active: string | null
}

/** Add `path` if missing (appended at the end). */
export function withTab(tabs: string[], path: string): string[] {
  return tabs.includes(path) ? tabs : [...tabs, path]
}

/** Remove `path`; returns the new list and which tab to show next (neighbour to the right, else left). */
export function withoutTab(tabs: string[], path: string): { tabs: string[]; next: string | null } {
  const i = tabs.indexOf(path)
  if (i < 0) return { tabs, next: null }
  const rest = tabs.filter((t) => t !== path)
  return { tabs: rest, next: rest[i] ?? rest[i - 1] ?? null }
}

/** Next/previous tab with wrap-around (`dir` is 1 or -1). */
export function cycle(tabs: string[], current: string | null, dir: 1 | -1): string | null {
  if (!tabs.length) return null
  const i = current ? tabs.indexOf(current) : -1
  if (i < 0) return tabs[0]
  return tabs[(i + dir + tabs.length) % tabs.length]
}

/** Keep only tabs that are still known projects; fall back to the first tab if the active one is gone. */
export function restore(saved: SavedTabs, known: string[]): SavedTabs {
  const open = saved.open.filter((p, i, a) => known.includes(p) && a.indexOf(p) === i)
  const active = saved.active && open.includes(saved.active) ? saved.active : (open[0] ?? null)
  return { open, active }
}

export function parseSaved(raw: string | null): SavedTabs {
  try {
    const v = raw ? (JSON.parse(raw) as Partial<SavedTabs>) : {}
    const open = Array.isArray(v.open)
      ? v.open.filter((p): p is string => typeof p === 'string')
      : []
    return { open, active: typeof v.active === 'string' ? v.active : null }
  } catch {
    return { open: [], active: null }
  }
}
