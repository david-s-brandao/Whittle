import { api } from '../api'
import type { CommitInfo, OpResult, SyncError } from '../types'
import { changes } from './changes.svelte'
import { projects } from './projects.svelte'
import { repo } from './repo.svelte'
import { ui } from './ui.svelte'

/** Actions on a single commit or tag: cherry-pick, revert, reset, tags, detached checkout. */
class Commits {
  busy = $state(false)

  async #run(run: (path: string) => Promise<string | OpResult | void>): Promise<boolean> {
    const path = projects.current?.path
    if (!path || this.busy) return false
    this.busy = true
    ui.notice = null
    try {
      const r = await run(path)
      if (typeof r === 'string') ui.notice = { kind: 'ok', text: r }
      else if (r && typeof r === 'object') {
        ui.notice = { kind: r.outcome === 'conflicts' ? 'error' : 'ok', text: r.message }
        if (r.outcome === 'conflicts') ui.view = 'changes'
      }
      return true
    } catch (e) {
      const text =
        e && typeof e === 'object' && 'message' in e ? (e as SyncError).message : String(e)
      ui.notice = { kind: 'error', text }
      return false
    } finally {
      this.busy = false
      await Promise.all([repo.refresh(), changes.refresh()])
    }
  }

  cherryPick(c: CommitInfo) {
    return this.#run((p) => api.cherryPick(p, c.id))
  }

  async revert(c: CommitInfo) {
    const ok = await api.confirm(
      `Revert “${c.summary}”?\n\nThis adds a new commit that undoes its changes. The original commit stays in the history.`,
      'Revert commit',
      'Revert',
    )
    if (ok) await this.#run((p) => api.revertCommit(p, c.id))
  }

  reset(c: CommitInfo, mode: 'soft' | 'mixed' | 'hard') {
    return this.#run((p) => api.resetTo(p, c.id, mode))
  }

  checkoutDetached(c: CommitInfo) {
    return this.#run(async (p) => {
      await api.checkoutCommit(p, c.id)
      return `Checked out ${c.short_id}. You're on a commit, not a branch: create a branch to keep new work.`
    })
  }

  createBranchAt(c: CommitInfo, name: string, switchTo: boolean) {
    return this.#run(async (p) => {
      await api.createBranch(p, name, c.id, switchTo)
      return switchTo ? `Created and switched to ${name}.` : `Created branch ${name}.`
    })
  }

  async createTag(c: CommitInfo, name: string, message: string, push: boolean) {
    const ok = await this.#run(async (p) => {
      await api.createTag(p, name, c.id, message.trim() || null)
      return `Tag ${name} created.`
    })
    if (ok && push) await this.pushTag(name, false)
  }

  pushTag(name: string, remoteDelete: boolean) {
    return this.#run(async (p) => {
      await api.pushTag(p, name, remoteDelete)
      return remoteDelete ? `Tag ${name} deleted on the remote.` : `Tag ${name} pushed.`
    })
  }

  async deleteTag(name: string) {
    const ok = await api.confirm(
      `Delete the tag “${name}” locally?\n\nIt is not removed from the remote.`,
      'Delete tag',
      'Delete',
    )
    if (ok) await this.#run((p) => api.deleteTag(p, name))
  }

  async deleteRemoteTag(name: string) {
    const ok = await api.confirm(
      `Delete the tag “${name}” on the remote?\n\nEveryone who fetches will lose it. Your local copy stays.`,
      'Delete remote tag',
      'Delete on remote',
    )
    if (ok) await this.pushTag(name, true)
  }
}

export const commits = new Commits()
