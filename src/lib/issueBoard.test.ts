import { describe, expect, it } from 'vitest'
import { branchRefs, buildBoard, refsIn } from './issueBoard'
import type { CommitInfo, IssueSummary, PullSummary } from './types'

const issue = (number: number, state: 'open' | 'closed' = 'open'): IssueSummary => ({
  number,
  title: `i${number}`,
  state,
  author: 'a',
  labels: [],
  assignees: [],
  comments: 0,
  created_at: '',
  updated_at: '',
  url: '',
})
const pr = (number: number, o: Partial<PullSummary> = {}): PullSummary => ({
  number,
  title: `p${number}`,
  state: 'open',
  draft: false,
  author: 'a',
  head_ref: 'x',
  base_ref: 'main',
  labels: [],
  body: '',
  created_at: '',
  updated_at: '',
  url: '',
  ...o,
})
const commit = (summary: string): CommitInfo => ({
  id: summary,
  short_id: 'abc',
  summary,
  author: 'a',
  email: '',
  time: 0,
  parents: [],
  signed: false,
})
const col = (items: ReturnType<typeof buildBoard>, n: number) =>
  items.find((i) => i.issue.number === n)!.column

describe('references', () => {
  it('finds #n in text but not in urls', () => {
    expect(refsIn('Fixes #12 and #7, see org/repo#3 and http://x/#9')).toEqual([12, 7])
  })
  it('finds standalone numbers in branch names only', () => {
    expect(branchRefs('fix/42-crash')).toEqual([42])
    expect(branchRefs('issue-7')).toEqual([7])
    expect(branchRefs('v2-fix')).toEqual([])
  })
})

describe('buildBoard', () => {
  it('puts untouched issues in Open and closed ones in Closed', () => {
    const b = buildBoard([issue(1), issue(2, 'closed')], [], [], [])
    expect([col(b, 1), col(b, 2)]).toEqual(['open', 'closed'])
  })
  it('moves issues with commits, branches or draft PRs to In progress', () => {
    const b = buildBoard(
      [issue(1), issue(2), issue(3)],
      [pr(9, { draft: true, body: 'Closes #3' })],
      [commit('fix crash (#1)')],
      ['feature/2-thing', 'feature/2-thing'],
    )
    expect([col(b, 1), col(b, 2), col(b, 3)]).toEqual(['progress', 'progress', 'progress'])
    expect(b[1].links.branches).toEqual(['feature/2-thing'])
  })
  it('moves issues with an open PR to In review', () => {
    const b = buildBoard([issue(5)], [pr(8, { body: 'fixes #5' })], [], [])
    expect(col(b, 5)).toBe('review')
  })
  it('keeps a merged PR as a link, not as review', () => {
    const b = buildBoard([issue(5)], [pr(8, { state: 'merged', title: 'x (#5)' })], [], [])
    expect(col(b, 5)).toBe('progress')
  })
})
