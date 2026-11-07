import { api } from '../api'
import type { StashInfo } from '../types'
import { changes } from './changes.svelte'
import { projects } from './projects.svelte'
import { repo } from './repo.svelte'
import { ui } from './ui.svelte'

/** Create / apply / pop / drop stashes. Results are reported through `ui.notice`. */
class Stashes {
  busy = $state(false)

  async #run(op: (path: string) => Promise<string>): Promise<boolean> {
    const path = projects.current?.path
    if (!path || this.busy) return false
    this.busy = true
    ui.notice = null
    try {
      ui.notice = { kind: 'ok', text: await op(path) }
      return true
    } catch (e) {
      ui.notice = { kind: 'error', text: String(e) }
      return false
    } finally {
      this.busy = false
      await Promise.all([repo.refresh(), changes.refresh()])
    }
  }

  create(message: string, includeUntracked: boolean) {
    return this.#run(async (p) => {
      await api.stashPush(p, message, includeUntracked)
      return 'Changes stashed. Your working tree is clean again.'
    })
  }

  apply(s: StashInfo, pop: boolean) {
    return this.#run((p) => api.stashApply(p, s.index, pop))
  }

  async drop(s: StashInfo) {
    const ok = await api.confirm(
      `Delete this stash?\n\n“${s.message}”\n\nThe stashed changes are removed for good.`,
      'Delete stash',
      'Delete',
    )
    if (ok) await this.#run((p) => api.stashDrop(p, s.index))
  }

  /** Show what a stash contains in the History tab's details panel. */
  preview(s: StashInfo) {
    ui.view = 'history'
    return repo.selectCommit({
      id: s.id,
      short_id: s.id.slice(0, 7),
      summary: s.message,
      author: 'Stash',
      email: '',
      time: s.time,
      parents: [],
      signed: false,
    })
  }
}

export const stashes = new Stashes()
