import { api } from '../api'
import type { SyncError } from '../types'
import { changes } from './changes.svelte'
import { projects } from './projects.svelte'
import { repo } from './repo.svelte'
import { ui } from './ui.svelte'

/** Initialise / update submodules. */
class Submodules {
  /** Path being updated, 'all' for everything, or null. */
  busy = $state<string | null>(null)

  async update(path: string | null) {
    const root = projects.current?.path
    if (!root || this.busy) return
    this.busy = path ?? 'all'
    ui.notice = null
    try {
      await api.updateSubmodules(root, path)
      ui.notice = { kind: 'ok', text: path ? `Updated ${path}.` : 'All submodules are up to date.' }
    } catch (e) {
      const text =
        e && typeof e === 'object' && 'message' in e ? (e as SyncError).message : String(e)
      ui.notice = { kind: 'error', text }
    } finally {
      this.busy = null
      await Promise.all([repo.refresh(), changes.refresh()])
    }
  }
}

export const submodules = new Submodules()
