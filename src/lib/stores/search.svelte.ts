import { api } from '../api'
import type { CommitInfo } from '../types'
import { projects } from './projects.svelte'

/** Commit search (message, author, email, hash). Debounced; stale answers are dropped. */
class Search {
  query = $state('')
  results = $state.raw<CommitInfo[]>([])
  loading = $state(false)
  error = $state<string | null>(null)

  #gen = 0
  #timer: ReturnType<typeof setTimeout> | undefined

  get active(): boolean {
    return this.query.trim().length > 0
  }

  set(query: string) {
    this.query = query
    clearTimeout(this.#timer)
    const gen = ++this.#gen
    if (!query.trim()) {
      this.results = []
      this.loading = false
      this.error = null
      return
    }
    this.loading = true
    this.#timer = setTimeout(() => void this.#run(query.trim(), gen), 250)
  }

  async #run(query: string, gen: number) {
    const path = projects.current?.path
    if (!path) return
    try {
      const found = await api.searchCommits(path, query)
      if (gen !== this.#gen) return
      this.results = found
      this.error = null
    } catch (e) {
      if (gen === this.#gen) this.error = String(e)
    } finally {
      if (gen === this.#gen) this.loading = false
    }
  }

  clear() {
    this.set('')
  }
}

export const search = new Search()
