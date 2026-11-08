<script lang="ts">
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  const groups: { title: string; items: { keys: string[]; label: string }[] }[] = [
    {
      title: 'General',
      items: [
        { keys: ['Ctrl', 'K'], label: 'Quick actions (Git, navigation, tools and GitHub)' },
        { keys: ['Ctrl', 'P'], label: 'Switch project' },
        { keys: ['Ctrl', 'B'], label: 'Switch or create branch' },
        { keys: ['Ctrl', 'T'], label: 'Open another repository in a new tab' },
        { keys: ['Ctrl', 'W'], label: 'Close the current tab' },
        { keys: ['Ctrl', 'Tab'], label: 'Next tab (add Shift for previous)' },
        { keys: ['Ctrl', '1–4'], label: 'Go to History, Changes, Issues or Pull requests' },
        { keys: ['?'], label: 'Show this panel' },
        { keys: ['Esc'], label: 'Close a dialog, or clear the search' },
      ],
    },
    {
      title: 'History',
      items: [
        { keys: ['Ctrl', 'F'], label: 'Search commits (message, author or hash)' },
        {
          keys: ['↑', '↓'],
          label: 'Select the previous / next commit (also PgUp, PgDn, Home, End)',
        },
        { keys: ['Esc'], label: 'Clear the search box' },
      ],
    },
    {
      title: 'Changes',
      items: [
        { keys: ['Ctrl', 'Enter'], label: 'Commit (while typing the message)' },
        { keys: ['Ctrl', 'S'], label: 'Save and stage the resolved conflict file' },
      ],
    },
    {
      title: 'In dialogs and lists',
      items: [
        { keys: ['↑', '↓'], label: 'Move through the list' },
        { keys: ['Enter'], label: 'Confirm / open the highlighted item' },
      ],
    },
  ]
  const tips = [
    'Click a branch in the sidebar to find it in the history; double-click to check it out.',
    'Double-click a changed file to stage or unstage it.',
    'Right-click a branch (or use ⋯) to merge, rebase or delete it.',
    'Drag the line above the commit details to resize that panel.',
    'In Changes, tick lines in a diff to stage just those lines.',
  ]

  // Only Escape is handled here. `?` is handled once, by the app-level shortcut handler,
  // which toggles the panel; handling it in both places would open and instantly close it.
  function onkeydown(e: KeyboardEvent) {
    if (ui.shortcutsOpen && e.key === 'Escape') {
      e.preventDefault()
      ui.shortcutsOpen = false
    }
  }
</script>

<svelte:window {onkeydown} />

{#if ui.shortcutsOpen}
  <div
    class="backdrop pt-[8vh]"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (ui.shortcutsOpen = false)}
  >
    <div
      class="modal w-[560px]"
      role="dialog"
      aria-modal="true"
      aria-label="Keyboard shortcuts"
      tabindex="-1"
    >
      <div class="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
        <Icon name="keyboard" class="text-accent" />
        <div class="text-[15px] font-semibold">Keyboard shortcuts</div>
        <button
          class="icon-btn ml-auto"
          title="Close (Esc)"
          onclick={() => (ui.shortcutsOpen = false)}><Icon name="x" /></button
        >
      </div>
      <div class="max-h-[65vh] space-y-5 overflow-y-auto px-5 py-4">
        {#each groups as g (g.title)}
          <section>
            <div class="section-label mb-2">{g.title}</div>
            <ul class="space-y-1.5">
              {#each g.items as it (it.label)}
                <li class="flex items-center justify-between gap-4 text-[13px]">
                  <span>{it.label}</span>
                  <span class="flex shrink-0 items-center gap-1">
                    {#each it.keys as k, i (i)}<span class="kbd">{k}</span>{/each}
                  </span>
                </li>
              {/each}
            </ul>
          </section>
        {/each}
        <section>
          <div class="section-label mb-2">Good to know</div>
          <ul class="list-disc space-y-1 pl-5 text-[12.5px] text-muted">
            {#each tips as t (t)}<li>{t}</li>{/each}
          </ul>
        </section>
      </div>
    </div>
  </div>
{/if}
