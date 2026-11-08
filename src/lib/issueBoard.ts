import type { CommitInfo, IssueSummary, PullSummary } from './types'

export type Column = 'open' | 'progress' | 'review' | 'closed'

export const columns: { id: Column; label: string; hint: string }[] = [
  {
    id: 'open',
    label: 'Open',
    hint: 'Nobody has started: no branch, commit or pull request mentions it',
  },
  {
    id: 'progress',
    label: 'In progress',
    hint: 'A branch, commit or draft pull request mentions it',
  },
  { id: 'review', label: 'In review', hint: 'An open pull request mentions it' },
  { id: 'closed', label: 'Closed', hint: 'Closed on GitHub' },
]

export interface IssueLinks {
  prs: PullSummary[]
  commits: CommitInfo[]
  branches: string[]
}

export interface BoardItem {
  issue: IssueSummary
  column: Column
  links: IssueLinks
}

/** Issue numbers mentioned as `#12` in free text. */
export function refsIn(text: string): number[] {
  const out = new Set<number>()
  for (const m of text.matchAll(/(?<![\w/])#(\d{1,7})\b/g)) out.add(Number(m[1]))
  return [...out]
}

/** Numbers that stand alone in a branch name: `fix/42-crash`, `issue-42`, `gh_42`, not `v2`. */
export function branchRefs(name: string): number[] {
  const out = new Set<number>()
  for (const m of name.matchAll(/(?<![A-Za-z0-9])(\d{1,6})(?![A-Za-z0-9])/g)) out.add(Number(m[1]))
  return [...out]
}

/** Works out each issue's column and what in the repository relates to it. */
export function buildBoard(
  issues: IssueSummary[],
  pulls: PullSummary[],
  commits: CommitInfo[],
  /** Local branch names, and remote ones without the remote prefix. */
  branchNames: string[],
): BoardItem[] {
  const known = new Set(issues.map((i) => i.number))
  const links = new Map<number, IssueLinks>()
  const of = (n: number) => {
    let l = links.get(n)
    if (!l) links.set(n, (l = { prs: [], commits: [], branches: [] }))
    return l
  }
  for (const pr of pulls) {
    const nums = new Set([...refsIn(`${pr.title}\n${pr.body ?? ''}`), ...branchRefs(pr.head_ref)])
    for (const n of nums) if (known.has(n) && n !== pr.number) of(n).prs.push(pr)
  }
  for (const c of commits)
    for (const n of refsIn(c.summary)) if (known.has(n)) of(n).commits.push(c)
  const seen = new Set<string>()
  for (const name of branchNames) {
    if (seen.has(name)) continue
    seen.add(name)
    for (const n of branchRefs(name)) if (known.has(n)) of(n).branches.push(name)
  }
  return issues.map((issue) => {
    const l = links.get(issue.number) ?? { prs: [], commits: [], branches: [] }
    let column: Column = 'open'
    if (issue.state === 'closed') column = 'closed'
    else if (l.prs.some((p) => p.state === 'open' && !p.draft)) column = 'review'
    else if (l.prs.length || l.commits.length || l.branches.length) column = 'progress'
    return { issue, column, links: l }
  })
}
