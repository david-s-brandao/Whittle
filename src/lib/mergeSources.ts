import type { BranchInfo } from './types'

export interface SourceSuggestion {
  name: string
  /** Why it is suggested, shown as a tooltip. */
  reason: string
}

const DEFAULTS = ['main', 'master', 'develop', 'dev']

/**
 * Likely things to merge into the current branch, best first:
 * its tracked remote branch, then the remote copies of the default branches
 * (e.g. `origin/main`: "get the latest main"), then the local default branches.
 * Never includes the current branch itself.
 */
export function suggestSources(
  local: BranchInfo[],
  remote: BranchInfo[],
  current: string | null,
): SourceSuggestion[] {
  const out: SourceSuggestion[] = []
  const seen = new Set<string>()
  const add = (name: string, reason: string) => {
    if (name === current || seen.has(name)) return
    seen.add(name)
    out.push({ name, reason })
  }

  const cur = local.find((b) => b.name === current)
  if (cur?.upstream) add(cur.upstream, 'The remote branch your current branch tracks')

  for (const d of DEFAULTS) {
    for (const r of remote) {
      // "origin/main" and also nested remotes like "upstream/main"
      if (r.name.endsWith('/' + d)) add(r.name, `Latest ${d} on the remote (as of your last fetch)`)
    }
  }
  for (const d of DEFAULTS) {
    if (local.some((b) => b.name === d)) add(d, `Your local ${d} branch`)
  }
  return out
}
