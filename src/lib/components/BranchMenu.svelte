<script lang="ts">
  import { branches } from '../stores/branches.svelte'
  import { operations } from '../stores/operations.svelte'
  import { sync } from '../stores/sync.svelte'
  import { repo } from '../stores/repo.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon, { type IconName } from './Icon.svelte'

  const m = $derived(ui.branchMenu)
  const current = $derived(repo.summary?.branch ?? 'current branch')

  function close() {
    ui.branchMenu = null
  }
  function run(fn: () => void) {
    close()
    fn()
  }
  // keep the menu inside the window
  const left = $derived(m ? Math.max(8, Math.min(m.x, window.innerWidth - 300)) : 0)
  const top = $derived(m ? Math.max(8, Math.min(m.y, window.innerHeight - 220)) : 0)
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

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
      class="absolute w-72 overflow-hidden rounded-lg border border-border bg-surface py-1 shadow-xl"
      style="left: {left}px; top: {top}px"
      role="menu"
      tabindex="-1"
      onmousedown={(e) => e.stopPropagation()}
    >
      <div class="truncate px-3 py-1.5 text-[11.5px] font-semibold text-muted">{m.name}</div>
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
            <span class="block text-[11.5px] text-muted">{hint}</span>
          </span>
        </button>
      {/snippet}
      {#if m.current}
        {@render item(
          'commit',
          `Merge another branch into ${current}…`,
          'Bring the latest main (or any branch) into this one',
          () => operations.openDialog(null),
        )}
        {@render item('pull', 'Pull', 'Fetch and bring in the upstream branch', () =>
          sync.run('pull'),
        )}
        {@render item('push', 'Push', 'Upload your commits', () => sync.run('push'))}
      {:else}
        {@render item(
          'commit',
          `Merge into ${current}…`,
          `Bring the changes from ${m.name} into your current branch`,
          () => operations.openDialog(m.name, 'merge'),
        )}
        {@render item(
          'branch',
          `Rebase ${current} onto this…`,
          'Replay your commits on top of it',
          () => operations.openDialog(m.name, 'rebase'),
        )}
        <div class="my-1 border-t border-border"></div>
        {@render item(
          'check',
          'Check out',
          m.remote ? 'Creates a local tracking branch' : 'Switch to this branch',
          () => branches.checkout(m.name, m.remote),
        )}
      {/if}
      {@render item('copy', 'Copy name', m.name, () =>
        navigator.clipboard?.writeText(m.name).catch(() => {}),
      )}
      {#if !m.remote && !m.current}
        <div class="my-1 border-t border-border"></div>
        {@render item(
          'trash',
          'Delete branch',
          'Remove the branch label',
          () => branches.remove(m.name),
          true,
        )}
      {/if}
    </div>
  </div>
{/if}
