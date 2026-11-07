<script lang="ts">
  import { branches } from '../stores/branches.svelte'
  import { commits } from '../stores/commits.svelte'
  import { operations } from '../stores/operations.svelte'
  import { repo } from '../stores/repo.svelte'
  import { stashes } from '../stores/stashes.svelte'
  import { submodules } from '../stores/submodules.svelte'
  import { ui } from '../stores/ui.svelte'
  import type { StashInfo, SubmoduleInfo } from '../types'
  import Icon, { type IconName } from './Icon.svelte'

  const sections = $derived([
    { id: 'Branches', icon: 'branch', count: repo.refs.local.length },
    { id: 'Remotes', icon: 'cloud', count: repo.refs.remote.length },
    { id: 'Tags', icon: 'tag', count: repo.refs.tags.length },
    { id: 'Stashes', icon: 'stash', count: repo.refs.stashes.length },
    ...(repo.submodules.length
      ? [{ id: 'Submodules', icon: 'folder', count: repo.submodules.length }]
      : []),
  ] as { id: string; icon: IconName; count: number }[])
  let collapsed = $state<Record<string, boolean>>({ Remotes: false })

  // ---- drag & drop: drop a branch on your current branch to merge it (or drag the current
  // branch onto another to rebase onto it), like Fork's sidebar.
  let dragOver = $state<string | null>(null)

  function dragStart(e: DragEvent, name: string) {
    e.dataTransfer?.setData('text/plain', name)
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
  }
  function drop(e: DragEvent, target: string, targetIsCurrent: boolean) {
    e.preventDefault()
    dragOver = null
    const src = e.dataTransfer?.getData('text/plain')
    if (!src || src === target) return
    const srcIsCurrent = repo.refs.local.some((b) => b.name === src && b.is_head)
    if (targetIsCurrent) operations.openDialog(src, 'merge')
    else if (srcIsCurrent) operations.openDialog(target, 'rebase')
    else
      ui.notice = {
        kind: 'error',
        text: `To combine “${src}” and “${target}”, check one of them out first, then drag the other onto it.`,
      }
  }
</script>

{#snippet header(id: string, count: number)}
  <div class="group flex items-center pr-2 pl-1">
    <button
      class="flex h-7 flex-1 items-center gap-1 rounded-md px-1.5 text-left hover:bg-hover"
      onclick={() => (collapsed[id] = !collapsed[id])}
    >
      <Icon name={collapsed[id] ? 'chevron-right' : 'chevron-down'} size={13} class="text-faint" />
      <span class="section-label">{id}</span>
      <span class="ml-1 text-[11px] text-faint">{count}</span>
    </button>
    {#if id === 'Submodules'}
      <button
        class="btn h-5 px-1.5 text-[11px]"
        title="Download and update every submodule"
        disabled={submodules.busy !== null}
        onclick={() => submodules.update(null)}>Update all</button
      >
    {/if}
    {#if id === 'Stashes'}
      <button
        class="icon-btn icon-btn-sm hover:text-accent"
        title="Stash current changes"
        onclick={() => (ui.stashDialogOpen = true)}><Icon name="plus" size={14} /></button
      >
    {/if}
    {#if id === 'Branches'}
      <button
        class="icon-btn icon-btn-sm hover:text-accent"
        title="Switch or create branch (Ctrl+B)"
        onclick={() => (ui.branchSwitcherOpen = true)}><Icon name="plus" size={14} /></button
      >
    {/if}
  </div>
{/snippet}

{#snippet stashRow(st: StashInfo)}
  <div
    role="presentation"
    class="group mx-2 flex h-7 items-center gap-2 rounded-md px-2.5 text-[12.5px] hover:bg-hover"
    title="{st.message}: click to see what's inside"
  >
    <button
      class="flex min-w-0 flex-1 items-center gap-2 text-left"
      onclick={() => stashes.preview(st)}
    >
      <Icon name="stash" size={14} class="shrink-0 text-faint" />
      <span class="truncate text-muted">{st.message}</span>
    </button>
    <span class="hidden shrink-0 items-center gap-0.5 group-hover:flex">
      <button
        class="btn h-5 px-1.5 text-[11px]"
        title="Apply the changes and keep the stash"
        disabled={stashes.busy}
        onclick={() => stashes.apply(st, false)}>Apply</button
      >
      <button
        class="btn h-5 px-1.5 text-[11px]"
        title="Apply the changes and remove the stash"
        disabled={stashes.busy}
        onclick={() => stashes.apply(st, true)}>Pop</button
      >
      <button
        class="icon-btn icon-btn-sm hover:text-del"
        title="Delete this stash"
        disabled={stashes.busy}
        onclick={() => stashes.drop(st)}><Icon name="trash" size={14} /></button
      >
    </span>
  </div>
{/snippet}

{#snippet submoduleRow(sm: SubmoduleInfo)}
  {@const dot =
    sm.status === 'clean'
      ? 'bg-add'
      : sm.status === 'uninitialized'
        ? 'border border-faint'
        : sm.status === 'dirty'
          ? 'bg-del'
          : 'bg-accent'}
  {@const hint =
    sm.status === 'clean'
      ? 'Up to date'
      : sm.status === 'uninitialized'
        ? 'Not downloaded yet'
        : sm.status === 'dirty'
          ? 'Has uncommitted changes inside'
          : 'Checked out at a different commit than this project records'}
  <div
    role="presentation"
    class="group mx-2 flex h-7 items-center gap-2 rounded-md px-2.5 text-[12.5px] hover:bg-hover"
    title="{sm.path}\n{hint}{sm.url ? `\n${sm.url}` : ''}"
  >
    <span class="size-2 shrink-0 rounded-full {dot}"></span>
    <span class="min-w-0 flex-1 truncate">{sm.path}</span>
    {#if submodules.busy === sm.path || submodules.busy === 'all'}
      <span class="shrink-0 text-[11px] text-accent">Updating…</span>
    {:else if sm.status !== 'clean'}
      <span class="hidden shrink-0 group-hover:flex">
        <button
          class="btn h-5 px-1.5 text-[11px]"
          title="Download and check out the commit this project expects"
          disabled={submodules.busy !== null}
          onclick={() => submodules.update(sm.path)}>Update</button
        >
      </span>
    {/if}
  </div>
{/snippet}

{#snippet tagRow(name: string)}
  <div
    role="presentation"
    class="group mx-2 flex h-7 items-center gap-2 rounded-md px-2.5 text-[12.5px] hover:bg-hover"
    title="{name}: click to find its commit in the history"
  >
    <button
      class="flex min-w-0 flex-1 items-center gap-2 text-left"
      onclick={() => {
        const id = repo.refs.labels.find((l) => l.kind === 'tag' && l.name === name)?.id
        ui.view = 'history'
        if (id) repo.revealCommit(id)
      }}
    >
      <Icon name="tag" size={14} class="shrink-0 text-faint" />
      <span class="truncate">{name}</span>
    </button>
    <span class="hidden shrink-0 items-center gap-0.5 group-hover:flex">
      <button
        class="icon-btn icon-btn-sm hover:text-accent"
        title="Push this tag to the remote"
        disabled={commits.busy}
        onclick={() => commits.pushTag(name, false)}><Icon name="push" size={14} /></button
      >
      <button
        class="icon-btn icon-btn-sm hover:text-del"
        title="Delete this tag locally"
        disabled={commits.busy}
        onclick={() => commits.deleteTag(name)}><Icon name="trash" size={14} /></button
      >
      <button
        class="icon-btn icon-btn-sm hover:text-del"
        title="Delete this tag on the remote"
        disabled={commits.busy}
        onclick={() => commits.deleteRemoteTag(name)}><Icon name="cloud" size={14} /></button
      >
    </span>
  </div>
{/snippet}

{#snippet branchRow(name: string, current: boolean, remote: boolean, upstream = '')}
  <div
    role="presentation"
    draggable="true"
    ondragstart={(e) => dragStart(e, name)}
    ondragover={(e) => {
      e.preventDefault()
      dragOver = name
    }}
    ondragleave={() => dragOver === name && (dragOver = null)}
    ondrop={(e) => drop(e, name, current)}
    class="group mx-2 flex h-7 items-center gap-2 rounded-md px-2.5 text-[12.5px] {dragOver === name
      ? 'ring-2 ring-accent'
      : ''} {current ? 'bg-accent/10 font-medium text-accent' : 'hover:bg-hover'}"
    title={current
      ? `${name} (current branch)${upstream ? ` · tracks ${upstream}` : ''}\nDrag another branch here to merge it`
      : `${name}: double-click to check out, right-click for more, drag onto the current branch to merge`}
    ondblclick={() => !current && branches.checkout(name, remote)}
    oncontextmenu={(e) => {
      e.preventDefault()
      ui.branchMenu = { name, remote, current, x: e.clientX, y: e.clientY }
    }}
  >
    <Icon
      name={remote ? 'cloud' : 'branch'}
      size={14}
      class="shrink-0 {current ? '' : 'text-faint'}"
    />
    <span class="truncate">{name}</span>
    <span class="ml-auto hidden shrink-0 items-center gap-0.5 group-hover:flex">
      {#if !current}
        <button
          class="icon-btn icon-btn-sm hover:text-accent"
          title="Check out"
          disabled={branches.busy}
          onclick={() => branches.checkout(name, remote)}><Icon name="check" size={14} /></button
        >
      {/if}
      <button
        class="icon-btn icon-btn-sm"
        title="More: merge, rebase{remote || current ? '' : ', delete'}…"
        onclick={(e) => {
          const r = e.currentTarget.getBoundingClientRect()
          ui.branchMenu = { name, remote, current, x: r.left, y: r.bottom + 4 }
        }}><Icon name="more" size={14} /></button
      >
    </span>
  </div>
{/snippet}

<nav class="h-full w-60 shrink-0 overflow-y-auto border-r border-border bg-surface-2 py-2">
  {#each sections as s (s.id)}
    <div class="mb-1">
      {@render header(s.id, s.count)}
      {#if !collapsed[s.id]}
        {#if s.id === 'Branches'}
          {#each repo.refs.local as b (b.name)}{@render branchRow(
              b.name,
              b.is_head,
              false,
              b.upstream ?? '',
            )}{/each}
        {:else if s.id === 'Remotes'}
          {#each repo.refs.remote as b (b.name)}{@render branchRow(b.name, false, true)}{/each}
        {:else if s.id === 'Tags'}
          {#each repo.refs.tags as t (t)}{@render tagRow(t)}{/each}
        {:else if s.id === 'Stashes'}
          {#each repo.refs.stashes as st (st.id)}{@render stashRow(st)}{/each}
        {:else if s.id === 'Submodules'}
          {#each repo.submodules as sm (sm.path)}{@render submoduleRow(sm)}{/each}
        {/if}
        {#if s.count === 0}
          <div class="mx-2 px-2.5 py-1 text-[12px] text-faint">None</div>
        {/if}
      {/if}
    </div>
  {/each}
</nav>
