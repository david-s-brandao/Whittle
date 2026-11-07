<script lang="ts">
  import { tick } from 'svelte'
  import { plural } from '../format'
  import { changes } from '../stores/changes.svelte'
  import { stashes } from '../stores/stashes.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  let message = $state('')
  let untracked = $state(false)
  let input: HTMLInputElement | undefined = $state()

  const nothing = $derived(changes.total === 0)
  const onlyUntracked = $derived(
    changes.total > 0 &&
      changes.status.staged.length === 0 &&
      changes.status.unstaged.every((f) => f.status === 'untracked'),
  )

  $effect(() => {
    if (ui.stashDialogOpen) {
      message = ''
      untracked = false
      tick().then(() => input?.focus())
    }
  })

  async function save() {
    if (nothing || stashes.busy) return
    ui.stashDialogOpen = false
    await stashes.create(message, untracked || onlyUntracked)
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') ui.stashDialogOpen = false
    else if (e.key === 'Enter') save()
  }
</script>

{#if ui.stashDialogOpen}
  <div
    class="backdrop"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (ui.stashDialogOpen = false)}
  >
    <div
      class="modal w-[460px]"
      role="dialog"
      aria-modal="true"
      aria-label="Stash changes"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
        <Icon name="stash" class="text-accent" />
        <div class="text-[15px] font-semibold">Stash changes</div>
      </div>
      <div class="space-y-4 px-5 py-5">
        <p class="text-muted">
          {#if nothing}
            There are no local changes to stash.
          {:else}
            Shelves {plural(changes.total, 'changed file')} and restores a clean working tree. You can
            bring them back any time from the sidebar.
          {/if}
        </p>
        <div>
          <label class="label" for="stash-msg">Message (optional)</label>
          <input
            id="stash-msg"
            bind:this={input}
            bind:value={message}
            disabled={nothing}
            placeholder="What are you putting aside?"
            class="input"
          />
        </div>
        <label class="flex cursor-pointer items-center gap-2 text-[12.5px] select-none">
          <input
            type="checkbox"
            class="size-3.5 accent-[var(--accent)]"
            checked={untracked || onlyUntracked}
            disabled={nothing || onlyUntracked}
            onchange={(e) => (untracked = e.currentTarget.checked)}
          />
          Include untracked (new) files
          {#if onlyUntracked}<span class="text-faint">(required: only new files changed)</span>{/if}
        </label>
      </div>
      <div class="flex justify-end gap-2 border-t border-border bg-surface-2 px-5 py-3">
        <button class="btn btn-outline h-8" onclick={() => (ui.stashDialogOpen = false)}
          >Cancel</button
        >
        <button class="btn btn-primary h-8 px-4 font-semibold" disabled={nothing} onclick={save}
          >Stash changes</button
        >
      </div>
    </div>
  </div>
{/if}
