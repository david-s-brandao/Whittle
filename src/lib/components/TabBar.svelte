<script lang="ts">
  import { changes } from '../stores/changes.svelte'
  import { projects } from '../stores/projects.svelte'
  import { repo } from '../stores/repo.svelte'
  import { tabs } from '../stores/tabs.svelte'
  import Icon from './Icon.svelte'

  let { onNew }: { onNew: () => void } = $props()

  const current = $derived(projects.current?.path)
  const nameOf = (path: string) =>
    projects.list.find((p) => p.path === path)?.name ??
    path.split('/').filter(Boolean).pop() ??
    path
  const branchOf = (path: string) =>
    path === current ? repo.summary?.branch : projects.summaries[path]?.branch
  const dirtyOf = (path: string) =>
    path === current ? changes.total > 0 : !!projects.summaries[path]?.dirty

  // keep branch / dirty info of background tabs fresh
  $effect(() => {
    void tabs.paths.length
    void current
    projects.loadSummaries()
  })
</script>

{#if tabs.paths.length}
  <div
    class="flex h-9 shrink-0 items-end gap-0.5 overflow-x-auto border-b border-border bg-surface-2 px-2 pt-1"
  >
    {#each tabs.paths as path (path)}
      {@const active = path === current}
      <div
        role="tab"
        aria-selected={active}
        tabindex="0"
        class="group relative flex h-8 max-w-56 min-w-28 shrink-0 cursor-default items-center gap-2 rounded-t-md border border-b-0 px-3 text-[12.5px] {active
          ? 'border-border bg-surface font-medium text-fg shadow-[inset_0_2px_0_var(--accent)]'
          : 'border-transparent text-muted hover:bg-hover hover:text-fg'}"
        title="{path}{branchOf(path) ? `\nbranch: ${branchOf(path)}` : ''}"
        onclick={() => tabs.activate(path)}
        onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && tabs.activate(path)}
        onauxclick={(e) => {
          if (e.button === 1) {
            e.preventDefault()
            tabs.close(path)
          }
        }}
      >
        <Icon name="folder" size={13} class={active ? 'text-accent' : 'text-faint'} />
        <span class="truncate">{nameOf(path)}</span>
        {#if dirtyOf(path)}
          <span class="size-1.5 shrink-0 rounded-full bg-accent" title="Uncommitted changes"></span>
        {/if}
        <button
          class="icon-btn icon-btn-sm -mr-1.5 ml-auto size-5 shrink-0 {active
            ? ''
            : 'invisible group-hover:visible'}"
          title="Close tab (Ctrl+W)"
          aria-label="Close tab"
          onclick={(e) => {
            e.stopPropagation()
            tabs.close(path)
          }}><Icon name="x" size={12} /></button
        >
      </div>
    {/each}
    <button
      class="icon-btn mb-0.5 ml-1 size-6 shrink-0"
      title="Open another repository in a new tab (Ctrl+T)"
      onclick={onNew}><Icon name="plus" size={14} /></button
    >
  </div>
{/if}
