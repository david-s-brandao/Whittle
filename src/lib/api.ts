import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ask, open } from '@tauri-apps/plugin-dialog'
import type {
  Blame,
  CommitInfo,
  ForgeComment,
  ForgePage,
  ForgeStatus,
  IssueDetail,
  IssueSummary,
  PullDetail,
  PullFile,
  PullSummary,
  ReviewComment,
  ReviewEvent,
  FileChange,
  FileDiff,
  Project,
  RepoRefs,
  MergeOptions,
  MergePreview,
  OpResult,
  RebaseItem,
  RepoSummary,
  Selection,
  Settings,
  SignatureInfo,
  SubmoduleInfo,
  ProgressOp,
  SyncOp,
  WorkStatus,
} from './types'

export const api = {
  listProjects: () => invoke<Project[]>('list_projects'),
  openProject: (path: string) => invoke<Project>('open_project', { path }),
  setStarred: (path: string, starred: boolean) =>
    invoke<void>('set_project_starred', { path, starred }),
  removeProject: (path: string) => invoke<void>('remove_project', { path }),
  repoSummary: (path: string) => invoke<RepoSummary>('repo_summary', { path }),
  repoRefs: (path: string) => invoke<RepoRefs>('repo_refs', { path }),
  repoLog: (path: string, skip: number, limit: number) =>
    invoke<CommitInfo[]>('repo_log', { path, skip, limit }),
  searchCommits: (path: string, query: string) =>
    invoke<CommitInfo[]>('search_commits', { path, query }),
  commitFiles: (path: string, id: string) => invoke<FileChange[]>('commit_files', { path, id }),
  commitFileDiff: (path: string, id: string, file: string) =>
    invoke<FileDiff>('commit_file_diff', { path, id, file }),
  workStatus: (path: string) => invoke<WorkStatus>('work_status', { path }),
  workFileDiff: (path: string, file: string, staged: boolean) =>
    invoke<FileDiff>('work_file_diff', { path, file, staged }),
  stageSelection: (path: string, file: string, unstage: boolean, selections: Selection[]) =>
    invoke<void>('stage_selection', { path, file, unstage, selections }),
  stashPush: (path: string, message: string, includeUntracked: boolean) =>
    invoke<string>('stash_push', { path, message, includeUntracked }),
  stashApply: (path: string, index: number, pop: boolean) =>
    invoke<string>('stash_apply', { path, index, pop }),
  stashDrop: (path: string, index: number) => invoke<string>('stash_drop', { path, index }),
  mergeBranch: (path: string, branch: string, options: MergeOptions) =>
    invoke<OpResult>('merge_branch', { path, branch, options }),
  mergePreview: (path: string, source: string) =>
    invoke<MergePreview>('merge_preview', { path, source }),
  rebaseOnto: (path: string, onto: string) => invoke<OpResult>('rebase_onto', { path, onto }),
  operationStep: (path: string, action: 'continue' | 'skip' | 'abort') =>
    invoke<OpResult>('operation_step', { path, action }),
  resolveConflict: (path: string, file: string, side: 'ours' | 'theirs') =>
    invoke<void>('resolve_conflict', { path, file, side }),
  markResolved: (path: string, file: string) => invoke<void>('mark_resolved', { path, file }),
  readFileText: (path: string, file: string) => invoke<string>('read_file_text', { path, file }),
  saveResolvedFile: (path: string, file: string, content: string) =>
    invoke<void>('save_resolved_file', { path, file, content }),
  openInEditor: (path: string, file: string) => invoke<void>('open_in_editor', { path, file }),
  cherryPick: (path: string, commit: string) =>
    invoke<OpResult>('cherry_pick_commit', { path, commit }),
  revertCommit: (path: string, commit: string) =>
    invoke<OpResult>('revert_commit', { path, commit }),
  resetTo: (path: string, target: string, mode: 'soft' | 'mixed' | 'hard') =>
    invoke<string>('reset_to', { path, target, mode }),
  checkoutCommit: (path: string, commit: string) =>
    invoke<void>('checkout_commit', { path, commit }),
  createTag: (path: string, name: string, target: string | null, message: string | null) =>
    invoke<void>('create_tag', { path, name, target, message }),
  deleteTag: (path: string, name: string) => invoke<string>('delete_tag', { path, name }),
  pushTag: (path: string, name: string, remoteDelete: boolean) =>
    invoke<void>('push_tag', { path, name, delete: remoteDelete }),
  commitSignature: (path: string, commit: string) =>
    invoke<SignatureInfo>('commit_signature', { path, commit }),
  fileBlame: (path: string, file: string, rev: string | null) =>
    invoke<Blame>('file_blame', { path, file, rev }),
  fileHistory: (path: string, file: string) => invoke<CommitInfo[]>('file_history', { path, file }),
  repoSubmodules: (path: string) => invoke<SubmoduleInfo[]>('repo_submodules', { path }),
  updateSubmodules: (path: string, sub: string | null) =>
    invoke<void>('update_submodules', { path, sub }),
  rebasePlan: (path: string, from: string) => invoke<CommitInfo[]>('rebase_plan', { path, from }),
  rebaseInteractive: (path: string, from: string, items: RebaseItem[]) =>
    invoke<OpResult>('rebase_interactive', { path, from, items }),
  getSettings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) => invoke<Settings>('save_settings', { settings }),
  gitVersion: (path: string | null) => invoke<string>('git_version', { path }),
  openTerminal: (path: string) => invoke<void>('open_terminal', { path }),
  openEditor: (path: string) => invoke<void>('open_editor', { path }),
  openFolder: (path: string) => invoke<void>('open_folder', { path }),
  forgeStatus: (path: string) => invoke<ForgeStatus>('forge_status', { path }),
  forgeConnect: (path: string, token: string) =>
    invoke<ForgeStatus>('forge_connect', { path, token }),
  forgeDisconnect: (path: string) => invoke<ForgeStatus>('forge_disconnect', { path }),
  forgeIssues: (path: string, state: string, query: string, page: number) =>
    invoke<ForgePage<IssueSummary>>('forge_issues', { path, state, query, page }),
  forgeIssue: (path: string, number: number) =>
    invoke<IssueDetail>('forge_issue', { path, number }),
  forgePulls: (path: string, state: string, query: string, page: number) =>
    invoke<ForgePage<PullSummary>>('forge_pulls', { path, state, query, page }),
  forgePull: (path: string, number: number) => invoke<PullDetail>('forge_pull', { path, number }),
  forgePullFiles: (path: string, number: number) =>
    invoke<PullFile[]>('forge_pull_files', { path, number }),
  forgeComment: (path: string, number: number, body: string) =>
    invoke<ForgeComment>('forge_comment', { path, number, body }),
  forgeSetState: (path: string, number: number, state: 'open' | 'closed') =>
    invoke<void>('forge_set_state', { path, number, state }),
  forgeReview: (path: string, number: number, event: ReviewEvent, body: string) =>
    invoke<void>('forge_review', { path, number, event, body }),
  forgeReviewComment: (
    path: string,
    number: number,
    commitId: string,
    file: string,
    line: number,
    side: 'LEFT' | 'RIGHT',
    body: string,
    inReplyTo: number | null,
  ) =>
    invoke<ReviewComment>('forge_review_comment', {
      path,
      number,
      commitId,
      file,
      line,
      side,
      body,
      inReplyTo,
    }),
  checkoutPull: (path: string, number: number) => invoke<string>('checkout_pull', { path, number }),
  openUrl: (url: string) => invoke<void>('open_url', { url }),
  stageFiles: (path: string, files: string[]) => invoke<void>('stage_files', { path, files }),
  unstageFiles: (path: string, files: string[]) => invoke<void>('unstage_files', { path, files }),
  discardFiles: (path: string, files: string[]) => invoke<void>('discard_files', { path, files }),
  commitChanges: (path: string, message: string, amend: boolean) =>
    invoke<void>('commit_changes', { path, message, amend }),
  headMessage: (path: string) => invoke<string>('head_message', { path }),
  confirm: (message: string, title: string, okLabel = 'Discard') =>
    ask(message, { title, kind: 'warning', okLabel, cancelLabel: 'Cancel' }),
  checkoutBranch: (path: string, name: string, remote: boolean) =>
    invoke<void>('checkout_branch', { path, name, remote }),
  createBranch: (path: string, name: string, start: string | null, switchTo: boolean) =>
    invoke<void>('create_branch', { path, name, start, switch: switchTo }),
  deleteBranch: (path: string, name: string, force: boolean) =>
    invoke<string>('delete_branch', { path, name, force }),
  watchRepo: (path: string) => invoke<void>('watch_repo', { path }),
  unwatchRepo: () => invoke<void>('unwatch_repo'),
  onRepoChanged: (cb: (e: { path: string; worktree: boolean; git: boolean }) => void) =>
    listen<{ path: string; worktree: boolean; git: boolean }>('repo-changed', (e) => cb(e.payload)),
  defaultCloneDir: () => invoke<string>('default_clone_dir'),
  cloneName: (url: string) => invoke<string>('clone_name', { url }),
  cloneRepo: (url: string, parent: string, name: string) =>
    invoke<string>('clone_repo', { url, parent, name }),
  gitSync: (path: string, op: SyncOp, force = false, pull?: { mode: string; autostash: boolean }) =>
    invoke<void>('git_sync', { path, op, force, pull: pull ?? null }),
  onProgress: (cb: (op: ProgressOp, line: string) => void) =>
    listen<{ op: ProgressOp; line: string }>('git-progress', (e) =>
      cb(e.payload.op, e.payload.line),
    ),
  pickFolder: async (): Promise<string | null> => {
    const r = await open({ directory: true, multiple: false })
    return typeof r === 'string' ? r : null
  },
}
