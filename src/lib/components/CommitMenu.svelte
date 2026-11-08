<script lang="ts">
  import { commits } from '../stores/commits.svelte'
  import { repo } from '../stores/repo.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon, { type IconName } from './Icon.svelte'

  const m = $derived(ui.commitMenu)
  const current = $derived(repo.summary?.branch ?? 'the current branch')
  const isHead = $derived(!!m && m.commit.id === repo.refs.head_id)

  function close() {
    ui.commitMenu = null
  }
  // Run the action first: its closure reads the menu's target, which close() clears.
  function run(fn: () => void) {
    try {
      fn()
    } finally {
      close()
    }
  }
  const left = $derived(m ? Math.max(8, Math.min(m.x, window.innerWidth - 310)) : 0)
  const top = $derived(m ? Math.max(8, Math.min(m.y, window.innerHeight - 380)) : 0)
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

{#snippet item(icon: IconName, title: string, hint: string, fn: () => void, danger = false)}
  <button
    role="menuitem"
    class="flex w-full items-start gap-2.5 px-3 py-1.5 text-left hover:bg-hover {danger
      ? 'hover:text-del'
      : ''}"
    onclick={() => run(fn)}
  >
    <Icon name={icon} size={15} class="mt-0.5 shrink-0 text-muted" />
    <span class="min-w-0">
      <span class="block text-[12.5px] font-medium">{title}</span>
      {#if hint}<span class="block text-[11.5px] text-muted">{hint}</span>{/if}
    </span>
  </button>
{/snippet}

{#if m}
  <div
    class="fixed inset-0 z-50"
    role="presentation"
    onmousedown={close}
    oncontextmenu={(e) => {
      e.preventDefault()
      close()
    }}
  >
    <div
      class="absolute w-76 overflow-hidden rounded-lg border border-border bg-surface py-1 shadow-xl"
      style="left: {left}px; top: {top}px; width: 19rem"
      role="menu"
      tabindex="-1"
      onmousedown={(e) => e.stopPropagation()}
    >
      <div class="truncate px-3 py-1.5 text-[11.5px] font-semibold text-muted">
        <span class="font-mono">{m.commit.short_id}</span> · {m.commit.summary}
      </div>
      {@render item(
        'branch',
        'Create branch here…',
        'Start a new branch at this commit',
        () => (ui.nameDialog = { kind: 'branch', commit: m.commit }),
      )}
      {@render item(
        'tag',
        'Create tag here…',
        'Mark this commit, e.g. a release',
        () => (ui.nameDialog = { kind: 'tag', commit: m.commit }),
      )}
      {@render item(
        'check',
        'Check out this commit',
        'Look around without a branch (detached HEAD)',
        () => commits.checkoutDetached(m.commit),
      )}
      <div class="my-1 border-t border-border"></div>
      {#if !isHead}
        {@render item(
          'commit',
          `Cherry-pick onto ${current}`,
          'Copy this commit onto your current branch',
          () => commits.cherryPick(m.commit),
        )}
      {/if}
      {@render item('undo', 'Revert this commit', 'Add a new commit that undoes it', () =>
        commits.revert(m.commit),
      )}
      {@render item(
        'branch',
        'Rebase interactively from here…',
        'Reorder, squash, reword or drop this commit and newer ones',
        () => (ui.interactiveRebase = m.commit),
      )}
      {#if !isHead}
        {@render item(
          'history',
          `Reset ${current} to here…`,
          'Move the branch back (soft, mixed or hard)',
          () => (ui.resetDialog = m.commit),
          true,
        )}
      {/if}
      <div class="my-1 border-t border-border"></div>
      {@render item('copy', 'Copy commit hash', m.commit.id.slice(0, 40), () =>
        navigator.clipboard?.writeText(m.commit.id).catch(() => {}),
      )}
    </div>
  </div>
{/if}
