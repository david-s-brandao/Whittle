export interface Project {
  path: string
  name: string
  starred: boolean
  last_used: number
}

export interface RepoSummary {
  branch: string | null
  dirty: boolean
  ahead: number
  behind: number
  has_upstream: boolean
}

export interface BranchInfo {
  name: string
  is_head: boolean
  upstream: string | null
}

export interface StashInfo {
  index: number
  message: string
  /** Stashes are commits, so their contents can be shown like one. */
  id: string
  time: number
}

/** A branch / tag name attached to a commit, drawn next to it in the history. */
export interface RefLabel {
  id: string
  name: string
  kind: 'local' | 'remote' | 'tag' | 'head'
  /** The checked-out branch (or the detached HEAD). */
  current: boolean
  /** The remote branch the checked-out branch tracks, e.g. origin/main. */
  tracked_by_current: boolean
}

export interface RepoRefs {
  local: BranchInfo[]
  remote: BranchInfo[]
  tags: string[]
  stashes: StashInfo[]
  /** Commit HEAD points at. */
  head_id: string | null
  labels: RefLabel[]
}

export interface CommitInfo {
  id: string
  short_id: string
  summary: string
  author: string
  email: string
  time: number
  parents: string[]
  /** Carries a GPG or SSH signature (see SignatureInfo for whether it checks out). */
  signed: boolean
}

export interface CommitMessage {
  message: string
  committer: string
  committer_email: string
  committer_time: number
  /** Branches and tags pointing exactly at the commit. */
  refs: string[]
}

export interface FileChange {
  path: string
  old_path: string | null
  status: 'added' | 'modified' | 'deleted' | 'renamed' | 'copied' | 'typechange' | 'untracked'
}

export interface DiffLine {
  /** '\\' is the "No newline at end of file" marker. */
  kind: '+' | '-' | ' ' | '\\'
  old_no: number | null
  new_no: number | null
  text: string
}

export interface Hunk {
  /** Fingerprint; echoed back when staging so a stale view is detected. */
  id: string
  header: string
  old_start: number
  new_start: number
  lines: DiffLine[]
}

export interface FileDiff {
  binary: boolean
  hunks: Hunk[]
}

export interface WorkStatus {
  staged: FileChange[]
  unstaged: FileChange[]
  conflicted: string[]
  /** A paused multi-step operation waiting for the user. */
  operation: 'merge' | 'rebase' | 'cherry-pick' | 'revert' | null
}

export type SyncOp = 'fetch' | 'pull' | 'push'
/** Operations that report progress events (clone is separate from SyncOp). */
export type ProgressOp = SyncOp | 'clone'

export interface SyncError {
  kind:
    | 'auth_ssh'
    | 'auth_https'
    | 'network'
    | 'rejected'
    | 'diverged'
    | 'conflict'
    | 'no_remote'
    | 'no_upstream'
    | 'other'
  message: string
  details: string
}

export type RebaseAction = 'pick' | 'reword' | 'squash' | 'fixup' | 'drop'

/** One line of an interactive rebase plan. */
export interface RebaseItem {
  id: string
  action: RebaseAction
  message: string | null
}

// ---- issues & pull requests ----

export interface ForgeStatus {
  supported: boolean
  provider: 'github' | 'gitlab' | 'other' | 'none'
  host: string
  owner: string
  repo: string
  connected: boolean
  login: string | null
  /** keyring = a token pasted into Whittle, gh = the GitHub CLI login. */
  source: 'keyring' | 'gh' | 'none'
  message: string | null
}

export interface ForgeLabel {
  name: string
  color: string
}

export interface IssueSummary {
  number: number
  title: string
  state: 'open' | 'closed'
  author: string
  labels: ForgeLabel[]
  assignees: string[]
  comments: number
  created_at: string
  updated_at: string
  url: string
}

export interface PullSummary {
  number: number
  title: string
  state: 'open' | 'closed' | 'merged'
  draft: boolean
  author: string
  head_ref: string
  base_ref: string
  labels: ForgeLabel[]
  /** People asked to review who haven't yet. */
  reviewers: string[]
  /** Start of the description (used to find which issues a pull request mentions). */
  body: string
  created_at: string
  updated_at: string
  url: string
}

export interface ForgeComment {
  id: number
  author: string
  body: string
  created_at: string
  url: string
  association: string
}

export interface ForgeReview {
  id: number
  author: string
  state: 'APPROVED' | 'CHANGES_REQUESTED' | 'COMMENTED' | 'DISMISSED' | 'PENDING'
  body: string
  submitted_at: string
  url: string
}

export interface ReviewComment {
  id: number
  author: string
  body: string
  path: string
  line: number | null
  side: 'LEFT' | 'RIGHT'
  created_at: string
  in_reply_to: number | null
  /** The review this comment was posted with. */
  review_id: number | null
  url: string
  commit_id: string
}

export interface IssueDetail {
  summary: IssueSummary
  body: string
  comments: ForgeComment[]
}

export interface PullDetail {
  summary: PullSummary
  body: string
  mergeable_state: string | null
  additions: number
  deletions: number
  changed_files: number
  commits: number
  head_sha: string
  comments: ForgeComment[]
  reviews: ForgeReview[]
  review_comments: ReviewComment[]
}

export interface RepoBranches {
  default_branch: string
  branches: string[]
}

export interface PullFile {
  filename: string
  previous_filename: string | null
  status: string
  additions: number
  deletions: number
  patch: string | null
}

export interface ForgePage<T> {
  items: T[]
  next_page: number | null
}

export type ReviewEvent = 'APPROVE' | 'REQUEST_CHANGES' | 'COMMENT'

export interface Settings {
  clone_dir: string | null
  git_path: string | null
  editor_cmd: string | null
  terminal_cmd: string | null
  font_size: number
  accent: string
  auto_fetch_minutes: number
}

export interface SubmoduleInfo {
  name: string
  path: string
  url: string | null
  status: 'uninitialized' | 'clean' | 'new_commits' | 'dirty'
  expected_id: string | null
  current_id: string | null
}

export interface BlameHunk {
  start: number
  count: number
  id: string
  short_id: string
  author: string
  time: number
  summary: string
}

export interface Blame {
  hunks: BlameHunk[]
  lines: string[]
}

export interface SignatureInfo {
  signed: boolean
  status: 'good' | 'good_untrusted' | 'expired' | 'revoked' | 'bad' | 'unverifiable' | 'none'
  signer: string
  key: string
}

/** Part of a file's diff to stage/unstage. `lines: null` = the whole hunk. */
export interface Selection {
  hunk_id: string
  lines: number[] | null
}

/** What a merge / rebase step ended with. */
export interface OpResult {
  outcome: 'done' | 'up_to_date' | 'conflicts'
  message: string
}

export interface MergeOptions {
  ff: 'auto' | 'never' | 'only'
  squash: boolean
  no_commit: boolean
  message: string | null
}

/** What merging a branch into the current one would do (computed without touching files). */
export interface MergePreview {
  source: string
  target: string
  incoming: number
  only_here: number
  up_to_date: boolean
  can_fast_forward: boolean
  commits: CommitInfo[]
  files: number
  /** Predicted conflicting files; null if it could not be checked. */
  conflicts: string[] | null
}
