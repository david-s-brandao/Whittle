<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import logo from './assets/logo.png'
  import { api } from './lib/api'
  import Dock from './lib/components/Dock.svelte'
  import ForgeView from './lib/components/ForgeView.svelte'
  import TabBar from './lib/components/TabBar.svelte'
  import RepositoryStatus from './lib/components/RepositoryStatus.svelte'
  import RepositoryNavigation from './lib/components/RepositoryNavigation.svelte'
  import CommandPalette from './lib/components/CommandPalette.svelte'
  import SettingsDialog from './lib/components/SettingsDialog.svelte'
  import InteractiveRebaseDialog from './lib/components/InteractiveRebaseDialog.svelte'
  import FileViewer from './lib/components/FileViewer.svelte'
  import CommitMenu from './lib/components/CommitMenu.svelte'
  import NameDialog from './lib/components/NameDialog.svelte'
  import ResetDialog from './lib/components/ResetDialog.svelte'
  import BranchMenu from './lib/components/BranchMenu.svelte'
  import BranchSwitcher from './lib/components/BranchSwitcher.svelte'
  import ChangesView from './lib/components/ChangesView.svelte'
  import CloneDialog from './lib/components/CloneDialog.svelte'
  import HistoryView from './lib/components/HistoryView.svelte'
  import Icon from './lib/components/Icon.svelte'
  import MergeDialog from './lib/components/MergeDialog.svelte'
  import OperationBanner from './lib/components/OperationBanner.svelte'
  import PullMenu from './lib/components/PullMenu.svelte'
  import ProjectSwitcher from './lib/components/ProjectSwitcher.svelte'
  import ShortcutsDialog from './lib/components/ShortcutsDialog.svelte'
  import Sidebar from './lib/components/Sidebar.svelte'
  import SyncStatus from './lib/components/SyncStatus.svelte'
  import ThemeToggle from './lib/components/ThemeToggle.svelte'
  import StashDialog from './lib/components/StashDialog.svelte'
  import Toolbar from './lib/components/Toolbar.svelte'
  import Welcome from './lib/components/Welcome.svelte'
  import { changes } from './lib/stores/changes.svelte'
  import { dock } from './lib/stores/dock.svelte'
  import { clone } from './lib/stores/clone.svelte'
  import { projects } from './lib/stores/projects.svelte'
  import { repo } from './lib/stores/repo.svelte'
  import { search } from './lib/stores/search.svelte'
  import { settings } from './lib/stores/settings.svelte'
  import { board } from './lib/stores/issueBoard.svelte'
  import { detail, forge, issues, pulls } from './lib/stores/forge.svelte'
  import { sync } from './lib/stores/sync.svelte'
  import { tabs } from './lib/stores/tabs.svelte'
  import { ui } from './lib/stores/ui.svelte'
  import { watch } from './lib/stores/watch.svelte'

  let switcherOpen = $state(false)

  onMount(() => {
    const stopAutoFetch = sync.startAutoFetch()
    settings.load()
    projects.refresh().then(() => tabs.restoreSession(projects.list.map((p) => p.path)))
    const unlisten = api.onProgress((op, line) => {
      sync.onProgress(op, line)
      clone.onProgress(op, line)
    })
    const unwatch = api.onRepoChanged((e) => watch.onChanged(e))
    return () => {
      stopAutoFetch()
      unlisten.then((f) => f())
      unwatch.then((f) => f())
    }
  })

  $effect(() => {
    const path = projects.current?.path
    // Only the project path is a dependency; the refreshes read lots of other state.
    untrack(() => {
      tabs.track(path)
      forge.reset()
      issues.reset()
      pulls.reset()
      board.reset()
      detail.clear()
      forge.load()
      search.clear()
      repo.refresh(path)
      changes.refresh()
      watch.follow(path)
    })
  })

  async function launch(what: 'terminal' | 'editor' | 'folder') {
    const path = projects.current?.path
    if (!path) return
    try {
      if (what === 'terminal') await api.openTerminal(path)
      else if (what === 'editor') await api.openEditor(path)
      else await api.openFolder(path)
    } catch (e) {
      ui.notice = { kind: 'error', text: String(e) }
    }
  }

  function typingInField(e: KeyboardEvent): boolean {
    const t = e.target
    return (
      t instanceof HTMLInputElement ||
      t instanceof HTMLTextAreaElement ||
      (t instanceof HTMLElement && t.isContentEditable)
    )
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.defaultPrevented) return
    if (e.target instanceof Element && e.target.closest('[role="dialog"]')) return
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault()
      ui.commandPaletteOpen = !ui.commandPaletteOpen
      return
    }
    if (e.key === '?' && !e.ctrlKey && !e.metaKey && !typingInField(e)) {
      e.preventDefault()
      ui.shortcutsOpen = !ui.shortcutsOpen
      return
    }
    if (!(e.ctrlKey || e.metaKey)) return
    if (projects.current && (e.key === '`' || e.key.toLowerCase() === 'j') && !e.shiftKey) {
      e.preventDefault()
      if (e.key === '`') dock.show('terminal')
      else dock.toggle()
      return
    }
    const k = e.key.toLowerCase()
    const views = ['history', 'changes', 'issues', 'pulls'] as const
    if (projects.current && /^[1-4]$/.test(e.key) && !e.shiftKey && !e.altKey) {
      e.preventDefault()
      ui.view = views[Number(e.key) - 1]
    } else if (e.key === 'Tab') {
      e.preventDefault()
      tabs.cycle(e.shiftKey ? -1 : 1)
    } else if (k === 'w' && projects.current) {
      e.preventDefault()
      tabs.close(projects.current.path)
    } else if (k === 't') {
      e.preventDefault()
      switcherOpen = true
    } else if (k === 'p') {
      e.preventDefault()
      switcherOpen = !switcherOpen
    } else if (k === 'b' && projects.current) {
      e.preventDefault()
      ui.branchSwitcherOpen = !ui.branchSwitcherOpen
    }
  }
</script>

<svelte:window {onkeydown} onfocus={() => changes.refresh()} />

<div class="flex h-full flex-col bg-surface text-fg">
  <header class="flex h-12 shrink-0 items-center gap-2 border-b border-border px-3">
    <img src={logo} alt="Whittle" class="size-6 select-none" draggable="false" />
    <button
      class="btn btn-outline h-8 max-w-64 gap-2 pr-2"
      title="Switch project (Ctrl+P)"
      onclick={() => (switcherOpen = true)}
    >
      <Icon name="folder" class="text-muted" />
      <span class="truncate">{projects.current?.name ?? 'Open a project'}</span>
      <Icon name="chevron-down" size={14} class="text-faint" />
    </button>
    {#if projects.current}
      <div class="mx-1 h-5 w-px bg-border"></div>
      <Toolbar />
    {/if}
    <div class="ml-auto flex shrink-0 items-center gap-1">
      <button
        class="btn h-8"
        title="Quick actions (Ctrl+K)"
        onclick={() => (ui.commandPaletteOpen = true)}
        ><Icon name="search" size={14} /><span class="kbd">Ctrl K</span></button
      >
      {#if projects.current}
        <button
          class="icon-btn {dock.open && dock.active === 'terminal' ? 'text-accent' : ''}"
          title="Terminal inside Whittle (Ctrl+`)"
          aria-pressed={dock.open && dock.active === 'terminal'}
          onclick={() => dock.show('terminal')}><Icon name="panel-bottom" /></button
        >
        <button
          class="icon-btn"
          title="Open a terminal window in this repository"
          onclick={() => launch('terminal')}><Icon name="terminal" /></button
        >
        <button
          class="icon-btn"
          title="Open this repository in your editor"
          onclick={() => launch('editor')}><Icon name="code" /></button
        >
        <button
          class="icon-btn"
          title="Show this folder in the file manager"
          onclick={() => launch('folder')}><Icon name="folder" /></button
        >
        <div class="mx-1 h-5 w-px bg-border"></div>
      {/if}
      <button
        class="icon-btn"
        title="Keyboard shortcuts (?)"
        onclick={() => (ui.shortcutsOpen = true)}><Icon name="keyboard" /></button
      >
      <button class="icon-btn" title="Settings" onclick={() => (ui.settingsOpen = true)}
        ><Icon name="settings" /></button
      >
      <ThemeToggle />
    </div>
  </header>

  <TabBar onNew={() => (switcherOpen = true)} />
  <OperationBanner />
  <div class="flex min-h-0 flex-1">
    {#if projects.current}
      <Sidebar />
      <main class="flex min-w-0 flex-1 flex-col">
        <RepositoryNavigation />
        <div
          id="repository-panel"
          role="tabpanel"
          aria-labelledby={`folder-${ui.view}`}
          tabindex="0"
          class="min-h-0 flex-1 outline-none"
          class:hidden={dock.maximized}
        >
          {#if ui.view === 'history'}<HistoryView />{:else if ui.view === 'changes'}<ChangesView
            />{:else}<ForgeView kind={ui.view} />{/if}
        </div>
        <Dock />
      </main>
    {:else}
      <main class="min-w-0 flex-1"><Welcome /></main>
    {/if}
  </div>
  {#if projects.current}<RepositoryStatus />{/if}
</div>

<SyncStatus />
<CommandPalette onprojects={() => (switcherOpen = true)} />
<ProjectSwitcher bind:open={switcherOpen} />
<BranchSwitcher />
<BranchMenu />
<CommitMenu />
<FileViewer />
<InteractiveRebaseDialog />
<SettingsDialog />
<NameDialog />
<ResetDialog />
<PullMenu />
<ShortcutsDialog />
<MergeDialog />
<StashDialog />
<CloneDialog />
