import type { CommitInfo } from '../types'

/** Small cross-component UI state. */
class Ui {
  /** Main area tab. */
  view = $state<'history' | 'changes' | 'issues' | 'pulls'>('history')
  branchSwitcherOpen = $state(false)
  stashDialogOpen = $state(false)
  /** Right-click menu on a commit. */
  commitMenu = $state<{ commit: CommitInfo; x: number; y: number } | null>(null)
  /** Small "name it" dialog for creating a branch or tag at a commit. */
  nameDialog = $state<{ kind: 'branch' | 'tag'; commit: CommitInfo } | null>(null)
  resetDialog = $state<CommitInfo | null>(null)
  /** Interactive rebase starting at this commit (it and everything newer can be edited). */
  interactiveRebase = $state<CommitInfo | null>(null)
  /** Blame / history of one file; `rev` null = the checked-out commit. */
  fileView = $state<{ path: string; rev: string | null; tab: 'blame' | 'history' } | null>(null)
  shortcutsOpen = $state(false)
  settingsOpen = $state(false)
  /** Right-click / "more" menu of a branch, positioned in window coordinates. */
  branchMenu = $state<{
    name: string
    remote: boolean
    current?: boolean
    x: number
    y: number
  } | null>(null)
  /** Confirmation dialog for merging / rebasing. */
  mergeDialog = $state<{ mode: 'merge' | 'rebase'; branch: string | null } | null>(null)
  /** Options menu of the Pull button (merge / rebase / fast-forward only, autostash). */
  pullMenu = $state<{ x: number; y: number } | null>(null)
  /** Transient message shown in the status banner (branch ops etc.). */
  notice = $state<{ kind: 'ok' | 'error'; text: string } | null>(null)
}

export const ui = new Ui()
