<script lang="ts">
  import { tick, untrack } from 'svelte'
  import { api } from '../api'
  import { fuzzyFilter } from '../fuzzy'
  import { githubLinks } from '../forgeLinks'
  import { changes } from '../stores/changes.svelte'
  import { clone } from '../stores/clone.svelte'
  import { forge } from '../stores/forge.svelte'
  import { operations } from '../stores/operations.svelte'
  import { projects } from '../stores/projects.svelte'
  import { repo } from '../stores/repo.svelte'
  import { sync } from '../stores/sync.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  let { onprojects }: { onprojects: () => void } = $props()
  let query = $state('')
  let active = $state(0)
  let input = $state<HTMLInputElement>()
  let list = $state<HTMLDivElement>()
  type Command = {
    id: string
    label: string
    group: string
    hint?: string
    disabled?: boolean
    run: () => unknown
  }
  const commands = $derived.by(() => {
    const all: Command[] = [
      {
        id: 'projects',
        label: 'Switch project',
        group: 'Workspace',
        hint: 'Ctrl+P',
        run: onprojects,
      },
      {
        id: 'open',
        label: 'Open local repository',
        group: 'Workspace',
        run: () => projects.openPicker(),
      },
      { id: 'clone', label: 'Clone repository', group: 'Workspace', run: () => clone.show() },
      { id: 'settings', label: 'Settings', group: 'App', run: () => (ui.settingsOpen = true) },
      {
        id: 'help',
        label: 'Keyboard shortcuts',
        group: 'App',
        hint: '?',
        run: () => (ui.shortcutsOpen = true),
      },
    ]
    const path = projects.current?.path
    if (!path) return all
    const busy = !!sync.running || changes.busy || operations.busy
    all.unshift(
      ...(['history', 'changes', 'issues', 'pulls'] as const).map((view) => ({
        id: view,
        label: {
          history: 'Show history',
          changes: 'Show changes',
          issues: 'Show issues',
          pulls: 'Show pull requests',
        }[view],
        group: 'Navigate',
        run: () => (ui.view = view),
      })),
      ...(['fetch', 'pull', 'push'] as const).map((op) => ({
        id: op,
        label: {
          fetch: 'Fetch all remotes',
          pull: 'Pull upstream changes',
          push: 'Push current branch',
        }[op],
        group: 'Git',
        disabled: busy,
        run: () => sync.run(op),
      })),
      {
        id: 'branch',
        label: 'Switch or create branch',
        group: 'Git',
        hint: 'Ctrl+B',
        disabled: busy,
        run: () => (ui.branchSwitcherOpen = true),
      },
      {
        id: 'merge',
        label: 'Merge another branch',
        group: 'Git',
        disabled: busy,
        run: () => operations.openDialog(null),
      },
      {
        id: 'rebase',
        label: 'Rebase onto another branch',
        group: 'Git',
        disabled: busy,
        run: () => operations.openDialog(null, 'rebase'),
      },
      {
        id: 'stash',
        label: 'Stash working changes',
        group: 'Git',
        disabled: busy,
        run: () => (ui.stashDialogOpen = true),
      },
      {
        id: 'stage',
        label: 'Stage all unstaged files',
        group: 'Changes',
        disabled: busy || !changes.status.unstaged.length,
        run: () => changes.stage(changes.status.unstaged.map((f) => f.path)),
      },
      {
        id: 'conflicts',
        label: 'Resolve next conflicted file',
        group: 'Changes',
        disabled: !changes.status.conflicted.length,
        run: () => {
          ui.view = 'changes'
          return changes.selectConflict(changes.status.conflicted[0])
        },
      },
      {
        id: 'refresh',
        label: 'Refresh repository',
        group: 'Workspace',
        run: () => Promise.all([repo.refresh(), changes.refresh()]),
      },
      {
        id: 'terminal',
        label: 'Open in terminal',
        group: 'Tools',
        run: () => api.openTerminal(path),
      },
      { id: 'editor', label: 'Open in editor', group: 'Tools', run: () => api.openEditor(path) },
      {
        id: 'folder',
        label: 'Open in file manager',
        group: 'Tools',
        run: () => api.openFolder(path),
      },
      {
        id: 'pause',
        label: sync.paused ? 'Resume automatic fetch' : 'Pause automatic fetch',
        group: 'Sync',
        run: () => (sync.paused = !sync.paused),
      },
    )
    const links = githubLinks(forge.status, repo.summary?.branch ?? null)
    if (links) {
      all.push(
        {
          id: 'github',
          label: 'Open repository on GitHub',
          group: 'GitHub',
          run: () => api.openUrl(links.repository),
        },
        {
          id: 'ci',
          label: 'Open GitHub Actions / CI runs',
          group: 'GitHub',
          run: () => api.openUrl(links.actions),
        },
        {
          id: 'issue-new',
          label: 'Create issue in browser',
          group: 'GitHub',
          run: () => api.openUrl(links.newIssue),
        },
        {
          id: 'pr-new',
          label: 'Create pull request from current branch',
          group: 'GitHub',
          disabled: !links.newPull,
          run: () => links.newPull && api.openUrl(links.newPull),
        },
      )
    }
    return all
  })
  const filtered = $derived(fuzzyFilter(commands, query, (c) => `${c.label} ${c.group}`))

  $effect(() => {
    if (!ui.commandPaletteOpen) return
    const previous = untrack(() => document.activeElement)
    query = ''
    active = 0
    tick().then(() => input?.focus())
    return () => {
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus()
    }
  })
  $effect(() => {
    const index = active
    if (ui.commandPaletteOpen)
      tick().then(() =>
        list?.querySelector(`[data-index="${index}"]`)?.scrollIntoView({ block: 'nearest' }),
      )
  })

  async function execute(command: Command) {
    if (command.disabled) return
    ui.commandPaletteOpen = false
    // Restore focus before opening the next dialog.
    await tick()
    try {
      await command.run()
    } catch (error) {
      ui.notice = { kind: 'error', text: String(error) }
    }
  }
  function onkeydown(e: KeyboardEvent) {
    e.stopPropagation()
    if (e.key === 'Escape' || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k')) {
      e.preventDefault()
      ui.commandPaletteOpen = false
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault()
      active = filtered.length
        ? (active + (e.key === 'ArrowDown' ? 1 : -1) + filtered.length) % filtered.length
        : 0
    } else if (e.key === 'Enter' && filtered[active]) {
      e.preventDefault()
      void execute(filtered[active])
    } else if (e.key === 'Tab') {
      e.preventDefault()
      input?.focus()
    }
  }
</script>

{#if ui.commandPaletteOpen}
  <div
    class="backdrop pt-[12vh]"
    role="presentation"
    onmousedown={(e) => {
      if (e.target === e.currentTarget) ui.commandPaletteOpen = false
    }}
  >
    <div
      class="modal w-[620px] max-w-[95vw]"
      role="dialog"
      aria-modal="true"
      aria-label="Quick actions"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-3 border-b border-border px-4">
        <Icon name="search" class="text-accent" />
        <input
          bind:this={input}
          bind:value={query}
          oninput={() => (active = 0)}
          role="combobox"
          aria-label="Search actions"
          aria-expanded="true"
          aria-controls="command-results"
          aria-activedescendant={filtered[active] ? `command-${filtered[active].id}` : undefined}
          aria-autocomplete="list"
          placeholder="What do you want to do?"
          class="h-14 w-full bg-transparent text-[14px] outline-none"
        />
        <span class="kbd">Esc</span>
      </div>
      <div
        bind:this={list}
        id="command-results"
        role="listbox"
        aria-label="Actions"
        class="max-h-[52vh] overflow-y-auto p-2"
      >
        {#each filtered as command, i (command.id)}
          <button
            id={`command-${command.id}`}
            data-index={i}
            role="option"
            aria-selected={active === i}
            aria-disabled={command.disabled ?? false}
            tabindex="-1"
            class="flex w-full items-center gap-3 rounded-md px-3 py-2.5 text-left text-[13px] {active ===
            i
              ? 'bg-accent/10'
              : 'hover:bg-hover'} {command.disabled ? 'opacity-40' : ''}"
            onclick={() => execute(command)}
          >
            <span class="w-20 shrink-0 text-[11px] text-muted">{command.group}</span>
            <span class="flex-1">{command.label}</span>
            {#if command.hint}<span class="kbd">{command.hint}</span>{/if}
          </button>
        {:else}<p class="p-8 text-center text-muted">No matching actions.</p>{/each}
      </div>
      <div class="border-t border-border px-4 py-2 text-[11px] text-muted">
        ↑ ↓ Navigate · Enter Run · Esc Close <span class="float-right"
          >{filtered.length} actions</span
        >
      </div>
    </div>
  </div>
{/if}
