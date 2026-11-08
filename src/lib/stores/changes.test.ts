import { beforeEach, expect, it, vi } from 'vitest'
import type { WorkStatus } from '../types'

const mocks = vi.hoisted(() => ({
  projects: { current: { path: '/a' } },
  api: {
    workStatus: vi.fn(),
    workFileDiff: vi.fn(),
    readFileText: vi.fn(),
    saveResolvedFile: vi.fn(),
    headMessage: vi.fn(),
    commitChanges: vi.fn(),
  },
}))
vi.mock('../api', () => ({ api: mocks.api }))
vi.mock('./projects.svelte', () => ({ projects: mocks.projects }))
const empty: WorkStatus = { staged: [], unstaged: [], conflicted: [], operation: null }
beforeEach(() => {
  vi.resetModules()
  vi.resetAllMocks()
  mocks.projects.current = { path: '/a' }
  mocks.api.workStatus.mockResolvedValue(empty)
  mocks.api.workFileDiff.mockResolvedValue({ binary: false, hunks: [] })
  mocks.api.readFileText.mockResolvedValue('<<<<<<< HEAD\na\n=======\nb\n>>>>>>> incoming\n')
})

it('keeps separate commit drafts and amend state while switching projects', async () => {
  const { changes } = await import('./changes.svelte')
  await changes.refresh()
  changes.message = 'feat: project A'
  changes.amend = true
  mocks.projects.current = { path: '/b' }
  await changes.refresh()
  expect(changes.message).toBe('')
  expect(changes.amend).toBe(false)
  changes.message = 'fix: project B'
  mocks.projects.current = { path: '/a' }
  await changes.refresh()
  expect(changes.message).toBe('feat: project A')
  expect(changes.amend).toBe(true)
})

it('does not overwrite another project draft with a late amend response', async () => {
  const { changes } = await import('./changes.svelte')
  await changes.refresh()
  let finish!: (value: string) => void
  mocks.api.headMessage.mockImplementation(
    () =>
      new Promise<string>((resolve) => {
        finish = resolve
      }),
  )
  const pending = changes.setAmend(true)
  mocks.projects.current = { path: '/b' }
  await changes.refresh()
  changes.message = 'project B'
  finish('old project A commit')
  await pending
  expect(changes.message).toBe('project B')
})

it('opens the next conflict only after the current file is saved successfully', async () => {
  const { changes } = await import('./changes.svelte')
  mocks.api.workStatus.mockResolvedValue({ ...empty, conflicted: ['a', 'b'] })
  await changes.refresh()
  await changes.selectConflict('a')
  mocks.api.saveResolvedFile.mockRejectedValueOnce(new Error('Write failed'))
  expect(await changes.saveResolved('a', 'result', true)).toBe(false)
  expect(changes.selection?.path).toBe('a')
  mocks.api.saveResolvedFile.mockResolvedValue(undefined)
  mocks.api.workStatus.mockResolvedValue({
    ...empty,
    staged: [{ path: 'a', old_path: null, status: 'modified' }],
    conflicted: ['b'],
  })
  expect(await changes.saveResolved('a', 'result', true)).toBe(true)
  expect(changes.selection).toEqual({ path: 'b', staged: false, conflict: true })
})

it('blocks commits, including amend, while conflicts remain', async () => {
  const { changes } = await import('./changes.svelte')
  changes.status = { ...empty, conflicted: ['a'] }
  changes.amend = true
  expect(changes.canCommit).toBe(false)
  await changes.commit()
  expect(mocks.api.commitChanges).not.toHaveBeenCalled()
})
