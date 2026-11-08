import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const mocks = vi.hoisted(() => ({
  api: { gitSync: vi.fn(), confirm: vi.fn() },
  projects: { current: { path: '/a' } },
  changes: { busy: false, status: { operation: null as string | null }, refresh: vi.fn() },
  repo: { refresh: vi.fn() },
  settings: { loaded: true, value: { auto_fetch_minutes: 5 } },
}))
vi.mock('../api', () => ({ api: mocks.api }))
vi.mock('./projects.svelte', () => ({ projects: mocks.projects }))
vi.mock('./changes.svelte', () => ({ changes: mocks.changes }))
vi.mock('./repo.svelte', () => ({ repo: mocks.repo }))
vi.mock('./settings.svelte', () => ({ settings: mocks.settings }))
vi.mock('./branches.svelte', () => ({ branches: { busy: false } }))
vi.mock('./operations.svelte', () => ({ operations: { busy: false } }))
vi.mock('./stashes.svelte', () => ({ stashes: { busy: false } }))
vi.mock('./prefs.svelte', () => ({ prefs: { pullMode: 'default', pullAutostash: false } }))

beforeEach(() => {
  vi.resetModules()
  vi.clearAllMocks()
  vi.useFakeTimers()
  vi.setSystemTime(0)
  vi.stubGlobal('navigator', { onLine: true })
  vi.stubGlobal('window', { addEventListener: vi.fn(), removeEventListener: vi.fn() })
  mocks.projects.current = { path: '/a' }
  mocks.changes.busy = false
  mocks.changes.status.operation = null
  mocks.settings.value.auto_fetch_minutes = 5
  mocks.api.gitSync.mockResolvedValue(undefined)
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('background fetch integration', () => {
  it('fetches on schedule without toast spam and cleans up the timer', async () => {
    const { sync } = await import('./sync.svelte')
    const stop = sync.startAutoFetch()
    await vi.advanceTimersByTimeAsync(300_000)
    expect(mocks.api.gitSync).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(15_000)
    expect(mocks.api.gitSync).toHaveBeenCalledExactlyOnceWith('/a', 'fetch', false, undefined)
    expect(sync.done).toBeNull()
    expect(sync.fetches['/a'].at).toBe(315_000)
    expect(mocks.repo.refresh).toHaveBeenCalledOnce()
    stop()
    await vi.advanceTimersByTimeAsync(600_000)
    expect(mocks.api.gitSync).toHaveBeenCalledTimes(1)
  })

  it('waits for busy operations, pauses, offline state and disabled settings', async () => {
    const { sync } = await import('./sync.svelte')
    const stop = sync.startAutoFetch()
    await vi.advanceTimersByTimeAsync(15_000)
    mocks.changes.busy = true
    await vi.advanceTimersByTimeAsync(300_000)
    mocks.changes.busy = false
    sync.paused = true
    await vi.advanceTimersByTimeAsync(15_000)
    sync.paused = false
    vi.stubGlobal('navigator', { onLine: false })
    await vi.advanceTimersByTimeAsync(15_000)
    vi.stubGlobal('navigator', { onLine: true })
    mocks.settings.value.auto_fetch_minutes = 0
    await vi.advanceTimersByTimeAsync(15_000)
    expect(mocks.api.gitSync).not.toHaveBeenCalled()
    mocks.settings.value.auto_fetch_minutes = 5
    await vi.advanceTimersByTimeAsync(15_000)
    expect(mocks.api.gitSync).toHaveBeenCalledOnce()
    stop()
  })

  it('does not overlap requests or refresh the wrong repository after switching tabs', async () => {
    const { sync } = await import('./sync.svelte')
    let finish!: () => void
    mocks.api.gitSync.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve
        }),
    )
    const pending = sync.run('fetch', false, true)
    await sync.run('fetch')
    expect(mocks.api.gitSync).toHaveBeenCalledOnce()
    mocks.projects.current = { path: '/b' }
    finish()
    await pending
    expect(sync.fetches['/a'].at).toBe(0)
    expect(sync.fetches['/b']).toBeUndefined()
    expect(mocks.repo.refresh).not.toHaveBeenCalled()
  })

  it('records background errors quietly and applies backoff', async () => {
    const { sync } = await import('./sync.svelte')
    mocks.api.gitSync.mockRejectedValue({ kind: 'network', message: 'Offline', details: '' })
    await sync.run('fetch', false, true)
    expect(sync.error).toBeNull()
    expect(sync.fetches['/a'].error).toBe('Offline')
    const stop = sync.startAutoFetch()
    await vi.advanceTimersByTimeAsync(599_999)
    expect(mocks.api.gitSync).toHaveBeenCalledTimes(1)
    await vi.advanceTimersByTimeAsync(1)
    expect(mocks.api.gitSync).toHaveBeenCalledTimes(2)
    stop()
  })
})
