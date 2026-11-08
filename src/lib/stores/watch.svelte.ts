import { api } from '../api'
import { changes } from './changes.svelte'
import { projects } from './projects.svelte'
import { repo } from './repo.svelte'

interface Pending {
  worktree: boolean
  git: boolean
}

/** Keeps the UI in sync with changes made outside Whittle (editor, terminal, other tools). */
class Watch {
  error = $state<string | null>(null)
  #pending: Pending | null = null
  #running = false

  /** (Re)start watching the current project, or stop when none is open. */
  async follow(path: string | undefined) {
    this.error = null
    try {
      if (path) await api.watchRepo(path)
      else await api.unwatchRepo()
    } catch (e) {
      this.error = String(e)
    }
  }

  /** Called for every `repo-changed` event. Bursts are merged and handled one at a time. */
  onChanged(e: { path: string; worktree: boolean; git: boolean }) {
    if (e.path !== projects.current?.path) return
    const p = (this.#pending ??= { worktree: false, git: false })
    p.worktree ||= e.worktree
    p.git ||= e.git
    void this.#drain()
  }

  async #drain() {
    if (this.#running) return
    this.#running = true
    try {
      while (this.#pending) {
        const p = this.#pending
        this.#pending = null
        // git state changed: branches, commits, ahead/behind. Worktree-only: just the file lists.
        await Promise.all([p.git ? repo.refresh() : null, changes.refresh()])
      }
    } finally {
      this.#running = false
    }
  }
}

export const watch = new Watch()
