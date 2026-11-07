import { api } from '../api'
import { GraphBuilder, type GraphRow } from '../graph'
import type {
  CommitInfo,
  FileChange,
  FileDiff,
  RefLabel,
  RepoRefs,
  RepoSummary,
  SignatureInfo,
  SubmoduleInfo,
} from '../types'
import { projects } from './projects.svelte'

const empty: RepoRefs = {
  local: [],
  remote: [],
  tags: [],
  stashes: [],
  head_id: null,
  labels: [],
}

// most relevant first: the checked-out branch, what it tracks, other branches, remotes, tags
const rank = (l: RefLabel) =>
  l.current ? 0 : l.tracked_by_current ? 1 : l.kind === 'local' ? 2 : l.kind === 'remote' ? 3 : 4
const PAGE = 500

/** State of the currently open repository. */
class Repo {
  refs = $state<RepoRefs>(empty)
  summary = $state<RepoSummary | null>(null)
  submodules = $state.raw<SubmoduleInfo[]>([])
  /** Branch / tag labels grouped by the commit they point at. */
  labelsById = $derived.by(() => {
    const out: Record<string, RefLabel[]> = {}
    for (const l of this.refs.labels) (out[l.id] ??= []).push(l)
    for (const list of Object.values(out)) list.sort((a, b) => rank(a) - rank(b))
    return out
  })
  /** Bumped to ask the commit list to scroll to `id`. */
  reveal = $state<{ id: string; nonce: number } | null>(null)
  commits = $state.raw<CommitInfo[]>([])
  /** Lane layout for `commits`, same length and order. */
  graph = $state.raw<GraphRow[]>([])
  graphLanes = $state(1)
  #builder = new GraphBuilder()
  allLoaded = $state(false)
  loadingCommits = $state(false)
  error = $state<string | null>(null)

  selected = $state<CommitInfo | null>(null)
  files = $state.raw<FileChange[]>([])
  selectedFile = $state<string | null>(null)
  diff = $state.raw<FileDiff | null>(null)
  diffError = $state<string | null>(null)
  /** Signature check of the selected commit (only asked for signed commits). */
  signature = $state<SignatureInfo | null>(null)

  /** Bumped on every reload so stale async results are discarded. */
  #gen = 0

  /** Path whose data is currently shown; refreshing the same path keeps selection and scroll. */
  #path: string | undefined

  async refresh(path: string | undefined = projects.current?.path) {
    const gen = ++this.#gen
    const soft = path !== undefined && path === this.#path
    this.#path = path
    // How many commits to re-fetch: keep what the user has already scrolled through.
    const want = soft ? Math.min(2000, Math.max(PAGE, this.commits.length)) : PAGE
    if (!soft) {
      this.#setCommits([])
      this.allLoaded = false
      this.clearSelection()
    }
    this.loadingCommits = false
    if (!path) {
      this.refs = empty
      this.summary = null
      this.submodules = []
      return
    }
    try {
      const [refs, summary, submodules] = await Promise.all([
        api.repoRefs(path),
        api.repoSummary(path),
        api.repoSubmodules(path).catch(() => []),
      ])
      if (gen !== this.#gen) return
      this.refs = refs
      this.summary = summary
      this.submodules = submodules ?? []
      this.error = null
    } catch (e) {
      if (gen === this.#gen) this.error = String(e)
    }
    if (gen !== this.#gen) return
    this.loadingCommits = true
    try {
      const page = await api.repoLog(path, 0, want)
      if (gen !== this.#gen) return
      this.#setCommits(page)
      this.allLoaded = page.length < want
    } catch (e) {
      if (gen === this.#gen) this.error = String(e)
    } finally {
      if (gen === this.#gen) this.loadingCommits = false
    }
  }

  #setCommits(list: CommitInfo[]) {
    const b = new GraphBuilder()
    this.graph = list.map((c) => b.push(c))
    this.#builder = b
    this.graphLanes = b.maxLanes
    this.commits = list
  }

  #appendCommits(page: CommitInfo[]) {
    this.graph = this.graph.concat(page.map((c) => this.#builder.push(c)))
    this.graphLanes = this.#builder.maxLanes
    this.commits = this.commits.concat(page)
  }

  /** Loads the next page of history (no-op if already loading or finished). */
  async loadMore() {
    const path = projects.current?.path
    if (!path || this.loadingCommits || this.allLoaded) return
    const gen = this.#gen
    this.loadingCommits = true
    try {
      const page = await api.repoLog(path, this.commits.length, PAGE)
      if (gen !== this.#gen) return
      this.#appendCommits(page)
      if (page.length < PAGE) this.allLoaded = true
    } catch (e) {
      if (gen === this.#gen) this.error = String(e)
    } finally {
      if (gen === this.#gen) this.loadingCommits = false
    }
  }

  /** Scroll the history to the commit HEAD points at, loading more history if needed. */
  async revealHead() {
    const id = this.refs.head_id
    if (id) await this.revealCommit(id)
  }

  /** Scroll the history to a commit (loading more history if needed). */
  async revealCommit(id: string) {
    while (!this.commits.some((c) => c.id === id) && !this.allLoaded) {
      const before = this.commits.length
      await this.loadMore()
      if (this.commits.length === before) break
    }
    if (this.commits.some((c) => c.id === id)) {
      this.reveal = { id, nonce: (this.reveal?.nonce ?? 0) + 1 }
    }
  }

  clearSelection() {
    this.selected = null
    this.files = []
    this.selectedFile = null
    this.diff = null
    this.diffError = null
  }

  async selectCommit(c: CommitInfo) {
    const path = projects.current?.path
    if (!path) return
    this.selected = c
    this.signature = null
    if (c.signed) {
      api
        .commitSignature(path, c.id)
        .then((sig) => {
          if (this.selected?.id === c.id) this.signature = sig
        })
        .catch(() => {})
    }
    this.files = []
    this.selectedFile = null
    this.diff = null
    this.diffError = null
    try {
      const files = await api.commitFiles(path, c.id)
      if (this.selected?.id !== c.id) return
      this.files = files
      if (files.length) await this.selectFile(files[0].path)
    } catch (e) {
      if (this.selected?.id === c.id) this.diffError = String(e)
    }
  }

  async selectFile(file: string) {
    const path = projects.current?.path
    const commit = this.selected
    if (!path || !commit) return
    this.selectedFile = file
    this.diff = null
    try {
      const diff = await api.commitFileDiff(path, commit.id, file)
      if (this.selected?.id === commit.id && this.selectedFile === file) this.diff = diff
    } catch (e) {
      if (this.selected?.id === commit.id) this.diffError = String(e)
    }
  }
}

export const repo = new Repo()
