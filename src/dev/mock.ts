/**
 * Dev-only fake backend so the UI can be viewed in a plain browser (no Tauri):
 *   npm run dev  ->  http://localhost:1420/?mock=history&theme=dark
 * `mock` = welcome | history | changes | switcher | branch | clone | toast
 * Never bundled in production builds (guarded by import.meta.env.DEV in main.ts).
 */
import { changes } from '../lib/stores/changes.svelte'
import { clone } from '../lib/stores/clone.svelte'
import { projects } from '../lib/stores/projects.svelte'
import { repo } from '../lib/stores/repo.svelte'
import { sync } from '../lib/stores/sync.svelte'
import { theme } from '../lib/stores/theme.svelte'
import type { CommitInfo, FileDiff, Project } from '../lib/types'

const now = Math.floor(Date.now() / 1000)
const P: Project[] = [
  { path: '/home/david/projects/whittle', name: 'whittle', starred: true, last_used: now - 600 },
  {
    path: '/home/david/projects/api-gateway',
    name: 'api-gateway',
    starred: true,
    last_used: now - 86400,
  },
  {
    path: '/home/david/projects/dotfiles',
    name: 'dotfiles',
    starred: false,
    last_used: now - 3 * 86400,
  },
  {
    path: '/home/david/work/billing-service',
    name: 'billing-service',
    starred: false,
    last_used: now - 9 * 86400,
  },
]

const subjects = [
  'Add auto-refresh when files change on disk',
  'Fix diff gutter alignment for long lines',
  'Clone dialog: derive folder name from URL',
  'Refactor sync store to coalesce progress events',
  'Show ahead/behind counts on Pull and Push',
  'Reject URLs that embed credentials',
  'Virtualize commit list for 100k commits',
  'Make branch switcher fuzzy search',
  'Cache sorted revwalk ids between pages',
  'Handle detached HEAD in summary',
  'Stage deletions with git add -A',
  'Update dependencies',
  'Initial commit',
]
const authors = ['David Brandão', 'Ana Ferreira', 'Miguel Santos', 'Sofia Lima']
// A believable history: main with feature branches that fork and merge back.
const shape: { n: number; parents: number[] }[] = [
  { n: 0, parents: [1, 4] }, // merge feature/a into main
  { n: 1, parents: [2] },
  { n: 2, parents: [3, 6] }, // merge feature/b
  { n: 3, parents: [8] },
  { n: 4, parents: [5] }, // feature/a
  { n: 5, parents: [8] },
  { n: 6, parents: [7] }, // feature/b
  { n: 7, parents: [8] },
  { n: 8, parents: [9] },
  { n: 9, parents: [10, 12] },
  { n: 10, parents: [11] },
  { n: 11, parents: [14] },
  { n: 12, parents: [13] },
  { n: 13, parents: [14] },
  { n: 14, parents: [15] },
  { n: 15, parents: [] },
]
const hashOf = (i: number) => (0xabc000 + i * 7919).toString(16).padStart(7, '0') + 'f'.repeat(33)
const commits: CommitInfo[] = shape.map(({ n, parents }) => {
  const id = hashOf(n)
  return {
    id,
    short_id: id.slice(0, 7),
    summary: subjects[n % subjects.length],
    author: authors[n % authors.length],
    email: 'dev@example.com',
    time: now - n * 5400 - (n > 6 ? n * 40000 : 0),
    parents: parents.map(hashOf),
    signed: n === 0 || n === 2,
  }
})

const diff: FileDiff = {
  binary: false,
  hunks: [
    {
      id: 'h1',
      old_start: 12,
      new_start: 12,
      header: '@@ -12,9 +12,14 @@ export class Theme {',
      lines: [
        { kind: ' ', old_no: 12, new_no: 12, text: '  choice = $state<ThemeChoice>(load())' },
        { kind: ' ', old_no: 13, new_no: 13, text: '' },
        { kind: '-', old_no: 14, new_no: null, text: '  get dark() {' },
        { kind: '-', old_no: 15, new_no: null, text: "    return this.choice === 'dark'" },
        { kind: '+', old_no: null, new_no: 14, text: '  get dark() {' },
        { kind: '+', old_no: null, new_no: 15, text: "    return this.choice === 'dark' ||" },
        {
          kind: '+',
          old_no: null,
          new_no: 16,
          text: "      (this.choice === 'system' && media.matches)",
        },
        { kind: ' ', old_no: 16, new_no: 17, text: '  }' },
        { kind: ' ', old_no: 17, new_no: 18, text: '' },
        { kind: ' ', old_no: 18, new_no: 19, text: '  set(choice: ThemeChoice) {' },
      ],
    },
    {
      id: 'h2',
      old_start: 40,
      new_start: 45,
      header: '@@ -40,6 +45,7 @@ export class Theme {',
      lines: [
        { kind: ' ', old_no: 40, new_no: 45, text: '  init() {' },
        {
          kind: '+',
          old_no: null,
          new_no: 46,
          text: '    media.addEventListener("change", () => this.apply())',
        },
        { kind: ' ', old_no: 41, new_no: 47, text: '    this.apply()' },
        { kind: ' ', old_no: 42, new_no: 48, text: '  }' },
      ],
    },
  ],
}

const f = (path: string, status: string, old_path: string | null = null) => ({
  path,
  status,
  old_path,
})
const status = {
  staged: [
    f('src/lib/stores/theme.svelte.ts', 'modified'),
    f('src/lib/components/Icon.svelte', 'added'),
  ],
  unstaged: [
    f('src/App.svelte', 'modified'),
    f('src/lib/components/Sidebar.svelte', 'modified'),
    f('brand/logo.png', 'untracked'),
  ],
  conflicted: [] as string[],
}

const conflictText = [
  'export const api = {',
  "  listProjects: () => invoke<Project[]>('list_projects'),",
  '<<<<<<< HEAD',
  "  openProject: (path: string) => invoke<Project>('open_project', { path }),",
  '=======',
  '  openProject: (path: string, touch = true) =>',
  "    invoke<Project>('open_project', { path, touch }),",
  '>>>>>>> feature/commit-graph',
  "  removeProject: (path: string) => invoke<void>('remove_project', { path }),",
  '}',
].join('\n')

const handlers: Record<string, (a: Record<string, unknown>) => unknown> = {
  list_projects: () => P,
  open_project: (a) => P.find((p) => p.path === a.path) ?? P[0],
  repo_summary: () => ({ branch: 'main', dirty: true, ahead: 2, behind: 3, has_upstream: true }),
  repo_refs: () => ({
    local: [
      { name: 'main', is_head: true, upstream: 'origin/main' },
      { name: 'feature/commit-graph', is_head: false, upstream: null },
      { name: 'fix/diff-scroll', is_head: false, upstream: 'origin/fix/diff-scroll' },
      { name: 'release/0.1', is_head: false, upstream: null },
    ],
    remote: [
      { name: 'origin/main', is_head: false, upstream: null },
      { name: 'origin/fix/diff-scroll', is_head: false, upstream: null },
    ],
    tags: ['v0.1.0', 'v0.2.0'],
    head_id: hashOf(1),
    labels: [
      {
        id: hashOf(0),
        name: 'origin/main',
        kind: 'remote',
        current: false,
        tracked_by_current: true,
      },
      {
        id: hashOf(0),
        name: 'origin/fix/diff-scroll',
        kind: 'remote',
        current: false,
        tracked_by_current: false,
      },
      { id: hashOf(1), name: 'main', kind: 'local', current: true, tracked_by_current: false },
      {
        id: hashOf(4),
        name: 'feature/commit-graph',
        kind: 'local',
        current: false,
        tracked_by_current: false,
      },
      {
        id: hashOf(6),
        name: 'fix/diff-scroll',
        kind: 'local',
        current: false,
        tracked_by_current: false,
      },
      { id: hashOf(8), name: 'v0.2.0', kind: 'tag', current: false, tracked_by_current: false },
      {
        id: hashOf(8),
        name: 'release/0.1',
        kind: 'local',
        current: false,
        tracked_by_current: false,
      },
    ],
    stashes: [
      {
        index: 0,
        message: 'WIP on main: 3f2a9c1 sidebar spacing',
        id: 'a'.repeat(40),
        time: now - 7200,
      },
    ],
  }),
  repo_log: (a) => commits.slice(Number(a.skip), Number(a.skip) + Number(a.limit)),
  commit_files: () => [
    f('src/lib/stores/theme.svelte.ts', 'modified'),
    f('src/lib/components/Icon.svelte', 'added'),
    f('src/old/Legacy.svelte', 'deleted'),
    f('src/lib/format.ts', 'renamed', 'src/lib/fmt.ts'),
  ],
  commit_file_diff: () => diff,
  work_status: () =>
    new URLSearchParams(location.search).get('mock') === 'conflict'
      ? { ...status, conflicted: ['src/lib/api.ts', 'README.md'], operation: 'merge' }
      : status,
  read_file_text: () => conflictText,
  work_file_diff: () => diff,
  head_message: () => 'Previous commit message',
  search_commits: (a) => {
    const terms = String(a.query).toLowerCase().split(/\s+/).filter(Boolean)
    return commits.filter((c) => {
      const hay = (c.summary + c.author + c.short_id).toLowerCase()
      return terms.every((t) => hay.includes(t))
    })
  },
  merge_preview: (a) => ({
    source: String(a.source),
    target: 'main',
    incoming: 3,
    only_here: 2,
    up_to_date: false,
    can_fast_forward: false,
    commits: commits.slice(0, 3),
    files: 7,
    conflicts: ['src/lib/api.ts', 'src/App.svelte'],
  }),
  commit_signature: () => ({
    signed: true,
    status: 'good',
    signer: 'david@example.com',
    key: 'SHA256:abc',
  }),
  file_blame: () => ({
    lines: [
      "import { invoke } from '@tauri-apps/api/core'",
      '',
      'export const api = {',
      "  listProjects: () => invoke<Project[]>('list_projects'),",
      "  openProject: (path: string) => invoke<Project>('open_project', { path }),",
      '}',
    ],
    hunks: [
      {
        start: 1,
        count: 3,
        id: hashOf(6),
        short_id: hashOf(6).slice(0, 7),
        author: 'Ana Ferreira',
        time: now - 90 * 86400,
        summary: 'Initial api wrapper',
      },
      {
        start: 4,
        count: 1,
        id: hashOf(2),
        short_id: hashOf(2).slice(0, 7),
        author: 'David Brandão',
        time: now - 3 * 86400,
        summary: 'Add list projects',
      },
      {
        start: 5,
        count: 2,
        id: hashOf(0),
        short_id: hashOf(0).slice(0, 7),
        author: 'Miguel Santos',
        time: now - 3600,
        summary: 'Validate repo on open',
      },
    ],
  }),
  file_history: () => commits.slice(0, 5),
  repo_submodules: () => [
    {
      name: 'libs/ui',
      path: 'libs/ui',
      url: 'git@github.com:org/ui.git',
      status: 'clean',
      expected_id: 'a'.repeat(40),
      current_id: 'a'.repeat(40),
    },
    {
      name: 'vendor/proto',
      path: 'vendor/proto',
      url: null,
      status: 'new_commits',
      expected_id: 'b'.repeat(40),
      current_id: 'c'.repeat(40),
    },
  ],
  rebase_plan: () => commits.slice(0, 5).reverse(),
  get_settings: () => ({
    clone_dir: null,
    git_path: null,
    editor_cmd: null,
    terminal_cmd: null,
    font_size: 13,
    accent: '#f66008',
  }),
  forge_status: () => {
    const anon = new URLSearchParams(location.search).get('mock') === 'connect'
    return {
      supported: true,
      provider: 'github',
      host: 'github.com',
      owner: 'david',
      repo: 'whittle',
      connected: !anon,
      login: anon ? null : 'david-s-brandao',
      source: anon ? 'none' : 'gh',
      message: null,
    }
  },
  forge_issues: () => ({
    items: [
      {
        number: 42,
        title: 'Commit graph lines overlap when more than 12 branches are open',
        state: 'open',
        author: 'ana-ferreira',
        labels: [
          { name: 'bug', color: 'd73a4a' },
          { name: 'graph', color: '0e8a16' },
        ],
        assignees: ['david-s-brandao'],
        comments: 3,
        created_at: new Date(Date.now() - 2 * 86400e3).toISOString(),
        updated_at: new Date(Date.now() - 3600e3).toISOString(),
        url: 'https://github.com/david/whittle/issues/42',
      },
      {
        number: 41,
        title: 'Add an option to hide the sidebar',
        state: 'open',
        author: 'miguel-santos',
        labels: [{ name: 'enhancement', color: 'a2eeef' }],
        assignees: [],
        comments: 0,
        created_at: new Date(Date.now() - 5 * 86400e3).toISOString(),
        updated_at: new Date(Date.now() - 86400e3).toISOString(),
        url: 'u',
      },
      {
        number: 39,
        title: 'Pull fails with “divergent branches” on a fresh clone',
        state: 'closed',
        author: 'sofia-lima',
        labels: [
          { name: 'bug', color: 'd73a4a' },
          { name: 'good first issue', color: '7057ff' },
          { name: 'needs-repro', color: 'fbca04' },
          { name: 'sync', color: 'c5def5' },
        ],
        assignees: ['ana-ferreira'],
        comments: 7,
        created_at: new Date(Date.now() - 12 * 86400e3).toISOString(),
        updated_at: new Date(Date.now() - 6 * 86400e3).toISOString(),
        url: 'u',
      },
    ],
    next_page: 2,
  }),
  forge_issue: (a) => ({
    summary: {
      number: Number(a.number),
      title: 'Commit graph lines overlap when more than 12 branches are open',
      state: 'open',
      author: 'ana-ferreira',
      labels: [
        { name: 'bug', color: 'd73a4a' },
        { name: 'graph', color: '0e8a16' },
      ],
      assignees: ['david-s-brandao'],
      comments: 2,
      created_at: new Date(Date.now() - 2 * 86400e3).toISOString(),
      updated_at: new Date().toISOString(),
      url: 'https://github.com/david/whittle/issues/42',
    },
    body: '## What happens\n\nWith **13 or more** open branches the lanes are squeezed into the last column, so lines overlap.\n\n### Steps\n1. Open a repo with many branches\n2. Scroll the history\n\n```\nlanes: 14 -> drawn in 12 columns\n```\n\n- [x] reproduced on Linux\n- [ ] reproduced on macOS\n\nSee also https://example.com/lanes and @david-s-brandao.\n\n<!-- template comment -->',
    comments: [
      {
        id: 1,
        author: 'david-s-brandao',
        body: 'Thanks! I can reproduce it. The `MAX_LANES` cap is the cause.',
        created_at: new Date(Date.now() - 86400e3).toISOString(),
        url: 'u',
        association: 'OWNER',
      },
      {
        id: 2,
        author: 'miguel-santos',
        body: 'Could we scroll the graph horizontally instead?',
        created_at: new Date(Date.now() - 3600e3).toISOString(),
        url: 'u',
        association: 'CONTRIBUTOR',
      },
    ],
  }),
  forge_pulls: () => ({
    items: [
      {
        number: 57,
        title: 'Add interactive rebase dialog',
        state: 'open',
        draft: false,
        author: 'ana-ferreira',
        head_ref: 'feature/interactive-rebase',
        base_ref: 'main',
        labels: [{ name: 'enhancement', color: 'a2eeef' }],
        created_at: new Date(Date.now() - 86400e3).toISOString(),
        updated_at: new Date(Date.now() - 1800e3).toISOString(),
        url: 'https://github.com/david/whittle/pull/57',
      },
      {
        number: 55,
        title: 'WIP: tabs for multiple repositories',
        state: 'open',
        draft: true,
        author: 'miguel-santos',
        head_ref: 'feature/tabs',
        base_ref: 'main',
        labels: [],
        created_at: new Date(Date.now() - 4 * 86400e3).toISOString(),
        updated_at: new Date(Date.now() - 2 * 86400e3).toISOString(),
        url: 'u',
      },
      {
        number: 52,
        title: 'Fix watcher startup on huge repos',
        state: 'merged',
        draft: false,
        author: 'david-s-brandao',
        head_ref: 'fix/watcher',
        base_ref: 'main',
        labels: [{ name: 'performance', color: 'fbca04' }],
        created_at: new Date(Date.now() - 9 * 86400e3).toISOString(),
        updated_at: new Date(Date.now() - 8 * 86400e3).toISOString(),
        url: 'u',
      },
    ],
    next_page: null,
  }),
  forge_pull: (a) => ({
    summary: {
      number: Number(a.number),
      title: 'Add interactive rebase dialog',
      state: 'open',
      draft: false,
      author: 'ana-ferreira',
      head_ref: 'feature/interactive-rebase',
      base_ref: 'main',
      labels: [{ name: 'enhancement', color: 'a2eeef' }],
      created_at: new Date(Date.now() - 86400e3).toISOString(),
      updated_at: new Date().toISOString(),
      url: 'https://github.com/david/whittle/pull/57',
    },
    body: 'Adds the **interactive rebase** dialog.\n\n- reorder by dragging\n- squash / fixup / reword / drop\n\nCloses #42.',
    mergeable_state: 'clean',
    additions: 412,
    deletions: 37,
    changed_files: 3,
    commits: 5,
    head_sha: 'abc123',
    comments: [
      {
        id: 11,
        author: 'miguel-santos',
        body: 'Nice work! One question about the squash order.',
        created_at: new Date(Date.now() - 7200e3).toISOString(),
        url: 'u',
        association: 'CONTRIBUTOR',
      },
    ],
    reviews: [
      {
        id: 90,
        author: 'david-s-brandao',
        state: 'CHANGES_REQUESTED',
        body: 'Almost there: please handle the empty plan case and add a test.',
        submitted_at: new Date(Date.now() - 3600e3).toISOString(),
        url: 'u',
      },
    ],
    review_comments: [
      {
        id: 301,
        author: 'david-s-brandao',
        body: 'This should refuse an empty plan instead of silently doing nothing.',
        path: 'src/cli/mod.rs',
        line: 13,
        side: 'RIGHT',
        created_at: new Date(Date.now() - 3600e3).toISOString(),
        in_reply_to: null,
        review_id: 90,
        url: 'u',
        commit_id: 'abc123',
      },
      {
        id: 302,
        author: 'ana-ferreira',
        body: 'Good catch, fixed in the next push.',
        path: 'src/cli/mod.rs',
        line: 13,
        side: 'RIGHT',
        created_at: new Date(Date.now() - 1800e3).toISOString(),
        in_reply_to: 301,
        review_id: null,
        url: 'u',
        commit_id: 'abc123',
      },
    ],
  }),
  forge_pull_files: () => [
    {
      filename: 'src/cli/mod.rs',
      previous_filename: null,
      status: 'modified',
      additions: 18,
      deletions: 3,
      patch:
        '@@ -10,7 +10,9 @@ pub fn rebase_interactive(\n     items: &[RebaseItem],\n ) -> Result<OpResult, CliError> {\n-    let plan = build(items);\n+    if items.is_empty() {\n+        return Err(CliError::Failed("nothing to rebase".into()));\n+    }\n+    let plan = build(items);\n     run(repo, &plan)\n }\n \n',
    },
    {
      filename: 'src/lib/components/InteractiveRebaseDialog.svelte',
      previous_filename: null,
      status: 'added',
      additions: 380,
      deletions: 0,
      patch: '@@ -0,0 +1,2 @@\n+<script lang="ts">\n+</script>',
    },
    {
      filename: 'assets/logo.png',
      previous_filename: null,
      status: 'modified',
      additions: 0,
      deletions: 0,
      patch: null,
    },
  ],
  default_clone_dir: () => '/home/david/projects',
  clone_name: (a) =>
    String(a.url)
      .replace(/\.git$/, '')
      .split(/[/:]/)
      .pop() ?? '',
}

export async function installMock() {
  const w = window as unknown as Record<string, unknown>
  let cb = 1
  w.__TAURI_INTERNALS__ = {
    transformCallback: () => cb++,
    unregisterCallback: () => {},
    invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
      if (cmd.startsWith('plugin:event|')) return cb++
      const h = handlers[cmd]
      return h ? h(args) : null
    },
  }
  w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} }

  const q = new URLSearchParams(location.search)
  const t = q.get('theme')
  if (t === 'dark' || t === 'light' || t === 'system') theme.set(t)
  const scene = q.get('mock') || 'history'

  // wait for the app to mount, then drive it into the requested scene
  await new Promise((r) => setTimeout(r, 150))
  if (scene === 'welcome') return
  await projects.openPath(P[0].path)
  await repo.refresh()
  await changes.refresh()
  await new Promise((r) => setTimeout(r, 100))
  const click = (text: string) =>
    [...document.querySelectorAll<HTMLElement>('button')]
      .find((b) => b.textContent?.trim().startsWith(text))
      ?.click()
  if (scene === 'history') {
    await repo.selectCommit(commits[0])
  } else if (scene === 'changes') {
    click('Changes')
    await changes.select('src/App.svelte', false)
  } else if (scene === 'linesel') {
    click('Changes')
    await changes.select('src/App.svelte', false)
    await new Promise((r) => setTimeout(r, 100))
    const boxes = document.querySelectorAll<HTMLInputElement>('input[aria-label="Select line"]')
    boxes[1]?.click()
    boxes[3]?.click()
  } else if (scene === 'search') {
    const input = document.querySelector<HTMLInputElement>('input[placeholder^="Search commits"]')
    if (input) {
      input.value = 'ana fix'
      input.dispatchEvent(new Event('input', { bubbles: true }))
    }
    await new Promise((r) => setTimeout(r, 600))
    await repo.selectCommit(commits[1])
  } else if (scene === 'commitmenu') {
    await repo.refresh()
    await new Promise((r) => setTimeout(r, 200))
    document
      .querySelectorAll<HTMLElement>('[role="option"]')[3]
      ?.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 420, clientY: 230 }))
  } else if (scene === 'reset') {
    await repo.refresh()
    const { ui } = await import('../lib/stores/ui.svelte')
    ui.resetDialog = commits[3]
  } else if (scene === 'blame') {
    await repo.refresh()
    const { ui } = await import('../lib/stores/ui.svelte')
    ui.fileView = { path: 'src/lib/api.ts', rev: null, tab: 'blame' }
  } else if (scene === 'irebase') {
    await repo.refresh()
    const { ui } = await import('../lib/stores/ui.svelte')
    ui.interactiveRebase = commits[4]
    await new Promise((r) => setTimeout(r, 300))
    const sels = document.querySelectorAll<HTMLSelectElement>('select')
    const set = (i: number, v: string) => {
      if (sels[i]) {
        sels[i].value = v
        sels[i].dispatchEvent(new Event('change', { bubbles: true }))
      }
    }
    set(1, 'squash')
    set(3, 'reword')
    set(4, 'drop')
  } else if (scene === 'settings') {
    const { ui } = await import('../lib/stores/ui.svelte')
    ui.settingsOpen = true
  } else if (scene === 'tabs') {
    const { tabs } = await import('../lib/stores/tabs.svelte')
    tabs.track(P[1].path)
    tabs.track(P[2].path)
    tabs.track(P[0].path)
  } else if (['issues', 'issue', 'pulls', 'pull', 'prfiles', 'connect', 'review'].includes(scene)) {
    const { ui } = await import('../lib/stores/ui.svelte')
    const { detail } = await import('../lib/stores/forge.svelte')
    ui.view = scene === 'issues' || scene === 'issue' || scene === 'connect' ? 'issues' : 'pulls'
    await new Promise((r) => setTimeout(r, 500))
    if (scene === 'issue') await detail.openIssue(42)
    if (scene === 'pull' || scene === 'prfiles' || scene === 'review') await detail.openPull(57)
    await new Promise((r) => setTimeout(r, 200))
    if (scene === 'prfiles') {
      click('Files changed')
      await new Promise((r) => setTimeout(r, 300))
    }
    if (scene === 'review') click('Review changes')
  } else if (scene === 'pullmenu') {
    ;[...document.querySelectorAll<HTMLElement>('header button')]
      .find((b) => b.title.startsWith('Pull options'))
      ?.click()
  } else if (scene === 'merge') {
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'p', ctrlKey: false }))
    ;[...document.querySelectorAll<HTMLElement>('header button')]
      .find((b) => b.textContent?.trim().startsWith('Merge'))
      ?.click()
  } else if (scene === 'shortcuts') {
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '?' }))
  } else if (scene === 'conflict') {
    click('Changes')
    await changes.selectConflict('src/lib/api.ts')
  } else if (scene === 'stash') {
    ;[...document.querySelectorAll<HTMLElement>('header button')]
      .find((b) => b.textContent?.includes('Stash'))
      ?.click()
  } else if (scene === 'switcher') {
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'p', ctrlKey: true }))
  } else if (scene === 'branch') {
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'b', ctrlKey: true }))
  } else if (scene === 'clone') {
    await clone.show()
    await clone.setUrl('git@github.com:owner/awesome-project.git')
  } else if (scene === 'toast') {
    await repo.selectCommit(commits[0])
    sync.running = 'push'
    sync.percent = 62
    sync.line = 'Writing objects:  62% (31/50), 12.4 KiB | 4.1 MiB/s'
    sync.error = {
      op: 'pull',
      kind: 'auth_ssh',
      message:
        'The server rejected your SSH key. Check that your key is loaded (run `ssh-add -l`) and that its public key is added to your account on the server.',
      details: 'git@github.com: Permission denied (publickey).',
    }
  }
}
