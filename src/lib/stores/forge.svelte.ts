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
import { board } from './issueBoard.svelte'
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

/** Who can be asked to review: people with access who committed recently. */
class People {
  list = $state.raw<string[]>([])
  loading = $state(false)
  error = $state<string | null>(null)
  #path: string | null = null

  async load(force = false) {
    const path = projects.current?.path
    if (!path || this.loading) return
    if (!force && this.#path === path && !this.error) return
    this.loading = true
    this.error = null
    try {
      const list = await api.forgeReviewable(path)
      if (projects.current?.path === path) {
        this.list = list
        this.#path = path
      }
    } catch (e) {
      this.error = asText(e)
    } finally {
      this.loading = false
    }
  }

  reset() {
    this.list = []
    this.error = null
    this.#path = null
  }
}

export const people = new People()

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
  #filesGen = 0
  refreshing = $state(false)
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
    this.#filesGen++
    this.issue = null
    this.pull = null
    this.files = null
    this.error = null
    this.loading = false
    this.refreshing = false
    this.filesLoading = false
  }

  /** Refresh in place, retaining the mounted composer, draft and scroll position. */
  async refresh() {
    const path = projects.current?.path
    const issue = this.issue
    const pull = this.pull
    if (!path || (!issue && !pull) || this.busy || this.refreshing) return
    const gen = this.#gen
    this.refreshing = true
    try {
      if (issue) {
        const updated = await api.forgeIssue(path, issue.summary.number)
        if (gen === this.#gen) this.issue = updated
      } else if (pull) {
        const updated = await api.forgePull(path, pull.summary.number)
        if (gen === this.#gen) {
          this.pull = updated
          this.#filesGen++
          this.files = null
          this.filesLoading = false
        }
      }
    } catch (e) {
      if (gen === this.#gen) ui.notice = { kind: 'error', text: asText(e) }
    } finally {
      if (gen === this.#gen) this.refreshing = false
    }
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
    const filesGen = ++this.#filesGen
    this.filesLoading = true
    try {
      const f = await api.forgePullFiles(path, n)
      if (gen === this.#gen && filesGen === this.#filesGen) this.files = f
    } catch (e) {
      if (gen === this.#gen && filesGen === this.#filesGen) this.error = asText(e)
    } finally {
      if (gen === this.#gen && filesGen === this.#filesGen) this.filesLoading = false
    }
  }

  /** Runs a write; shows the service's error text if it fails. */
  async #write<T>(op: (path: string) => Promise<T>): Promise<T | null> {
    const path = projects.current?.path
    if (!path || this.busy || this.refreshing) return null
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
    const gen = this.#gen
    const number = this.issue?.summary.number ?? this.pull?.summary.number
    if (!number) return false
    const c = await this.#write((p) => api.forgeComment(p, number, body))
    if (!c) return false
    if (gen !== this.#gen) return true
    const add = (list: ForgeComment[]) => [...list, c]
    if (this.issue)
      this.issue = {
        ...this.issue,
        summary: { ...this.issue.summary, comments: this.issue.summary.comments + 1 },
        comments: add(this.issue.comments),
      }
    else if (this.pull) this.pull = { ...this.pull, comments: add(this.pull.comments) }
    return true
  }

  async setState(state: 'open' | 'closed') {
    const gen = this.#gen
    const number = this.issue?.summary.number ?? this.pull?.summary.number
    if (!number) return
    const ok = await this.#write((p) => api.forgeSetState(p, number, state).then(() => true))
    if (!ok || gen !== this.#gen) return
    ui.notice = {
      kind: 'ok',
      text: state === 'closed' ? `#${number} closed.` : `#${number} reopened.`,
    }
    const isIssue = !!this.issue
    await Promise.all([
      isIssue ? this.openIssue(number) : this.openPull(number),
      isIssue ? issues.reload() : pulls.reload(),
      board.loaded ? board.load() : Promise.resolve(),
    ])
  }

  async review(event: ReviewEvent, body: string): Promise<boolean> {
    const gen = this.#gen
    const number = this.pull?.summary.number
    if (!number) return false
    const ok = await this.#write((p) => api.forgeReview(p, number, event, body).then(() => true))
    if (!ok) return false
    if (gen !== this.#gen) return true
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
    const gen = this.#gen
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
    if (!rc || gen !== this.#gen || this.pull?.summary.number !== pull.summary.number) return !!rc
    this.pull = { ...this.pull, review_comments: [...this.pull.review_comments, rc] }
    return true
  }

  /** Replace the people asked to review the open pull request (whoever opened it). */
  async setReviewers(reviewers: string[]): Promise<boolean> {
    const gen = this.#gen
    const pull = this.pull
    if (!pull) return false
    const number = pull.summary.number
    const saved = await this.#write((p) => api.forgeSetReviewers(p, number, reviewers))
    if (!saved) return false
    if (gen === this.#gen && this.pull) {
      this.pull = { ...this.pull, summary: { ...this.pull.summary, reviewers: saved } }
    }
    pulls.items = pulls.items.map((i) => (i.number === number ? { ...i, reviewers: saved } : i))
    return true
  }

  /** Open a pull request, then show it. Returns true when created. */
  async createPull(
    head: string,
    base: string,
    title: string,
    body: string,
    draft: boolean,
    reviewers: string[],
  ): Promise<boolean> {
    const created = await this.#write((p) =>
      api.forgeCreatePull(p, head, base, title, body, draft, reviewers),
    )
    if (!created) return false
    ui.notice = { kind: 'ok', text: `Pull request #${created.number} created.` }
    pulls.state = 'open'
    pulls.query = ''
    await Promise.all([pulls.reload(), this.openPull(created.number)])
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
