import { api } from '../api'
import type { IssueSummary, PullSummary } from '../types'
import { projects } from './projects.svelte'

const MAX_PAGES = 5 // 30 per page: the 150 most recently updated of each

const asText = (e: unknown) =>
  e && typeof e === 'object' && 'message' in e
    ? String((e as { message: unknown }).message)
    : String(e)

/** Every issue and pull request (open or not) of the repository, for the board and table. */
class IssueBoardData {
  issues = $state.raw<IssueSummary[]>([])
  pulls = $state.raw<PullSummary[]>([])
  loading = $state(false)
  loaded = $state(false)
  error = $state<string | null>(null)
  /** More exist on GitHub than were loaded. */
  truncated = $state(false)
  #gen = 0

  async load() {
    const path = projects.current?.path
    const gen = ++this.#gen
    if (!path) return
    this.loading = true
    this.error = null
    try {
      const [issues, pulls] = await Promise.all([
        this.#all((p) => api.forgeIssues(path, 'all', '', p)),
        // pull requests are only context: a failure there should not hide the issues
        this.#all((p) => api.forgePulls(path, 'all', '', p)).catch(() => ({
          items: [] as PullSummary[],
          more: false,
        })),
      ])
      if (gen !== this.#gen) return
      this.issues = issues.items
      this.pulls = pulls.items
      this.truncated = issues.more
      this.loaded = true
    } catch (e) {
      if (gen === this.#gen) this.error = asText(e)
    } finally {
      if (gen === this.#gen) this.loading = false
    }
  }

  async #all<T>(fetchPage: (page: number) => Promise<{ items: T[]; next_page: number | null }>) {
    const items: T[] = []
    let page: number | null = 1
    for (let i = 0; i < MAX_PAGES && page !== null; i++) {
      const r = await fetchPage(page)
      items.push(...r.items)
      page = r.next_page
    }
    return { items, more: page !== null }
  }

  reset() {
    this.#gen++
    this.issues = []
    this.pulls = []
    this.loaded = false
    this.loading = false
    this.error = null
    this.truncated = false
  }
}

export const board = new IssueBoardData()
