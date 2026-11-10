<script lang="ts">
  import { dock, DOCK_MIN, type DockTab } from '../stores/dock.svelte'
  import { repo } from '../stores/repo.svelte'
  import CommitDetails from './CommitDetails.svelte'
  import DescriptionPanel from './DescriptionPanel.svelte'
  import ForgeView from './ForgeView.svelte'
  import Icon, { type IconName } from './Icon.svelte'
  import TerminalPanel from './TerminalPanel.svelte'

  const META: Record<DockTab, { label: string; icon: IconName; hint: string }> = {
    files: {
      label: 'Files',
      icon: 'changes',
      hint: 'Changed files and diff of the selected commit',
    },
    description: {
      label: 'Description',
      icon: 'file-text',
      hint: 'Full message and details of the selected commit',
    },
    terminal: { label: 'Terminal', icon: 'terminal', hint: 'Terminal in this repository (Ctrl+`)' },
    issues: { label: 'Issues', icon: 'issue', hint: 'Issues and board' },
    pulls: { label: 'Pull requests', icon: 'pr', hint: 'Pull requests' },
  }

  let box: HTMLDivElement | undefined = $state()
  let dragging = $state(false)

  const active = $derived(dock.active)
  const forgeTab = $derived(active === 'issues' || active === 'pulls' ? active : null)

  function start(e: PointerEvent) {
    if (dock.maximized) return
    dragging = true
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }
  function move(e: PointerEvent) {
    const parent = box?.parentElement
    if (!dragging || !parent) return
    const rect = parent.getBoundingClientRect()
    dock.height = Math.round(
      Math.min(rect.height - 120, Math.max(DOCK_MIN, rect.bottom - e.clientY)),
    )
  }
  function end() {
    if (!dragging) return
    dragging = false
    dock.persist()
  }
  function onresizekey(e: KeyboardEvent) {
    const step = e.shiftKey ? 80 : 24
    if (e.key === 'ArrowUp') dock.height += step
    else if (e.key === 'ArrowDown') dock.height = Math.max(DOCK_MIN, dock.height - step)
    else return
    e.preventDefault()
    dock.persist()
  }

  function navigate(e: KeyboardEvent, i: number) {
    const tabs = dock.available
    let n: number
    if (e.key === 'ArrowRight') n = (i + 1) % tabs.length
    else if (e.key === 'ArrowLeft') n = (i + tabs.length - 1) % tabs.length
    else return
    e.preventDefault()
    dock.select(tabs[n])
    document.getElementById(`dock-${tabs[n]}`)?.focus()
  }
</script>

<div
  bind:this={box}
  class="flex shrink-0 flex-col border-t border-border bg-surface {dock.maximized
    ? 'min-h-0 flex-1'
    : ''}"
  style={dock.open && !dock.maximized ? `height: ${dock.height}px` : ''}
>
  {#if dock.open && !dock.maximized}
    <!-- drag handle: hairline with a wider invisible hit area; turns orange on hover / drag -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div class="group relative z-10 -mt-px h-px shrink-0 bg-border">
      <div
        role="separator"
        aria-orientation="horizontal"
        aria-label="Resize bottom panel"
        aria-valuenow={dock.height}
        aria-valuemin={DOCK_MIN}
        aria-valuemax={9999}
        tabindex="0"
        class="absolute inset-x-0 -top-1.5 -bottom-1.5 cursor-row-resize touch-none outline-none"
        onpointerdown={start}
        onpointermove={move}
        onpointerup={end}
        onpointercancel={end}
        onkeydown={onresizekey}
        ondblclick={() => dock.toggleMaximize()}
        title="Drag to resize. Double-click to maximize."
      >
        <div
          class="absolute inset-x-0 top-1/2 h-0.5 -translate-y-1/2 bg-accent transition-opacity group-focus-within:opacity-100 {dragging
            ? 'opacity-100'
            : 'opacity-0 group-hover:opacity-100'}"
        ></div>
      </div>
    </div>
  {/if}

  <div class="flex h-9 shrink-0 items-center gap-1 bg-surface-2 px-2">
    <div
      role="tablist"
      aria-label="Bottom panel"
      class="flex min-w-0 items-center gap-0.5 overflow-x-auto"
    >
      {#each dock.available as tab, i (tab)}
        {@const on = dock.open && active === tab}
        <button
          id="dock-{tab}"
          role="tab"
          aria-selected={on}
          aria-controls="dock-panel"
          tabindex={on || (!dock.open && i === 0) ? 0 : -1}
          title={META[tab].hint}
          class="flex h-7 shrink-0 items-center gap-1.5 rounded-md px-2.5 text-[12px] font-medium transition-colors {on
            ? 'bg-surface text-fg shadow-[inset_0_-2px_0_var(--accent)]'
            : 'text-muted hover:bg-hover hover:text-fg'}"
          onclick={() => dock.select(tab)}
          onkeydown={(e) => navigate(e, i)}
        >
          <Icon name={META[tab].icon} size={14} class={on ? 'text-accent' : ''} />
          {META[tab].label}
          {#if tab === 'files' && repo.selected && repo.files.length}
            <span class="chip">{repo.files.length}</span>
          {/if}
        </button>
      {/each}
    </div>
    <div class="ml-auto flex shrink-0 items-center gap-0.5 pl-2">
      {#if forgeTab && dock.open}
        <button
          class="btn h-7 gap-1.5 px-2 text-[12px]"
          title="Move to the main area"
          onclick={() => dock.pullUp(forgeTab)}
          ><Icon name="external" size={13} /> Move to main area</button
        >
      {/if}
      {#if dock.open}
        <button
          class="icon-btn icon-btn-sm"
          title={dock.maximized ? 'Restore size' : 'Maximize panel'}
          aria-label={dock.maximized ? 'Restore size' : 'Maximize panel'}
          onclick={() => dock.toggleMaximize()}
          ><Icon name={dock.maximized ? 'minimize' : 'maximize'} size={14} /></button
        >
      {/if}
      <button
        class="icon-btn icon-btn-sm"
        title={dock.open ? 'Collapse panel (Ctrl+J)' : 'Expand panel (Ctrl+J)'}
        aria-label={dock.open ? 'Collapse panel' : 'Expand panel'}
        aria-expanded={dock.open}
        onclick={() => dock.toggle()}
        ><Icon name={dock.open ? 'chevron-down' : 'chevron-up'} size={14} /></button
      >
    </div>
  </div>

  <div
    id="dock-panel"
    role="tabpanel"
    aria-label={META[active].label}
    class="min-h-0 flex-1 border-t border-border"
    class:hidden={!dock.open}
  >
    <!-- the terminal stays mounted so its shells keep running while you look at other tabs -->
    <div class="h-full" class:hidden={active !== 'terminal'}>
      <TerminalPanel visible={dock.open && active === 'terminal'} />
    </div>
    {#if active === 'files'}<CommitDetails />
    {:else if active === 'description'}<DescriptionPanel />
    {:else if forgeTab}<ForgeView kind={forgeTab} />{/if}
  </div>
</div>
