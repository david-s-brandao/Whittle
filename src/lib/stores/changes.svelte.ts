import { api } from '../api'
import { plural } from '../format'
import type { FileDiff, Selection, WorkStatus } from '../types'
import { projects } from './projects.svelte'

const empty: WorkStatus = { staged: [], unstaged: [], conflicted: [], operation: null }

export interface FileSelection {
  path: string
  staged: boolean
  /** The file is in conflict: show its text with markers instead of a diff. */
  conflict?: boolean
}

/** Working-tree state of the open repo: staged / unstaged files, diff, commit box. */
class Changes {
  status = $state.raw<WorkStatus>(empty)
  selection = $state<FileSelection | null>(null)
  diff = $state.raw<FileDiff | null>(null)
  /** Text of the selected conflicted file (with conflict markers). */
  conflictText = $state<string | null>(null)
  message = $state('')
  amend = $state(false)
  busy = $state(false)
  error = $state<string | null>(null)
  notice = $state<string | null>(null)

  #gen = 0
  #diffGen = 0
  #repoPath: string | undefined
  #commitDrafts = new Map<string, { message: string; amend: boolean }>()

  /** Number of distinct files with any change (what the tab badge shows). */
  get total(): number {
    const all = [
      ...this.status.staged.map((f) => f.path),
      ...this.status.unstaged.map((f) => f.path),
      ...this.status.conflicted,
    ]
    return all.filter((p, i) => all.indexOf(p) === i).length
  }

  async refresh() {
    const path = projects.current?.path
    const gen = ++this.#gen
    if (path !== this.#repoPath) {
      if (this.#repoPath)
        this.#commitDrafts.set(this.#repoPath, { message: this.message, amend: this.amend })
      const draft = path ? this.#commitDrafts.get(path) : undefined
      this.message = draft?.message ?? ''
      this.amend = draft?.amend ?? false
      this.notice = null
      this.#repoPath = path
      this.#diffGen++
      this.status = empty
      this.selection = null
      this.diff = null
      this.conflictText = null
      this.error = null
    }
    if (!path) {
      this.status = empty
      this.selection = null
      this.diff = null
      return
    }
    try {
      const status = await api.workStatus(path)
      if (gen !== this.#gen) return
      this.status = status
      // keep the selection if the file is still listed in the same area, else drop it
      const sel = this.selection
      if (sel?.conflict) {
        // a resolved file leaves the conflict list; follow it into "staged" if it went there
        if (!status.conflicted.includes(sel.path)) {
          this.selection = status.staged.some((f) => f.path === sel.path)
            ? { path: sel.path, staged: true }
            : null
        }
        await this.loadDiff()
        return
      }
      const list = sel ? (sel.staged ? status.staged : status.unstaged) : []
      if (sel && !list.some((f) => f.path === sel.path)) {
        const other = sel.staged ? status.unstaged : status.staged
        this.selection = other.some((f) => f.path === sel.path)
          ? { path: sel.path, staged: !sel.staged }
          : null
      }
      await this.loadDiff()
    } catch (e) {
      if (gen === this.#gen) this.error = String(e)
    }
  }

  async loadDiff() {
    const gen = ++this.#diffGen
    const path = projects.current?.path
    const sel = this.selection
    if (!path || !sel) {
      this.diff = null
      this.conflictText = null
      return
    }
    const same = () =>
      gen === this.#diffGen &&
      projects.current?.path === path &&
      this.selection?.path === sel.path &&
      this.selection.staged === sel.staged &&
      this.selection.conflict === sel.conflict
    try {
      if (sel.conflict) {
        const text = await api.readFileText(path, sel.path)
        if (same()) {
          this.conflictText = text
          this.diff = null
        }
      } else {
        const diff = await api.workFileDiff(path, sel.path, sel.staged)
        if (same()) {
          this.diff = diff
          this.conflictText = null
        }
      }
    } catch (e) {
      if (same()) this.error = String(e)
    }
  }

  async select(path: string, staged: boolean) {
    this.selection = { path, staged }
    this.diff = null
    this.conflictText = null
    await this.loadDiff()
  }

  async selectConflict(path: string) {
    this.selection = { path, staged: false, conflict: true }
    this.diff = null
    this.conflictText = null
    await this.loadDiff()
  }

  /** Runs a write operation, then reloads status. Errors surface in `error`. */
  async #run(op: (repoPath: string) => Promise<void>): Promise<boolean> {
    const path = projects.current?.path
    if (!path || this.busy) return false
    this.busy = true
    this.error = null
    this.notice = null
    try {
      await op(path)
      return true
    } catch (e) {
      if (projects.current?.path === path) this.error = String(e)
      return false
    } finally {
      this.busy = false
      await this.refresh()
    }
  }

  stage(files: string[]) {
    return this.#run((p) => api.stageFiles(p, files))
  }

  /** Stage (or, for a staged file, unstage) parts of the selected file's diff. */
  applySelection(selections: Selection[]) {
    const sel = this.selection
    if (!sel) return Promise.resolve(false)
    return this.#run((p) => api.stageSelection(p, sel.path, sel.staged, selections))
  }

  /** Resolve a conflicted file by taking one side wholesale. */
  resolve(file: string, side: 'ours' | 'theirs') {
    return this.#run((p) => api.resolveConflict(p, file, side))
  }

  /** Writes the merged result from the conflict editor and stages the file. */
  async saveResolved(file: string, content: string, advance = false) {
    const path = projects.current?.path
    const ok = await this.#run((p) => api.saveResolvedFile(p, file, content))
    if (ok && advance && projects.current?.path === path && this.selection?.path === file) {
      const next = this.status.conflicted[0]
      if (next) await this.selectConflict(next)
    }
    return ok
  }

  markResolved(file: string) {
    return this.#run((p) => api.markResolved(p, file))
  }

  async openInEditor(file: string) {
    const path = projects.current?.path
    if (!path) return
    try {
      await api.openInEditor(path, file)
    } catch (e) {
      this.error = String(e)
    }
  }

  unstage(files: string[]) {
    return this.#run((p) => api.unstageFiles(p, files))
  }

  async discard(files: string[]) {
    if (!files.length) return
    const what = files.length === 1 ? `“${files[0]}”` : plural(files.length, 'file')
    const ok = await api.confirm(
      `Discard all changes to ${what}?\n\nThis cannot be undone. New (untracked) files will be deleted.`,
      'Discard changes',
    )
    if (ok) await this.#run((p) => api.discardFiles(p, files))
  }

  async setAmend(on: boolean) {
    this.amend = on
    const path = projects.current?.path
    if (on && path && !this.message.trim()) {
      try {
        const message = await api.headMessage(path)
        if (projects.current?.path === path && this.amend && !this.message.trim())
          this.message = message
      } catch (e) {
        if (projects.current?.path !== path) return
        this.error = String(e)
        this.amend = false
      }
    }
  }

  get canCommit(): boolean {
    if (this.busy || this.status.conflicted.length) return false
    if (this.amend) return true // amend may only reword, or add staged files
    return this.status.staged.length > 0 && this.message.trim().length > 0
  }

  /** Why the commit button is disabled, in plain words (empty when enabled). */
  get blocker(): string {
    if (this.canCommit || this.busy) return ''
    if (this.status.conflicted.length) return 'Resolve all conflicted files before committing.'
    if (!this.status.staged.length) return 'Stage at least one file to commit.'
    return 'Write a commit message.'
  }

  async commit() {
    if (!this.canCommit) return
    const path = projects.current?.path
    const amend = this.amend
    const n = this.status.staged.length
    const ok = await this.#run((p) => api.commitChanges(p, this.message, amend))
    if (ok) {
      if (path) this.#commitDrafts.delete(path)
      if (projects.current?.path !== path) return
      this.message = ''
      this.amend = false
      this.notice = amend ? 'Commit amended.' : `Committed ${plural(n, 'file')}.`
    }
  }
}

export const changes = new Changes()
