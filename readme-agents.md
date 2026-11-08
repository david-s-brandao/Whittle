# Whittle — Git GUI for Ubuntu (formerly the working name "Knife")

## Mission

Build a fast, clean, intuitive desktop Git client for Linux (Ubuntu first), inspired by the workflow of Fork (Mac/Windows only). It should cover the everyday Git basics, work with private repositories, and feel polished.

Inspired by Fork's _workflow_, not its look: do not copy Fork's name, logo, icons or visual assets. Knife has its own identity.

---

## Tech stack (required)

| Layer         | Choice                                                                                        |
| ------------- | --------------------------------------------------------------------------------------------- |
| App shell     | **Tauri 2**                                                                                   |
| Backend       | **Rust**                                                                                      |
| Frontend      | **Svelte 5 + TypeScript**                                                                     |
| Styling       | **Tailwind CSS**                                                                              |
| Git reads     | **git2-rs** (libgit2) or **gitoxide** for log, status, diffs, refs, blame                     |
| Git writes    | **git CLI** (spawned from Rust) for commit, push, pull, fetch, merge, rebase, checkout, stash |
| Diff viewer   | CodeMirror 6 (or Monaco if needed)                                                            |
| Local storage | SQLite (`rusqlite`) or JSON in `~/.config/knife/`                                             |
| Packaging     | `.deb` and AppImage                                                                           |

**Why the split:** library reads are fast and need no process spawning. CLI writes automatically respect the user's SSH keys, credential helpers, hooks, GPG signing and `.gitconfig`.

---

## Design & UX

### Colors

- **Primary accent: orange** (suggested `#F97316`, hover `#EA580C`). Define it as a CSS variable / Tailwind token so it can be tweaked in one place.
- **Light mode:** white / very light gray surfaces, dark text, orange accents.
- **Dark mode:** near-black / dark gray surfaces, light text, **same orange accent**.
- Theme options: Light / Dark / Follow system. Persist the choice.

### Layout (main window)

```
┌──────────────────────────────────────────────────────────────┐
│ [▣ Project ▾]  branch: main ▾   Fetch  Pull  Push  Stash  ⚙  │  ← toolbar
├───────────┬──────────────────────────────────────────────────┤
│ Sidebar   │  Commit graph + list (message, author, date, hash)│
│ Branches  │                                                  │
│ Remotes   ├──────────────────────────────────────────────────┤
│ Tags      │  Details: changed files  |  diff viewer          │
│ Stashes   │                                                  │
└───────────┴──────────────────────────────────────────────────┘
```

Plus a **Changes** view: unstaged / staged file lists, diff, and a commit message box.

### Project switcher (key feature)

- A **button in the top-left** showing the current project name opens a **modal** (not a plain dropdown).
- Keyboard shortcut: `Ctrl+P` opens it, typing filters instantly (fuzzy search), `Enter` opens, `Esc` closes.
- Sections in the modal:
  1. **★ Starred**: pinned projects at the top.
  2. **Recent**: all other projects sorted by **last used** (most recent first).
- Each row: star toggle, project name, path, current branch, and an optional dirty indicator.
- Actions in the modal: **Open local repo**, **Clone repo**, and **Remove from list** (does not delete files).
- Optional tabs for multiple open repos, as in Fork (phase 3).

### UX principles

- Everything common is at most 2 clicks or has a shortcut.
- Long operations (fetch, push, clone) show progress and never freeze the UI. Run them async in Rust and stream progress events to the frontend.
- Errors are human-readable, with a "show details" toggle for the raw git output.
- Confirm destructive actions (discard changes, force push, delete branch, hard reset).

---

## Private repositories

- **SSH:** rely on the system `ssh-agent` and `~/.ssh` keys through the git CLI. Show a clear message if no key or agent is found.
- **HTTPS:** rely on the user's git credential helper. Optionally offer to store tokens in the **system keyring** (`keyring` crate → libsecret / GNOME Keyring). **Never store tokens in plain text.**
- Clone dialog accepts SSH and HTTPS URLs and shows auth errors clearly.
- Optional later: GitHub / GitLab account sign-in to list and clone private repos.

---

## Phases & tasks

### Phase 0 — Setup

- [x] Scaffold Tauri 2 + Svelte 5 + TypeScript + Tailwind
- [x] Set up the Rust workspace: `git` module (reads), `cli` module (writes), `store` module (projects/settings)
- [x] Add theme tokens (orange accent, light/dark surfaces) and the theme toggle
- [x] Configure lint/format: `cargo fmt`, `clippy`, `prettier`, `eslint`
- [x] Verify `cargo tauri dev` runs on Ubuntu

### Phase 1 — MVP

- [x] Open a local repository (folder picker; validate that it is a git repo)
- [x] Project store: add/remove, last-used timestamp, starred flag
- [x] **Project switcher modal** with Starred + Recent, fuzzy search, and `Ctrl+P`
- [x] Sidebar: local branches, remote branches, tags, stashes
- [x] Commit list (virtualized) with message, author, date and short hash
- [x] Commit details: changed files + diff
- [x] Changes view: unstaged/staged lists, stage/unstage file, discard file (with confirm)
- [x] Commit (message + optional amend)
- [x] Fetch / Pull / Push with progress and error handling
- [x] Checkout branch, create branch, delete branch
- [x] Clone repo dialog (SSH + HTTPS)
- [x] Auto-refresh when files change on disk (`notify` crate, debounced)

### Phase 2 — Commit graph & polish

- [x] **Commit graph**: lane-assignment algorithm (merges, forks, lane reuse, stable lanes), drawn on `<canvas>` alongside the virtualized rows
- [x] Lane colors: generated palette with enough contrast between neighbors, working in both themes
- [x] Stage/unstage individual hunks and lines
- [x] Stash: create, apply, pop, drop
- [x] Merge, and rebase onto a branch
- [x] Merge-conflict view: list conflicted files, use ours/theirs, open in an external editor, mark resolved
- [x] Built-in merge conflict editor: side-by-side current/incoming per conflict, pick or combine, edit the result, save and stage
- [x] Conflict editor polish: session drafts per repository/file, undo/redo while editing, live full-result preview, stale-draft detection and rebase-aware side labels
- [x] Left-aligned folder-style repository navigation with keyboard selection and change/conflict indicators; responsive conflict workspace and commit blocking while conflicts remain
- [x] Conflict workflow: save with `Ctrl+S`, optionally advance to the next conflicted file, filter files by path
- [x] Commit productivity: session drafts per project and conventional-commit type helpers that preserve scope, breaking-change flag and body
- [x] Search commits (message, author, hash)
- [x] Keyboard shortcuts panel (`?`)

### Phase 3 — Nice to have

- [x] Multiple repos in tabs
- [x] Interactive rebase (reorder, squash, edit message, drop)
- [x] Cherry-pick, revert, reset (soft/mixed/hard, with confirm)
- [x] Blame and file history
- [x] Tags: create, delete, push
- [x] Submodule support
- [x] GPG/SSH commit-signing indicator
- [x] "Open in terminal" / "Open in editor" actions
- [x] Settings page: default clone folder, git binary path, font size, accent shade

---

### Phase 4 — Issues & pull requests (GitHub)
- [x] **Issues** tab: list with open/closed/all filter and search, read the full thread (markdown), comment, close/reopen
- [x] **Pull requests** tab: list, conversation (comments + reviews), files changed with inline comment threads, submit a review (comment / approve / request changes), close/reopen, check out locally
- [x] Sign-in without storing secrets in files: system keyring for pasted tokens, or the existing GitHub CLI login
- [x] GitHub browser shortcuts in the command palette: repository, Actions/CI, new issue and new PR from the current branch
- [x] Centered issue/PR discussions and composers, per-thread session drafts, quote replies, Markdown formatting tools, jump-to-comment and in-place discussion refresh
- [x] PR file filtering and stale-response protection when navigating between discussions
- [ ] GitLab / Bitbucket support
- [ ] Merge a pull request from the app

---

## Extra ideas

- [x] **Quick actions bar** (`Ctrl+K`): searchable command palette for Git operations, views, projects, tools and GitHub browser actions; keyboard navigation and context-aware availability.
- **Dirty badges** in the project switcher, showing uncommitted changes at a glance.
- [x] **Auto-fetch** in the background every 5 minutes by default; configurable in Settings → Git (off / 1 / 5 / 10 / 15 / 30 / 60 minutes). Applies to the active repository while the app is running. Waits during Git operations, skips while offline, avoids overlapping syncs and backs off on errors. Status bar shows last fetch, errors, ahead/behind and a session pause toggle. Successful manual fetch/pull resets the timer.
- [x] **Commit message helpers**: subject-length counter, conventional-commit prefixes.
- **Undo toast** after risky-but-reversible actions (e.g. "Branch deleted — Undo", using the reflog).
- **Drag & drop**: drag a branch onto another to merge or rebase.
- **First-run screen**: open, clone, or pick from recently detected repos in `~/projects`, `~/code` and similar folders.

---

### Daily workflow additions

- Repository sections use folder-style tabs at the left. Focus the selected tab and use Left/Right or Home/End to switch sections.
- Issue/PR conversations and comment composers use a centered reading column. “Write a comment” jumps to the composer; “Quote reply” adds a quoted response. Drafts survive switching threads/views within the session. Use the formatting toolbar or Ctrl+B / Ctrl+I inside the composer; Ctrl+Enter posts the comment. Refresh discussions in place from their header, and filter PR files by path.
- Press `Ctrl+K` or the toolbar search button to find an action. GitHub browser actions appear for detected GitHub repositories; creating a PR opens the comparison page for the current branch (push the branch first).
- Click the last-fetch text in the bottom status bar to fetch immediately or retry a failure. Click the auto-fetch interval to pause/resume for this session; change the persisted interval in Settings → Git.
- In Changes, filter by path to operate on matching files. Bulk stage/unstage/discard buttons explicitly target the filtered list.
- Commit messages and conflict resolutions are retained in memory when switching projects/views during the session. Closing the app clears these drafts.
- In the conflict editor, `Ctrl+S` saves and stages a fully resolved file. “After saving, open the next conflicted file” is enabled by default and can be turned off.

---

## Performance targets

- Startup to usable window: < 1 s
- Open a repo with 100k commits: first screen < 500 ms (load history incrementally)
- Scrolling the commit graph: smooth at 60 fps (render visible rows only)
- The UI thread never blocks on git operations

---

## Rules for the agent

- Work **phase by phase**. Finish and verify each task before starting the next.
- After each task: run `cargo fmt`, `cargo clippy`, `npm run check` and the tests; fix all warnings.
- Write unit tests for the git layer and the graph lane algorithm, using temporary test repos created in the tests.
- Never run destructive git commands on the user's real repositories during development; use test repos.
- Never log or store credentials or tokens.
- Keep the Rust ↔ frontend API typed (Tauri commands with typed payloads, shared TS types).
- Keep components small; use one Svelte store for the current repo state.
- Ask before adding large new dependencies.
- Update this file's checkboxes as tasks are completed.
