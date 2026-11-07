import { api } from '../api'
import type { ProgressOp, SyncError, SyncOp } from '../types'
import { changes } from './changes.svelte'
import { prefs } from './prefs.svelte'
import { projects } from './projects.svelte'
import { repo } from './repo.svelte'

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

  /** Called from the Tauri `git-progress` event. */
  onProgress(op: ProgressOp, line: string) {
    if (this.running !== op) return
    this.line = line
    const m = line.match(/(\d{1,3})%/)
    this.percent = m ? Math.min(100, Number(m[1])) : this.percent
  }

  async run(op: SyncOp, force = false) {
    const path = projects.current?.path
    if (!path || this.running) return
    if (force) {
      const ok = await api.confirm(
        'Force push rewrites the remote branch. Commits on the server that you don’t have locally will be lost.\n\nKnife uses --force-with-lease, which refuses if the remote changed since your last fetch.',
        'Force push',
        'Force push',
      )
      if (!ok) return
    }
    this.running = op
    this.line = 'Starting…'
    this.percent = null
    this.error = null
    this.done = null
    try {
      const pull =
        op === 'pull' ? { mode: prefs.pullMode, autostash: prefs.pullAutostash } : undefined
      await api.gitSync(path, op, force, pull)
      this.done = { fetch: 'Fetched.', pull: 'Pulled.', push: 'Pushed.' }[op]
    } catch (e) {
      this.error = { ...asSyncError(e), op }
    } finally {
      this.running = null
      // reflect new commits / branches / ahead-behind everywhere
      await Promise.all([repo.refresh(), changes.refresh()])
    }
  }

  dismiss() {
    this.error = null
    this.done = null
  }
}

export const sync = new Sync()
