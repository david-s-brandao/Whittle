import { api } from '../api'
import type { MergeOptions, OpResult, RebaseItem } from '../types'
import { changes } from './changes.svelte'
import { projects } from './projects.svelte'
import { repo } from './repo.svelte'
import { ui } from './ui.svelte'

/** Merge, rebase and the continue / skip / abort steps of a paused operation. */
class Operations {
  busy = $state(false)

  async #run(run: (path: string) => Promise<OpResult>): Promise<void> {
    const path = projects.current?.path
    if (!path || this.busy) return
    this.busy = true
    ui.notice = null
    try {
      const r = await run(path)
      ui.notice = { kind: r.outcome === 'conflicts' ? 'error' : 'ok', text: r.message }
      if (r.outcome === 'conflicts') ui.view = 'changes' // that's where conflicts get resolved
    } catch (e) {
      ui.notice = { kind: 'error', text: String(e) }
    } finally {
      this.busy = false
      await Promise.all([repo.refresh(), changes.refresh()])
    }
  }

  merge(branch: string, options: MergeOptions) {
    return this.#run((p) => api.mergeBranch(p, branch, options))
  }

  /** Open the merge/rebase dialog; `branch` preselects the source (null = let the user pick). */
  openDialog(branch: string | null, mode: 'merge' | 'rebase' = 'merge') {
    ui.mergeDialog = { mode, branch }
  }

  /** `items` oldest first, as git applies them. */
  interactive(from: string, items: RebaseItem[]) {
    return this.#run((p) => api.rebaseInteractive(p, from, items))
  }

  rebase(onto: string) {
    return this.#run((p) => api.rebaseOnto(p, onto))
  }

  continue() {
    return this.#run((p) => api.operationStep(p, 'continue'))
  }

  skip() {
    return this.#run((p) => api.operationStep(p, 'skip'))
  }

  async abort() {
    const op = changes.status.operation
    const ok = await api.confirm(
      `Abort the ${op}?\n\nEverything goes back to how it was before it started. Conflict resolutions you made so far are discarded.`,
      `Abort ${op}`,
      'Abort',
    )
    if (ok) await this.#run((p) => api.operationStep(p, 'abort'))
  }
}

export const operations = new Operations()
