<script lang="ts">
  import { tick } from 'svelte'
  import { fuzzyFilter } from '../fuzzy'
  import { clone } from '../stores/clone.svelte'
  import { projects } from '../stores/projects.svelte'
  import type { Project } from '../types'
  import Icon from './Icon.svelte'

  let { open = $bindable(false) }: { open: boolean } = $props()

  let query = $state('')
  let active = $state(0)
  let input: HTMLInputElement | undefined = $state()

  const filtered = $derived(fuzzyFilter(projects.list, query, (p) => `${p.name} ${p.path}`))
  const starred = $derived(filtered.filter((p) => p.starred))
  const recent = $derived(filtered.filter((p) => !p.starred))
  // Flat order used for keyboard navigation (matches visual order).
  const flat = $derived([...starred, ...recent])

  $effect(() => {
    if (open) {
      query = ''
      active = 0
      projects.refresh().then(() => projects.loadSummaries())
      tick().then(() => input?.focus())
    }
  })

  async function choose(p: Project) {
    if (await projects.openPath(p.path)) open = false
  }
  async function openLocal() {
    if (await projects.openPicker()) open = false
  }
  async function cloneRepo() {
    open = false
    await clone.show()
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false
    else if (e.key === 'ArrowDown') {
      e.preventDefault()
      active = Math.min(active + 1, flat.length - 1)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      active = Math.max(active - 1, 0)
    } else if (e.key === 'Enter' && flat[active]) choose(flat[active])
  }
</script>

{#snippet row(p: Project)}
  {@const s = projects.summaries[p.path]}
  {@const on = flat[active]?.path === p.path}
  <div
    role="option"
    aria-selected={on}
    tabindex="-1"
    class="group flex cursor-pointer items-center gap-3 rounded-lg px-2.5 py-2 {on
      ? 'bg-accent/10'
      : 'hover:bg-hover'}"
    onclick={() => choose(p)}
    onkeydown={null}
  >
    <button
      class="icon-btn icon-btn-sm {p.starred ? 'text-accent' : 'text-faint hover:text-accent'}"
      title={p.starred ? 'Unstar' : 'Star'}
      onclick={(e) => {
        e.stopPropagation()
        projects.toggleStar(p)
      }}><Icon name="star" size={15} fill={p.starred} /></button
    >
    <div class="min-w-0 flex-1">
      <div class="truncate font-medium">{p.name}</div>
      <div class="truncate text-[11.5px] text-muted">{p.path}</div>
    </div>
    {#if s?.branch}
      <span
        class="flex shrink-0 items-center gap-1 rounded-md bg-hover px-1.5 py-0.5 text-[11.5px] text-muted"
      >
        <Icon name="branch" size={12} />{s.branch}
      </span>
    {/if}
    {#if s?.dirty}
      <span class="size-2 shrink-0 rounded-full bg-accent" title="Uncommitted changes"></span>
    {/if}
    <button
      class="icon-btn icon-btn-sm invisible hover:text-del group-hover:visible"
      title="Remove from list (files are not deleted)"
      onclick={(e) => {
        e.stopPropagation()
        projects.remove(p)
      }}><Icon name="x" size={14} /></button
    >
  </div>
{/snippet}

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (open = false)}
  >
    <div
      class="modal w-[580px]"
      role="dialog"
      aria-modal="true"
      aria-label="Switch project"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-3 border-b border-border px-4">
        <Icon name="search" class="text-faint" />
        <input
          bind:this={input}
          bind:value={query}
          oninput={() => (active = 0)}
          placeholder="Search projects…"
          class="h-12 w-full bg-transparent text-[14px] outline-none placeholder:text-faint"
        />
        <span class="kbd">Esc</span>
      </div>
      {#if projects.error}
        <div class="bg-del/10 px-4 py-2 text-[12.5px] text-del">{projects.error}</div>
      {/if}
      <div class="max-h-[50vh] overflow-y-auto p-2" role="listbox">
        {#if starred.length}
          <div class="section-label px-2.5 pt-1 pb-1.5">Starred</div>
          {#each starred as p (p.path)}{@render row(p)}{/each}
        {/if}
        {#if recent.length}
          <div class="section-label px-2.5 pt-2 pb-1.5">Recent</div>
          {#each recent as p (p.path)}{@render row(p)}{/each}
        {/if}
        {#if !flat.length}
          <div class="px-2 py-10 text-center text-muted">
            {projects.list.length
              ? 'No matching projects.'
              : 'No projects yet. Open or clone a repository.'}
          </div>
        {/if}
      </div>
      <div class="flex items-center gap-2 border-t border-border bg-surface-2 px-3 py-2.5">
        <button class="btn btn-primary" onclick={openLocal}
          ><Icon name="folder" size={14} />Open repository…</button
        >
        <button class="btn btn-outline" onclick={cloneRepo}
          ><Icon name="download" size={14} />Clone…</button
        >
        <span class="ml-auto text-[11.5px] text-faint">
          <span class="kbd">↑</span> <span class="kbd">↓</span> navigate ·
          <span class="kbd">Enter</span> open
        </span>
      </div>
    </div>
  </div>
{/if}
