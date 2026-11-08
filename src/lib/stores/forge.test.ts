import { beforeEach, expect, it, vi } from 'vitest'
import type { ForgeComment, IssueDetail } from '../types'

const mocks = vi.hoisted(() => ({
  api: {
    forgeIssue: vi.fn(),
    forgeComment: vi.fn(),
    forgeSetState: vi.fn(),
    forgeIssues: vi.fn(),
    forgePulls: vi.fn(),
  },
  projects: { current: { path: '/a' } },
  ui: { notice: null },
}))
vi.mock('../api', () => ({ api: mocks.api }))
vi.mock('./projects.svelte', () => ({ projects: mocks.projects }))
vi.mock('./ui.svelte', () => ({ ui: mocks.ui }))
vi.mock('./changes.svelte', () => ({ changes: { refresh: vi.fn() } }))
vi.mock('./repo.svelte', () => ({ repo: { refresh: vi.fn() } }))

const issue = (number: number): IssueDetail => ({
  summary: {
    number,
    title: `Issue ${number}`,
    state: 'open',
    author: 'alice',
    comments: 0,
    labels: [],
    assignees: [],
    created_at: '',
    updated_at: '',
    url: `https://github.com/a/b/issues/${number}`,
  },
  body: 'Description',
  comments: [],
})
const comment: ForgeComment = {
  id: 1,
  author: 'bob',
  body: 'Reply',
  created_at: '',
  url: '',
  association: 'NONE',
}

beforeEach(() => {
  vi.resetModules()
  vi.resetAllMocks()
  mocks.api.forgeIssue.mockImplementation((_path, number) => Promise.resolve(issue(number)))
  mocks.api.forgeIssues.mockResolvedValue({ items: [], next_page: null })
  mocks.api.forgePulls.mockResolvedValue({ items: [], next_page: null })
  mocks.ui.notice = null
})

it('does not append a late submitted comment to a different thread', async () => {
  const { detail } = await import('./forge.svelte')
  await detail.openIssue(1)
  let finish!: (comment: ForgeComment) => void
  mocks.api.forgeComment.mockImplementation(
    () =>
      new Promise<ForgeComment>((resolve) => {
        finish = resolve
      }),
  )
  const pending = detail.comment('Reply')
  await detail.openIssue(2)
  finish(comment)
  expect(await pending).toBe(true)
  expect(detail.issue?.summary.number).toBe(2)
  expect(detail.issue?.comments).toEqual([])
})

it('updates the comment count after posting to the active issue', async () => {
  const { detail } = await import('./forge.svelte')
  await detail.openIssue(1)
  mocks.api.forgeComment.mockResolvedValue(comment)
  expect(await detail.comment('Reply')).toBe(true)
  expect(detail.issue?.summary.comments).toBe(1)
  expect(detail.issue?.comments).toEqual([comment])
})

it('keeps the thread visible on refresh failure', async () => {
  const { detail } = await import('./forge.svelte')
  await detail.openIssue(1)
  mocks.api.forgeIssue.mockRejectedValue(new Error('Offline'))
  await detail.refresh()
  expect(detail.issue?.summary.number).toBe(1)
  expect(detail.loading).toBe(false)
  expect(detail.refreshing).toBe(false)
  expect(detail.error).toBeNull()
  expect(mocks.ui.notice).toEqual({ kind: 'error', text: 'Offline' })
})

it('reloads the issue list, not the PR list, after changing an issue state', async () => {
  const { detail } = await import('./forge.svelte')
  await detail.openIssue(1)
  mocks.api.forgeSetState.mockResolvedValue(undefined)
  await detail.setState('closed')
  expect(mocks.api.forgeIssues).toHaveBeenCalledOnce()
  expect(mocks.api.forgePulls).not.toHaveBeenCalled()
})
