import { cycle, parseSaved, restore, withoutTab, withTab } from '../tabs'
import { onProjectsChanged, projects } from './projects.svelte'
import { ui } from './ui.svelte'

const KEY = 'whittle.tabs'

/** The open repositories shown as tabs. Switching a tab opens that project in the main view. */
class Tabs {
  paths = $state<string[]>([])
  #savedActive: string | null = null

  constructor() {
    try {
      const s = parseSaved(localStorage.getItem(KEY))
      this.paths = s.open
      this.#savedActive = s.active
    } catch {
      // no saved session
    }
  }

  #persist() {
    try {
      localStorage.setItem(
        KEY,
        JSON.stringify({ open: this.paths, active: projects.current?.path ?? null }),
      )
    } catch {
      // the session just isn't remembered
    }
  }

  /** After the project list is known: drop tabs for projects that no longer exist, reopen the last one. */
  async restoreSession(known: string[]) {
    const r = restore({ open: this.paths, active: this.#savedActive }, known)
    this.paths = r.open
    if (r.active && !projects.current) await projects.openPath(r.active)
    this.#persist()
  }

  /** The current project always has a tab. */
  track(path: string | undefined) {
    if (path) this.paths = withTab(this.paths, path)
    this.#persist()
  }

  async activate(path: string) {
    if (path === projects.current?.path) return
    const ok = await projects.openPath(path)
    if (!ok) {
      // folder moved or deleted: its tab can't work any more
      this.paths = withoutTab(this.paths, path).tabs
      ui.notice = { kind: 'error', text: projects.error ?? 'That repository could not be opened.' }
      this.#persist()
    }
  }

  async close(path: string) {
    const { tabs, next } = withoutTab(this.paths, path)
    this.paths = tabs
    if (path === projects.current?.path) {
      if (next) await this.activate(next)
      else projects.current = null
    }
    this.#persist()
  }

  async cycle(dir: 1 | -1) {
    const next = cycle(this.paths, projects.current?.path ?? null, dir)
    if (next) await this.activate(next)
  }

  /** Forget tabs of projects that were removed from the list. */
  prune(known: string[]) {
    const kept = this.paths.filter((p) => known.includes(p))
    if (kept.length !== this.paths.length) {
      this.paths = kept
      this.#persist()
    }
  }
}

export const tabs = new Tabs()
onProjectsChanged((known) => tabs.prune(known))
