import { api } from '../api'
import type {
  ForgeComment,
  ForgePage,
  ForgeStatus,
  IssueDetail,
  IssueSummary,
  PullDetail,
  PullFile,
  PullSummary,
  ReviewEvent,
} from '../types'
import { changes } from './changes.svelte'
import { projects } from './projects.svelte'
import { repo } from './repo.svelte'
import { ui } from './ui.svelte'

const asText = (e: unknown) =>
  e && typeof e === 'object' && 'message' in e
    ? String((e as { message: unknown }).message)
    : String(e)

/** Connection to the hosting service (GitHub) for the open repository. */
class Forge {
  status = $state<ForgeStatus | null>(null)
  loading = $state(false)
  error = $state<string | null>(null)
  #gen = 0

  async load() {
    const path = projects.current?.path
    const gen = ++this.#gen
    if (!path) {
      this.status = null
      return
    }
    this.loading = true
    this.error = null
    try {
      const s = await api.forgeStatus(path)
      if (gen === this.#gen) this.status = s
    } catch (e) {
      if (gen === this.#gen) this.error = asText(e)
    } finally {
      if (gen === this.#gen) this.loading = false
    }
  }

  /** Validate + save a pasted token (in the system keyring; the token is not kept here). */
  async connect(token: string) {
    const path = projects.current?.path
    if (!path) return
    this.error = null
    try {
      this.status = await api.forgeConnect(path, token)
    } catch (e) {
      this.error = asText(e)
      throw e
    }
  }

  async disconnect() {
    const path = projects.current?.path
    if (!path) return
    try {
      this.status = await api.forgeDisconnect(path)
    } catch (e) {
      this.error = asText(e)
    }
  }

  reset() {
    this.#gen++
    this.status = null
    this.error = null
  }

  get ready(): boolean {
    return !!this.status?.supported
  }
}

export const forge = new Forge()

type Filter = 'open' | 'closed' | 'all'

/** A paged, filterable, searchable list of issues or pull requests. */
class ForgeList<T extends { number: number }> {
  state = $state<Filter>('open')
  query = $state('')
  items = $state.raw<T[]>([])
  nextPage = $state<number | null>(null)
  loading = $state(false)
  error = $state<string | null>(null)
  loaded = $state(false)
  selected = $state<number | null>(null)

  #gen = 0
  #timer: ReturnType<typeof setTimeout> | undefined
  #fetch: (path: string, state: Filter, query: string, page: number) => Promise<ForgePage<T>>

  constructor(
    fetch: (path: string, state: Filter, query: string, page: number) => Promise<ForgePage<T>>,
  ) {
    this.#fetch = fetch
  }

  async reload() {
    const path = projects.current?.path
    const gen = ++this.#gen
    if (!path) return
    this.loading = true
    this.error = null
    try {
      const page = await this.#fetch(path, this.state, this.query.trim(), 1)
      if (gen !== this.#gen) return
      this.items = page.items
      this.nextPage = page.next_page
      this.loaded = true
    } catch (e) {
      if (gen === this.#gen) this.error = asText(e)
    } finally {
      if (gen === this.#gen) this.loading = false
    }
  }

  async more() {
    const path = projects.current?.path
    if (!path || this.loading || this.nextPage === null) return
    const gen = this.#gen
    this.loading = true
    try {
      const page = await this.#fetch(path, this.state, this.query.trim(), this.nextPage)
      if (gen !== this.#gen) return
      const seen = new Set(this.items.map((i) => i.number))
      this.items = this.items.concat(page.items.filter((i) => !seen.has(i.number)))
      this.nextPage = page.next_page
    } catch (e) {
      if (gen === this.#gen) this.error = asText(e)
    } finally {
      if (gen === this.#gen) this.loading = false
    }
  }

  setFilter(state: Filter) {
    this.state = state
    void this.reload()
  }

  /** Search as you type, waiting for a short pause. */
  setQuery(q: string) {
    this.query = q
    clearTimeout(this.#timer)
    this.#timer = setTimeout(() => void this.reload(), 350)
  }

  reset() {
    this.#gen++
    clearTimeout(this.#timer)
    this.items = []
    this.nextPage = null
    this.loaded = false
    this.error = null
    this.selected = null
    this.query = ''
    this.state = 'open'
  }
}

export const issues = new ForgeList<IssueSummary>((p, s, q, n) => api.forgeIssues(p, s, q, n))
export const pulls = new ForgeList<PullSummary>((p, s, q, n) => api.forgePulls(p, s, q, n))

/** The issue or pull request currently open on the right-hand side, and what you can do to it. */
class ForgeDetail {
  issue = $state<IssueDetail | null>(null)
  pull = $state<PullDetail | null>(null)
  files = $state.raw<PullFile[] | null>(null)
  loading = $state(false)
  filesLoading = $state(false)
  busy = $state(false)
  error = $state<string | null>(null)
  #gen = 0

  clear() {
    this.#gen++
    this.issue = null
    this.pull = null
    this.files = null
    this.error = null
    this.loading = false
  }

  async openIssue(number: number) {
    const path = projects.current?.path
    if (!path) return
    this.clear()
    issues.selected = number
    const gen = this.#gen
    this.loading = true
    try {
      const d = await api.forgeIssue(path, number)
      if (gen === this.#gen) this.issue = d
    } catch (e) {
      if (gen === this.#gen) this.error = asText(e)
    } finally {
      if (gen === this.#gen) this.loading = false
    }
  }

  async openPull(number: number) {
    const path = projects.current?.path
    if (!path) return
    this.clear()
    pulls.selected = number
    const gen = this.#gen
    this.loading = true
    try {
      const d = await api.forgePull(path, number)
      if (gen === this.#gen) this.pull = d
    } catch (e) {
      if (gen === this.#gen) this.error = asText(e)
    } finally {
      if (gen === this.#gen) this.loading = false
    }
  }

  async loadFiles() {
    const path = projects.current?.path
    const n = this.pull?.summary.number
    if (!path || !n || this.files || this.filesLoading) return
    const gen = this.#gen
    this.filesLoading = true
    try {
      const f = await api.forgePullFiles(path, n)
      if (gen === this.#gen) this.files = f
    } catch (e) {
      if (gen === this.#gen) this.error = asText(e)
    } finally {
      this.filesLoading = false
    }
  }

  /** Runs a write; shows the service's error text if it fails. */
  async #write<T>(op: (path: string) => Promise<T>): Promise<T | null> {
    const path = projects.current?.path
    if (!path || this.busy) return null
    this.busy = true
    this.error = null
    try {
      return await op(path)
    } catch (e) {
      ui.notice = { kind: 'error', text: asText(e) }
      return null
    } finally {
      this.busy = false
    }
  }

  /** Comment on whichever of issue / pull request is open. Returns true when posted. */
  async comment(body: string): Promise<boolean> {
    const number = this.issue?.summary.number ?? this.pull?.summary.number
    if (!number) return false
    const c = await this.#write((p) => api.forgeComment(p, number, body))
    if (!c) return false
    const add = (list: ForgeComment[]) => [...list, c]
    if (this.issue) this.issue = { ...this.issue, comments: add(this.issue.comments) }
    else if (this.pull) this.pull = { ...this.pull, comments: add(this.pull.comments) }
    return true
  }

  async setState(state: 'open' | 'closed') {
    const number = this.issue?.summary.number ?? this.pull?.summary.number
    if (!number) return
    const ok = await this.#write((p) => api.forgeSetState(p, number, state).then(() => true))
    if (!ok) return
    ui.notice = {
      kind: 'ok',
      text: state === 'closed' ? `#${number} closed.` : `#${number} reopened.`,
    }
    await Promise.all([
      this.issue ? this.openIssue(number) : this.openPull(number),
      this.issue ? issues.reload() : pulls.reload(),
    ])
  }

  async review(event: ReviewEvent, body: string): Promise<boolean> {
    const number = this.pull?.summary.number
    if (!number) return false
    const ok = await this.#write((p) => api.forgeReview(p, number, event, body).then(() => true))
    if (!ok) return false
    ui.notice = {
      kind: 'ok',
      text:
        event === 'APPROVE'
          ? 'Approved.'
          : event === 'REQUEST_CHANGES'
            ? 'Changes requested.'
            : 'Review comment posted.',
    }
    await this.openPull(number)
    return true
  }

  /** Comment on a diff line, or reply to a thread. */
  async lineComment(
    file: string,
    line: number,
    side: 'LEFT' | 'RIGHT',
    body: string,
    inReplyTo: number | null,
  ): Promise<boolean> {
    const pull = this.pull
    if (!pull) return false
    const rc = await this.#write((p) =>
      api.forgeReviewComment(
        p,
        pull.summary.number,
        pull.head_sha,
        file,
        line,
        side,
        body,
        inReplyTo,
      ),
    )
    if (!rc || this.pull?.summary.number !== pull.summary.number) return !!rc
    this.pull = { ...this.pull, review_comments: [...this.pull.review_comments, rc] }
    return true
  }

  /** Fetch the pull request and switch to a local branch for it. */
  async checkout(): Promise<void> {
    const number = this.pull?.summary.number
    const path = projects.current?.path
    if (!number || !path || this.busy) return
    this.busy = true
    try {
      ui.notice = { kind: 'ok', text: await api.checkoutPull(path, number) }
    } catch (e) {
      ui.notice = { kind: 'error', text: asText(e) }
    } finally {
      this.busy = false
      await Promise.all([repo.refresh(), changes.refresh()])
    }
  }
}

export const detail = new ForgeDetail()
