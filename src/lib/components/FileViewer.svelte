<script lang="ts">
  import { api } from '../api'
  import { splitPath, formatDate, relativeTime } from '../format'
  import { laneColor, lanePalette } from '../graphColors'
  import { projects } from '../stores/projects.svelte'
  import { repo } from '../stores/repo.svelte'
  import { theme } from '../stores/theme.svelte'
  import { ui } from '../stores/ui.svelte'
  import type { Blame, CommitInfo } from '../types'
  import Icon from './Icon.svelte'

  let tab = $state<'blame' | 'history'>('blame')
  let blame = $state<Blame | null>(null)
  let history = $state<CommitInfo[] | null>(null)
  let error = $state<string | null>(null)
  let loading = $state(false)
  let gen = 0

  const v = $derived(ui.fileView)
  const palette = $derived(lanePalette(theme.dark))

  $effect(() => {
    if (!v) return
    tab = v.tab
    blame = null
    history = null
    error = null
    const path = projects.current?.path
    if (!path) return
    const my = ++gen
    loading = true
    Promise.all([
      api.fileBlame(path, v.path, v.rev).catch((e) => {
        if (my === gen && tab === 'blame') error = String(e)
        return null
      }),
      api.fileHistory(path, v.path).catch(() => null),
    ]).then(([b, h]) => {
      if (my !== gen) return
      blame = b
      history = h
      loading = false
    })
  })

  // a stable colour per commit so runs of lines by the same commit read as a block
  const colorOf = (id: string) => {
    let h = 0
    for (let i = 0; i < 8; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0
    return laneColor(palette, h)
  }

  /** hunk that starts at each line, so the label is drawn once per block */
  const startsAt = $derived.by(() => {
    const m: Record<number, Blame['hunks'][number]> = {}
    for (const h of blame?.hunks ?? []) m[h.start] = h
    return m
  })
  const hunkOf = $derived.by(() => {
    const out: (Blame['hunks'][number] | undefined)[] = []
    for (const h of blame?.hunks ?? []) for (let i = 0; i < h.count; i++) out[h.start + i] = h
    return out
  })

  function showCommit(c: { id: string }) {
    const target = repo.commits.find((x) => x.id === c.id)
    ui.fileView = null
    ui.view = 'history'
    if (target) repo.selectCommit(target)
    repo.revealCommit(c.id)
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') ui.fileView = null
  }
</script>

{#if v}
  {@const [dir, name] = splitPath(v.path)}
  <div
    class="backdrop pt-[5vh]"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (ui.fileView = null)}
  >
    <div
      class="modal flex h-[84vh] w-[1000px] flex-col"
      role="dialog"
      aria-modal="true"
      aria-label="File blame and history"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-3 border-b border-border px-5 py-3">
        <Icon name="file" class="text-accent" />
        <div class="min-w-0 flex-1">
          <div class="truncate text-[14px] font-semibold">
            <span class="text-faint">{dir}</span>{name}
          </div>
          <div class="text-[11.5px] text-muted">
            {v.rev ? `as of ${v.rev.slice(0, 7)}` : 'at the checked-out commit'}
          </div>
        </div>
        <div class="flex rounded-md border border-border p-0.5 text-[12px] font-medium">
          {#each ['blame', 'history'] as const as t (t)}
            <button
              class="rounded px-3 py-1 capitalize {tab === t
                ? 'bg-accent text-white'
                : 'text-muted hover:text-fg'}"
              onclick={() => (tab = t)}>{t}</button
            >
          {/each}
        </div>
        <button class="icon-btn" title="Close (Esc)" onclick={() => (ui.fileView = null)}
          ><Icon name="x" /></button
        >
      </div>

      <div class="min-h-0 flex-1 overflow-auto">
        {#if loading}
          <div class="grid h-full place-items-center text-muted">Loading…</div>
        {:else if tab === 'blame'}
          {#if error || !blame}
            <div class="grid h-full place-items-center px-6 text-center text-muted">
              {error ?? 'No blame available.'}
            </div>
          {:else}
            <div class="selectable min-w-max font-mono text-[12px] leading-[20px]">
              {#each blame.lines as text, i (i)}
                {@const n = i + 1}
                {@const h = hunkOf[n]}
                {@const first = startsAt[n]}
                <div class="flex {first ? 'border-t border-border/60' : ''}">
                  <button
                    class="flex w-72 shrink-0 items-center gap-2 border-l-[3px] pr-3 pl-2 text-left font-sans text-[11.5px] hover:bg-hover"
                    style="border-color: {h ? colorOf(h.id) : 'transparent'}"
                    title={h ? `${h.summary}\n${h.author} · ${formatDate(h.time)}` : ''}
                    disabled={!h}
                    onclick={() => h && showCommit(h)}
                  >
                    {#if first}
                      <span class="font-mono text-faint">{first.short_id}</span>
                      <span class="min-w-0 flex-1 truncate">{first.author}</span>
                      <span class="shrink-0 text-faint">{relativeTime(first.time)}</span>
                    {/if}
                  </button>
                  <span class="w-12 shrink-0 pr-3 text-right text-[11px] text-faint select-none"
                    >{n}</span
                  >
                  <span class="pr-6 whitespace-pre">{text}</span>
                </div>
              {/each}
            </div>
          {/if}
        {:else if !history}
          <div class="grid h-full place-items-center text-muted">No history available.</div>
        {:else}
          <ul class="divide-y divide-border">
            {#each history as c (c.id)}
              <li>
                <button
                  class="flex w-full items-center gap-4 px-5 py-2 text-left text-[13px] hover:bg-hover"
                  onclick={() => showCommit(c)}
                >
                  <span class="w-16 shrink-0 font-mono text-[11.5px] text-faint">{c.short_id}</span>
                  <span class="min-w-0 flex-1 truncate">{c.summary}</span>
                  <span class="w-36 shrink-0 truncate text-muted">{c.author}</span>
                  <span
                    class="w-24 shrink-0 text-right text-[12px] text-muted"
                    title={formatDate(c.time)}>{relativeTime(c.time)}</span
                  >
                </button>
              </li>
            {/each}
            {#if !history.length}
              <li class="px-5 py-8 text-center text-muted">No commits touch this file yet.</li>
            {/if}
          </ul>
        {/if}
      </div>
    </div>
  </div>
{/if}
