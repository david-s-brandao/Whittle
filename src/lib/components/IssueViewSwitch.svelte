<script lang="ts">
  import { ui } from '../stores/ui.svelte'

  const views = [
    ['list', 'List'],
    ['board', 'Board'],
    ['table', 'Table'],
  ] as const

  function pick(v: (typeof views)[number][0]) {
    ui.issuesView = v
    try {
      localStorage.setItem('whittle.issuesView', v)
    } catch {
      // just not remembered
    }
  }
</script>

<div
  class="flex rounded-md border border-border bg-surface p-0.5 text-[12px] font-medium"
  role="group"
  aria-label="Show issues as"
>
  {#each views as [id, label] (id)}
    <button
      class="rounded px-2.5 py-1 {ui.issuesView === id
        ? 'bg-accent text-white'
        : 'text-muted hover:text-fg'}"
      aria-pressed={ui.issuesView === id}
      onclick={() => pick(id)}>{label}</button
    >
  {/each}
</div>
