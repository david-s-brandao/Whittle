<script lang="ts">
  import { prefs, type PullMode } from '../stores/prefs.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  const m = $derived(ui.pullMenu)
  const modes: { id: PullMode; title: string; hint: string }[] = [
    { id: 'default', title: 'Default', hint: 'Follow your git config, otherwise merge' },
    { id: 'merge', title: 'Merge', hint: 'Add a merge commit if the branches diverged' },
    { id: 'rebase', title: 'Rebase', hint: 'Replay your commits on top: straight history' },
    { id: 'ff-only', title: 'Fast-forward only', hint: 'Refuse if a merge would be needed' },
  ]
  function close() {
    ui.pullMenu = null
  }
  function choose(id: PullMode) {
    prefs.pullMode = id
    prefs.save()
  }
  function toggleStash(on: boolean) {
    prefs.pullAutostash = on
    prefs.save()
  }
  const left = $derived(m ? Math.max(8, Math.min(m.x, window.innerWidth - 330)) : 0)
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

{#if m}
  <div class="fixed inset-0 z-50" role="presentation" onmousedown={close}>
    <div
      class="absolute w-80 overflow-hidden rounded-lg border border-border bg-surface py-1 shadow-xl"
      style="left: {left}px; top: {m.y}px"
      role="menu"
      tabindex="-1"
      onmousedown={(e) => e.stopPropagation()}
    >
      <div class="section-label px-3 pt-1.5 pb-1">When pulling, bring changes in by</div>
      {#each modes as it (it.id)}
        <button
          role="menuitemradio"
          aria-checked={prefs.pullMode === it.id}
          class="flex w-full items-start gap-2.5 px-3 py-1.5 text-left hover:bg-hover"
          onclick={() => choose(it.id)}
        >
          <span class="mt-0.5 grid size-4 shrink-0 place-items-center text-accent">
            {#if prefs.pullMode === it.id}<Icon name="check" size={14} />{/if}
          </span>
          <span>
            <span class="block text-[12.5px] font-medium">{it.title}</span>
            <span class="block text-[11.5px] text-muted">{it.hint}</span>
          </span>
        </button>
      {/each}
      <div class="my-1 border-t border-border"></div>
      <label class="flex cursor-pointer items-start gap-2.5 px-3 py-1.5 hover:bg-hover">
        <input
          type="checkbox"
          class="mt-0.5 ml-0.5 size-3.5 accent-[var(--accent)]"
          checked={prefs.pullAutostash}
          onchange={(e) => toggleStash(e.currentTarget.checked)}
        />
        <span>
          <span class="block text-[12.5px] font-medium">Stash &amp; re-apply local changes</span>
          <span class="block text-[11.5px] text-muted"
            >Lets you pull with uncommitted work in progress</span
          >
        </span>
      </label>
    </div>
  </div>
{/if}
