import { api } from '../api'
import type { Project, RepoSummary } from '../types'

/** Set by the tabs store so a removed project also loses its tab (avoids an import cycle). */
let removedHook: ((known: string[]) => void) | undefined
export function onProjectsChanged(fn: (known: string[]) => void) {
  removedHook = fn
}

class Projects {
  list = $state<Project[]>([])
  current = $state<Project | null>(null)
  summaries = $state<Record<string, RepoSummary>>({})
  error = $state<string | null>(null)

  async refresh() {
    this.list = await api.listProjects()
  }

  async loadSummaries() {
    await Promise.all(
      this.list.map(async (p) => {
        try {
          this.summaries[p.path] = await api.repoSummary(p.path)
        } catch {
          // folder moved/deleted: leave the row without branch info
        }
      }),
    )
  }

  async openPath(path: string): Promise<boolean> {
    try {
      this.current = await api.openProject(path)
      this.error = null
      await this.refresh()
      return true
    } catch (e) {
      this.error = String(e)
      return false
    }
  }

  async openPicker(): Promise<boolean> {
    const path = await api.pickFolder()
    return path ? this.openPath(path) : false
  }

  async toggleStar(p: Project) {
    await api.setStarred(p.path, !p.starred)
    await this.refresh()
  }

  async remove(p: Project) {
    await api.removeProject(p.path)
    if (this.current?.path === p.path) this.current = null
    await this.refresh()
    removedHook?.(this.list.map((x) => x.path))
  }
}

export const projects = new Projects()
