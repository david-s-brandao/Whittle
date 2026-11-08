import { api } from '../api'
import { FetchSchedule } from '../fetchSchedule'
import type { ProgressOp, SyncError, SyncOp } from '../types'
import { changes } from './changes.svelte'
import { prefs } from './prefs.svelte'
import { projects } from './projects.svelte'
import { repo } from './repo.svelte'
import { settings } from './settings.svelte'
import { branches } from './branches.svelte'
import { operations } from './operations.svelte'
import { stashes } from './stashes.svelte'

export const opLabel: Record<SyncOp, string> = {
  fetch: 'Fetching',
  pull: 'Pulling',
  push: 'Pushing',
}

function asSyncError(e: unknown): SyncError {
  if (e && typeof e === 'object' && 'kind' in e && 'message' in e) return e as SyncError
  return { kind: 'other', message: String(e), details: '' }
}

/** Fetch / pull / push with live progress. One operation at a time. */
class Sync {
  running = $state<SyncOp | null>(null)
  line = $state('')
  percent = $state<number | null>(null)
  error = $state<(SyncError & { op: SyncOp }) | null>(null)
  done = $state<string | null>(null)
  background = $state(false)
  activePath = $state<string | null>(null)
  paused = $state(false)
  fetches = $state<Record<string, { at: number | null; error: string | null }>>({})
  #schedule = new FetchSchedule()

  /** One timer for the application lifetime; switches repositories without resetting their clocks. */
  startAutoFetch() {
    const tick = () => {
      const path = projects.current?.path
      if (!path || !settings.loaded || this.paused || !navigator.onLine) return
      if (
        this.running ||
        changes.busy ||
        branches.busy ||
        operations.busy ||
        stashes.busy ||
        changes.status.operation
      )
        return
      if (this.#schedule.due(path, settings.value.auto_fetch_minutes, Date.now()))
        void this.run('fetch', false, true)
    }
    const timer = setInterval(tick, 15_000)
    window.addEventListener('online', tick)
    return () => {
      clearInterval(timer)
      window.removeEventListener('online', tick)
    }
  }

  /** Called from the Tauri `git-progress` event. */
  onProgress(op: ProgressOp, line: string) {
    if (this.running !== op) return
    this.line = line
    const m = line.match(/(\d{1,3})%/)
    this.percent = m ? Math.min(100, Number(m[1])) : this.percent
  }

  async run(op: SyncOp, force = false, background = false) {
    const path = projects.current?.path
    if (!path || this.running) return
    if (force) {
      const ok = await api.confirm(
        'Force push rewrites the remote branch. Commits on the server that you don’t have locally will be lost.\n\nWhittle uses --force-with-lease, which refuses if the remote changed since your last fetch.',
        'Force push',
        'Force push',
      )
      if (!ok) return
      if (this.running || projects.current?.path !== path) return
    }
    this.running = op
    this.activePath = path
    this.background = background
    this.line = 'Starting…'
    this.percent = null
    if (!background) {
      this.error = null
      this.done = null
    }
    try {
      const pull =
        op === 'pull' ? { mode: prefs.pullMode, autostash: prefs.pullAutostash } : undefined
      await api.gitSync(path, op, force, pull)
      if (!background) this.done = { fetch: 'Fetched.', pull: 'Pulled.', push: 'Pushed.' }[op]
      if (op === 'fetch' || op === 'pull') {
        this.#schedule.finished(path, true, Date.now())
        this.fetches[path] = { at: Date.now(), error: null }
      }
    } catch (e) {
      const error = asSyncError(e)
      if (!background) this.error = { ...error, op }
      if (op === 'fetch') {
        this.#schedule.finished(path, false, Date.now())
        this.fetches[path] = { at: this.fetches[path]?.at ?? null, error: error.message }
      }
    } finally {
      this.running = null
      this.activePath = null
      this.background = false
      // reflect new commits / branches / ahead-behind everywhere
      if (projects.current?.path === path) await Promise.all([repo.refresh(), changes.refresh()])
    }
  }

  dismiss() {
    this.error = null
    this.done = null
  }
}

export const sync = new Sync()
