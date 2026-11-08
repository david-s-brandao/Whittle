<script lang="ts">
  import { changes } from '../stores/changes.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  const folders = [
    { id: 'history', label: 'History', icon: 'history' },
    { id: 'changes', label: 'Changes', icon: 'changes' },
    { id: 'issues', label: 'Issues', icon: 'issue' },
    { id: 'pulls', label: 'Pull requests', icon: 'pr' },
  ] as const

  function navigate(event: KeyboardEvent, index: number) {
    let next: number
    if (event.key === 'ArrowRight') next = (index + 1) % folders.length
    else if (event.key === 'ArrowLeft') next = (index + folders.length - 1) % folders.length
    else if (event.key === 'Home') next = 0
    else if (event.key === 'End') next = folders.length - 1
    else return
    event.preventDefault()
    ui.view = folders[next].id
    document.getElementById(`folder-${ui.view}`)?.focus()
  }
</script>

<div
  role="tablist"
  aria-label="Repository folders"
  class="flex shrink-0 items-end gap-1 overflow-x-auto border-b border-border bg-surface-2 px-3 pt-2"
>
  {#each folders as folder, index (folder.id)}
    <button
      id={`folder-${folder.id}`}
      role="tab"
      aria-selected={ui.view === folder.id}
      aria-controls="repository-panel"
      tabindex={ui.view === folder.id ? 0 : -1}
      class="relative flex h-10 shrink-0 items-center gap-2 rounded-t-lg border border-b-0 px-4 text-[12.5px] font-medium transition-colors {ui.view ===
      folder.id
        ? 'border-border bg-surface text-fg shadow-[inset_0_2px_0_var(--accent)]'
        : 'border-transparent text-muted hover:bg-hover hover:text-fg'}"
      title="{folder.label} (Ctrl+{index + 1})"
      onclick={() => (ui.view = folder.id)}
      onkeydown={(event) => navigate(event, index)}
    >
      <Icon
        name={ui.view === folder.id ? 'folder' : folder.icon}
        size={15}
        class={ui.view === folder.id ? 'text-accent' : ''}
      />
      {folder.label}
      {#if folder.id === 'changes'}
        <span class="chip {changes.total ? 'chip-accent' : ''}">{changes.total}</span>
        {#if changes.status.conflicted.length}<span
            class="size-2 rounded-full bg-del"
            title={`${changes.status.conflicted.length} conflicted files`}
            aria-label={`${changes.status.conflicted.length} conflicted files`}
          ></span>{/if}
      {/if}
    </button>
  {/each}
</div>
