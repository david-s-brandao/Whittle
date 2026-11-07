import { api } from '../api'
import { changes } from './changes.svelte'
import { projects } from './projects.svelte'
import { repo } from './repo.svelte'
import { ui } from './ui.svelte'

/** Checkout / create / delete branches. Results are reported through `ui.notice`. */
class Branches {
  busy = $state(false)

  async #run(
    okText: string | null,
    op: (path: string) => Promise<string | void>,
  ): Promise<boolean> {
    const path = projects.current?.path
    if (!path || this.busy) return false
    this.busy = true
    ui.notice = null
    try {
      const out = await op(path)
      ui.notice = { kind: 'ok', text: (typeof out === 'string' && out) || okText || 'Done.' }
      return true
    } catch (e) {
      ui.notice = { kind: 'error', text: String(e) }
      return false
    } finally {
      this.busy = false
      await Promise.all([repo.refresh(), changes.refresh()])
    }
  }

  checkout(name: string, remote = false) {
    return this.#run(`Switched to ${name}.`, (p) => api.checkoutBranch(p, name, remote))
  }

  create(name: string, switchTo: boolean) {
    return this.#run(
      switchTo ? `Created and switched to ${name}.` : `Created branch ${name}.`,
      (p) => api.createBranch(p, name, null, switchTo),
    )
  }

  async remove(name: string) {
    const ok = await api.confirm(
      `Delete branch “${name}”?\n\nOnly the branch label is removed; git will refuse if it has commits that exist nowhere else.`,
      'Delete branch',
      'Delete',
    )
    if (!ok) return
    const path = projects.current?.path
    if (!path) return
    this.busy = true
    ui.notice = null
    try {
      const msg = await api.deleteBranch(path, name, false)
      ui.notice = { kind: 'ok', text: msg || `Deleted ${name}.` }
    } catch (e) {
      const text = String(e)
      if (text.includes('not fully merged')) {
        const force = await api.confirm(
          `“${name}” has commits that are not merged into any other branch.\n\nDeleting it will leave those commits unreachable (recoverable for a while via the reflog, using the hash git prints).`,
          'Delete unmerged branch',
          'Delete anyway',
        )
        if (force) {
          try {
            const msg = await api.deleteBranch(path, name, true)
            ui.notice = { kind: 'ok', text: msg || `Deleted ${name}.` }
          } catch (e2) {
            ui.notice = { kind: 'error', text: String(e2) }
          }
        }
      } else {
        ui.notice = { kind: 'error', text }
      }
    } finally {
      this.busy = false
      await repo.refresh()
    }
  }
}

export const branches = new Branches()
