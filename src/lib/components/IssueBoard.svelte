<script lang="ts">
  import { untrack } from 'svelte'
  import { relativeIso } from '../format'
  import { buildBoard, columns, type BoardItem, type Column } from '../issueBoard'
  import { detail, forge, issues } from '../stores/forge.svelte'
  import { board } from '../stores/issueBoard.svelte'
  import { repo } from '../stores/repo.svelte'
  import { ui } from '../stores/ui.svelte'
  import ConnectPanel from './ConnectPanel.svelte'
  import Icon from './Icon.svelte'
  import IssueDetailView from './IssueDetailView.svelte'
  import IssueViewSwitch from './IssueViewSwitch.svelte'
  import LabelChip from './LabelChip.svelte'
  import StateIcon from './StateIcon.svelte'

  const CLOSED_SHOWN = 15
  let query = $state('')
  let showAllClosed = $state(false)

  $effect(() => {
    if (forge.ready && !board.loaded && !board.loading && !board.error) untrack(() => board.load())
  })

  const branchNames = $derived([
    ...repo.refs.local.map((b) => b.name),
    ...repo.refs.remote.map((b) => b.name.replace(/^[^/]+\//, '')),
  ])
  const items = $derived(buildBoard(board.issues, board.pulls, repo.commits, branchNames))
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase().replace(/^#/, '')
    if (!q) return items
    return items.filter(
      ({ issue: i }) =>
        String(i.number) === q ||
        i.title.toLowerCase().includes(q) ||
        i.assignees.some((a) => a.toLowerCase().includes(q)) ||
        i.labels.some((l) => l.name.toLowerCase().includes(q)),
    )
  })
  const byColumn = (c: Column) => shown.filter((i) => i.column === c)
  const selected = $derived(items.find((i) => i.issue.number === issues.selected) ?? null)
  const label = (c: Column) => columns.find((x) => x.id === c)!.label
  const dot: Record<Column, string> = {
    open: 'bg-add',
    progress: 'bg-accent',
    review: 'bg-[#8250df]',
    closed: 'bg-faint',
  }

  function pick(n: number) {
    void detail.openIssue(n)
  }
  function closeDrawer() {
    detail.clear()
    issues.selected = null
  }
  function openPull(n: number) {
    ui.view = 'pulls'
    void detail.openPull(n)
  }
  function showCommit(id: string) {
    ui.view = 'history'
    void repo.revealCommit(id)
  }
</script>

{#snippet meta(it: BoardItem)}
  {#if it.links.prs.length || it.links.commits.length || it.links.branches.length}
    <span class="mt-1.5 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-[11.5px] text-muted">
      {#each it.links.prs.slice(0, 2) as p (p.number)}
        <span class="inline-flex items-center gap-1" title="Pull request #{p.number}: {p.title}">
          <StateIcon kind="pr" state={p.state} draft={p.draft} size={13} />#{p.number}
        </span>
      {/each}
      {#if it.links.branches.length}
        <span class="inline-flex items-center gap-1" title={it.links.branches.join('\n')}
          ><Icon name="branch" size={12} />{it.links.branches.length}</span
        >
      {/if}
      {#if it.links.commits.length}
        <span class="inline-flex items-center gap-1" title="Commits that mention this issue"
          ><Icon name="commit" size={12} />{it.links.commits.length}</span
        >
      {/if}
    </span>
  {/if}
{/snippet}

<div class="flex h-full min-h-0">
  <div class="flex min-w-0 flex-1 flex-col">
    <div class="flex shrink-0 flex-wrap items-center gap-2 border-b border-border px-3 py-2">
      <IssueViewSwitch />
      <div class="relative w-64 max-w-full">
        <Icon
          name="search"
          size={14}
          class="pointer-events-none absolute top-2.5 left-2.5 text-faint"
        />
        <input
          class="input h-8 pl-8"
          placeholder="Filter by title, #number, label or assignee…"
          aria-label="Filter issues"
          bind:value={query}
          spellcheck="false"
        />
      </div>
      <span class="text-[12px] text-muted">
        {shown.length} issue{shown.length === 1 ? '' : 's'}{board.truncated
          ? ' · showing the 150 most recently updated'
          : ''}
      </span>
      <button
        class="icon-btn ml-auto"
        title="Reload"
        disabled={board.loading}
        onclick={() => board.load()}
      >
        <Icon name="refresh" size={14} class={board.loading ? 'animate-spin' : ''} />
      </button>
    </div>

    {#if board.error}
      <div class="overflow-y-auto">
        <p class="p-4 text-[12.5px] text-del">{board.error}</p>
        {#if forge.status && !forge.status.connected}<ConnectPanel />{/if}
      </div>
    {:else if !board.loaded}
      <div class="grid flex-1 place-items-center text-muted">Loading issues…</div>
    {:else if ui.issuesView === 'board'}
      <div class="flex min-h-0 flex-1 gap-3 overflow-x-auto p-3">
        {#each columns as col (col.id)}
          {@const all = byColumn(col.id)}
          {@const list = col.id === 'closed' && !showAllClosed ? all.slice(0, CLOSED_SHOWN) : all}
          <section class="flex w-72 shrink-0 flex-col rounded-lg bg-surface-2 xl:w-80">
            <header class="flex items-center gap-2 px-3 py-2.5" title={col.hint}>
              <span class="size-2.5 rounded-full {dot[col.id]}"></span>
              <h3 class="text-[12.5px] font-semibold">{col.label}</h3>
              <span class="chip">{all.length}</span>
            </header>
            <ul class="min-h-0 flex-1 space-y-2 overflow-y-auto px-2 pb-2">
              {#each list as it (it.issue.number)}
                <li>
                  <button
                    class="w-full rounded-md border bg-surface p-2.5 text-left shadow-sm hover:border-accent {issues.selected ===
                    it.issue.number
                      ? 'border-accent'
                      : 'border-border'}"
                    onclick={() => pick(it.issue.number)}
                  >
                    <span class="flex items-start gap-2">
                      <span class="mt-0.5"><StateIcon kind="issue" state={it.issue.state} /></span>
                      <span class="line-clamp-3 min-w-0 flex-1 text-[13px] leading-snug font-medium"
                        >{it.issue.title}</span
                      >
                    </span>
                    <span class="mt-1 flex items-center gap-1.5 text-[11.5px] text-muted">
                      <span>#{it.issue.number}</span>
                      {#if it.issue.assignees.length}
                        <span>·</span>
                        <span class="truncate">{it.issue.assignees.join(', ')}</span>
                      {/if}
                      {#if it.issue.comments}
                        <span class="ml-auto inline-flex items-center gap-1"
                          ><Icon name="message" size={11} />{it.issue.comments}</span
                        >
                      {/if}
                    </span>
                    {#if it.issue.labels.length}
                      <span class="mt-1.5 flex flex-wrap gap-1">
                        {#each it.issue.labels.slice(0, 3) as l (l.name)}<LabelChip
                            label={l}
                          />{/each}
                      </span>
                    {/if}
                    {@render meta(it)}
                  </button>
                </li>
              {/each}
              {#if !all.length}
                <li class="px-2 py-4 text-center text-[12px] text-faint">Nothing here</li>
              {/if}
              {#if col.id === 'closed' && all.length > CLOSED_SHOWN}
                <li class="text-center">
                  <button
                    class="text-[12px] text-accent underline"
                    onclick={() => (showAllClosed = !showAllClosed)}
                    >{showAllClosed ? 'Show fewer' : `Show all ${all.length}`}</button
                  >
                </li>
              {/if}
            </ul>
          </section>
        {/each}
      </div>
    {:else}
      <div class="min-h-0 flex-1 overflow-auto">
        <table class="w-full text-[12.5px]">
          <thead class="sticky top-0 bg-surface-2 text-left">
            <tr class="section-label border-b border-border">
              <th class="px-3 py-2 font-semibold">Status</th>
              <th class="px-2 py-2 font-semibold">#</th>
              <th class="px-2 py-2 font-semibold">Title</th>
              <th class="px-2 py-2 font-semibold">Labels</th>
              <th class="px-2 py-2 font-semibold">Assignees</th>
              <th class="px-2 py-2 font-semibold">Pull requests</th>
              <th class="px-2 py-2 font-semibold">Branches</th>
              <th class="px-2 py-2 font-semibold">Commits</th>
              <th class="px-3 py-2 text-right font-semibold">Updated</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-border">
            {#each shown as it (it.issue.number)}
              <tr
                class="cursor-pointer {issues.selected === it.issue.number
                  ? 'bg-accent/10'
                  : 'hover:bg-hover'}"
                onclick={() => pick(it.issue.number)}
              >
                <td class="px-3 py-2 whitespace-nowrap">
                  <span class="inline-flex items-center gap-1.5">
                    <span class="size-2 rounded-full {dot[it.column]}"></span>{label(it.column)}
                  </span>
                </td>
                <td class="px-2 py-2 text-muted">#{it.issue.number}</td>
                <td class="max-w-md truncate px-2 py-2 font-medium" title={it.issue.title}
                  >{it.issue.title}</td
                >
                <td class="px-2 py-2">
                  <span class="flex gap-1"
                    >{#each it.issue.labels.slice(0, 2) as l (l.name)}<LabelChip
                        label={l}
                      />{/each}</span
                  >
                </td>
                <td class="px-2 py-2 text-muted">{it.issue.assignees.join(', ') || '–'}</td>
                <td class="px-2 py-2 text-muted">
                  {it.links.prs.map((p) => `#${p.number}`).join(', ') || '–'}
                </td>
                <td class="max-w-40 truncate px-2 py-2 font-mono text-[11.5px] text-muted">
                  {it.links.branches.join(', ') || '–'}
                </td>
                <td class="px-2 py-2 text-muted">{it.links.commits.length || '–'}</td>
                <td class="px-3 py-2 text-right whitespace-nowrap text-muted"
                  >{relativeIso(it.issue.updated_at)}</td
                >
              </tr>
            {/each}
          </tbody>
        </table>
        {#if !shown.length}
          <p class="p-6 text-center text-muted">No issues match.</p>
        {/if}
      </div>
    {/if}
    <p class="shrink-0 border-t border-border px-3 py-1.5 text-[11px] text-faint">
      Columns are worked out from your repository: a branch, commit or draft pull request that
      mentions an issue (<span class="font-mono">#12</span>, or <span class="font-mono">12</span> in a
      branch name) moves it to In progress; an open pull request moves it to In review. Commits are matched
      in the history loaded so far.
    </p>
  </div>

  {#if selected}
    <aside class="flex w-[28rem] max-w-[50%] shrink-0 flex-col border-l border-border xl:w-[34rem]">
      <div class="flex shrink-0 items-center gap-2 border-b border-border bg-surface-2 px-3 py-2">
        <span class="size-2.5 rounded-full {dot[selected.column]}"></span>
        <span class="text-[12.5px] font-semibold">{label(selected.column)}</span>
        <button class="icon-btn icon-btn-sm ml-auto" title="Close" onclick={closeDrawer}
          ><Icon name="x" size={14} /></button
        >
      </div>
      {#if selected.links.prs.length || selected.links.branches.length || selected.links.commits.length}
        <div class="max-h-44 shrink-0 space-y-1 overflow-y-auto border-b border-border px-4 py-2.5">
          <div class="section-label">Related</div>
          {#each selected.links.prs as p (p.number)}
            <button
              class="flex w-full items-center gap-2 text-left text-[12.5px] hover:text-accent"
              onclick={() => openPull(p.number)}
            >
              <StateIcon kind="pr" state={p.state} draft={p.draft} size={14} />
              <span class="truncate">#{p.number} {p.title}</span>
            </button>
          {/each}
          {#each selected.links.branches as b (b)}
            <div class="flex items-center gap-2 text-[12.5px]">
              <Icon name="branch" size={14} class="text-muted" />
              <span class="truncate font-mono text-[11.5px]">{b}</span>
            </div>
          {/each}
          {#each selected.links.commits.slice(0, 20) as c (c.id)}
            <button
              class="flex w-full items-center gap-2 text-left text-[12.5px] hover:text-accent"
              title="Show in the history"
              onclick={() => showCommit(c.id)}
            >
              <Icon name="commit" size={14} class="shrink-0 text-muted" />
              <span class="shrink-0 font-mono text-[11.5px] text-faint">{c.short_id}</span>
              <span class="truncate">{c.summary}</span>
            </button>
          {/each}
        </div>
      {/if}
      <div class="min-h-0 flex-1">
        {#if detail.loading}
          <div class="grid h-full place-items-center text-muted">Loading…</div>
        {:else if detail.error}
          <div class="grid h-full place-items-center px-6 text-center text-del">{detail.error}</div>
        {:else if detail.issue}
          <IssueDetailView />
        {/if}
      </div>
    </aside>
  {/if}
</div>
