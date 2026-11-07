<script lang="ts">
  import { changes } from '../stores/changes.svelte'
  import { prefs } from '../stores/prefs.svelte'
  import { projects } from '../stores/projects.svelte'
  import { repo } from '../stores/repo.svelte'
  import { operations } from '../stores/operations.svelte'
  import { opLabel, sync } from '../stores/sync.svelte'
  import { ui } from '../stores/ui.svelte'
  import type { SyncOp } from '../types'
  import Icon from './Icon.svelte'

  const s = $derived(repo.summary)
  const disabled = $derived(!projects.current || sync.running !== null)
</script>

{#snippet action(
  op: SyncOp,
  icon: 'fetch' | 'pull' | 'push',
  label: string,
  badge: string,
  title: string,
)}
  <button
    class="btn {sync.running === op ? 'bg-accent/10 text-accent' : ''}"
    {title}
    {disabled}
    onclick={() => sync.run(op)}
  >
    <Icon name={icon} size={15} class={sync.running === op ? 'animate-pulse' : 'text-muted'} />
    {sync.running === op ? opLabel[op] + '…' : label}
    {#if badge}<span class="chip chip-accent">{badge}</span>{/if}
  </button>
{/snippet}

<div class="flex items-center gap-1">
  <button
    class="btn h-8 gap-2 font-semibold"
    title="Switch or create branch (Ctrl+B)"
    onclick={() => (ui.branchSwitcherOpen = true)}
  >
    <Icon name="branch" class="text-accent" />
    {s?.branch ?? '…'}
    <Icon name="chevron-down" size={14} class="text-faint" />
  </button>
  <div class="mx-1 h-5 w-px bg-border"></div>
  {@render action(
    'fetch',
    'fetch',
    'Fetch',
    '',
    'Download new commits from all remotes (does not change your files)',
  )}
  <div class="flex items-center">
    {@render action(
      'pull',
      'pull',
      'Pull',
      s?.behind ? `${s.behind}` : '',
      s?.behind ? `${s.behind} commit(s) to pull` : 'Fetch and bring in the upstream branch',
    )}
    <button
      class="icon-btn -ml-1 h-7 w-5 rounded-l-none"
      title="Pull options: merge, rebase, fast-forward only, stash local changes"
      {disabled}
      onclick={(e) => {
        const r = e.currentTarget.getBoundingClientRect()
        ui.pullMenu = { x: r.left - 120, y: r.bottom + 4 }
      }}
    >
      <Icon name="chevron-down" size={12} />
    </button>
    {#if prefs.pullMode !== 'default' || prefs.pullAutostash}
      <span
        class="chip ml-0.5 normal-case"
        title="Pull will {prefs.pullMode === 'default'
          ? 'follow your git config'
          : prefs.pullMode}{prefs.pullAutostash ? ' and stash local changes' : ''}"
        >{prefs.pullMode === 'default' ? 'stash' : prefs.pullMode}</span
      >
    {/if}
  </div>
  {@render action(
    'push',
    'push',
    'Push',
    s?.ahead ? `${s.ahead}` : s && !s.has_upstream ? 'new' : '',
    s?.ahead
      ? `${s.ahead} commit(s) to push`
      : s && !s.has_upstream
        ? 'Publish this branch'
        : 'Upload your commits',
  )}
  <button
    class="btn"
    title="Bring another branch into {s?.branch ??
      'this branch'}: merge or rebase (e.g. get the latest main)"
    {disabled}
    onclick={() => operations.openDialog(null)}
  >
    <Icon name="commit" size={15} class="text-muted" /> Merge
  </button>
  <div class="mx-1 h-5 w-px bg-border"></div>
  <button
    class="btn"
    title="Put your uncommitted changes aside (stash)"
    disabled={!projects.current}
    onclick={() => (ui.stashDialogOpen = true)}
  >
    <Icon name="stash" size={15} class="text-muted" /> Stash
    {#if changes.total}<span class="chip">{changes.total}</span>{/if}
  </button>
</div>
