<script lang="ts">
  import Icon from './Icon.svelte'
  import TerminalView from './TerminalView.svelte'
  import { projects } from '../stores/projects.svelte'
  import { terminals } from '../stores/terminal.svelte'

  let { visible }: { visible: boolean } = $props()

  const path = $derived(projects.current?.path)
  const mine = $derived(terminals.forPath(path))
  const activeKey = $derived(terminals.activeFor(path))

  // the first time the Terminal tab is shown for a project, start a shell in it
  $effect(() => {
    if (visible && path && terminals.forPath(path).length === 0) terminals.add(path)
  })
</script>

<div class="flex h-full min-h-0 flex-col bg-surface">
  <div class="flex h-full min-h-0">
    <div class="relative min-w-0 flex-1">
      <!-- every shell stays mounted (so it keeps running); only the chosen one of this project shows -->
      {#each terminals.tabs as t (t.key)}
        <div class="absolute inset-0" class:hidden={t.path !== path}>
          <TerminalView
            tabKey={t.key}
            path={t.path}
            visible={visible && t.path === path && t.key === activeKey}
            onexit={() => {}}
          />
        </div>
      {/each}
    </div>
    <div
      class="flex w-9 shrink-0 flex-col items-center gap-1 border-l border-border bg-surface-2 py-1.5"
    >
      <button
        class="icon-btn icon-btn-sm"
        title="New terminal"
        disabled={!path}
        onclick={() => path && terminals.add(path)}><Icon name="plus" size={14} /></button
      >
      {#each mine as t, i (t.key)}
        <div class="group relative">
          <button
            class="grid size-7 place-items-center rounded-md font-mono text-[11px] font-semibold {t.key ===
            activeKey
              ? 'bg-accent/15 text-accent'
              : 'text-muted hover:bg-hover hover:text-fg'}"
            title="{t.title} (click to switch)"
            onclick={() => path && terminals.select(path, t.key)}>{i + 1}</button
          >
          <button
            class="absolute -top-1 -right-1 hidden size-4 place-items-center rounded-full bg-surface text-muted shadow ring-1 ring-border group-hover:grid hover:text-del"
            title="Close {t.title}"
            aria-label="Close {t.title}"
            onclick={() => terminals.close(t.key)}><Icon name="x" size={10} /></button
          >
        </div>
      {/each}
    </div>
  </div>
</div>
