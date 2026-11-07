<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity'
  import type { FileDiff, Selection } from '../types'

  /** Makes the diff interactive: pick hunks / lines and stage (or unstage) just those. */
  export interface DiffActions {
    /** true = this is the staged diff, so the action is "Unstage". */
    staged: boolean
    busy: boolean
    apply: (selections: Selection[]) => void
  }

  let { diff, actions }: { diff: FileDiff; actions?: DiffActions } = $props()

  const bg = { '+': 'bg-[var(--add-bg)]', '-': 'bg-[var(--del-bg)]', ' ': '', '\\': '' } as const
  const sign = { '+': 'text-add', '-': 'text-del', ' ': '', '\\': '' } as const

  const verb = $derived(actions?.staged ? 'Unstage' : 'Stage')
  const picked = new SvelteSet<string>()
  const key = (h: number, l: number) => `${h}:${l}`

  // a new diff (file changed, different file) invalidates any selection
  $effect(() => {
    void diff
    picked.clear()
  })

  const changed = (hi: number) =>
    diff.hunks[hi].lines.flatMap((l, i) => (l.kind === '+' || l.kind === '-' ? [i] : []))
  const hunkAllPicked = (hi: number) => {
    const c = changed(hi)
    return c.length > 0 && c.every((i) => picked.has(key(hi, i)))
  }
  const hunkSomePicked = (hi: number) => changed(hi).some((i) => picked.has(key(hi, i)))

  function toggleLine(hi: number, li: number) {
    const k = key(hi, li)
    if (picked.has(k)) picked.delete(k)
    else picked.add(k)
  }
  function toggleHunk(hi: number) {
    const all = hunkAllPicked(hi)
    for (const i of changed(hi)) {
      if (all) picked.delete(key(hi, i))
      else picked.add(key(hi, i))
    }
  }

  const pickedCount = $derived(picked.size)

  function applyHunk(hi: number) {
    actions?.apply([{ hunk_id: diff.hunks[hi].id, lines: null }])
  }
  function applyPicked() {
    const sels: Selection[] = []
    diff.hunks.forEach((h, hi) => {
      const lines = changed(hi).filter((i) => picked.has(key(hi, i)))
      if (!lines.length) return
      sels.push({ hunk_id: h.id, lines: lines.length === changed(hi).length ? null : lines })
    })
    if (sels.length) actions?.apply(sels)
  }
</script>

{#if diff.binary}
  <div class="grid h-full place-items-center p-6 text-muted">Binary file: no preview.</div>
{:else if !diff.hunks.length}
  <div class="grid h-full place-items-center p-6 text-muted">No textual changes.</div>
{:else}
  {#if actions && pickedCount > 0}
    <div
      class="sticky top-0 z-20 flex items-center gap-2 border-b border-border bg-surface/95 px-4 py-1.5 backdrop-blur"
    >
      <span class="text-[12.5px] font-medium">
        {pickedCount} line{pickedCount === 1 ? '' : 's'} selected
      </span>
      <button class="btn btn-primary h-6 px-2.5" disabled={actions.busy} onclick={applyPicked}>
        {verb} selected lines
      </button>
      <button class="btn h-6 px-2" onclick={() => picked.clear()}>Clear</button>
    </div>
  {/if}
  <div class="selectable min-w-max py-1 font-mono text-[12px] leading-[20px]">
    {#each diff.hunks as h, hi (h.id + hi)}
      <div class="mt-1 flex items-center gap-2 bg-surface-2 px-4 first:mt-0">
        {#if actions}
          <input
            type="checkbox"
            class="size-3.5 accent-[var(--accent)]"
            title="Select every changed line in this hunk"
            checked={hunkAllPicked(hi)}
            indeterminate={hunkSomePicked(hi) && !hunkAllPicked(hi)}
            onchange={() => toggleHunk(hi)}
          />
        {/if}
        <span class="text-[11px] text-faint">{h.header}</span>
        {#if actions}
          <button
            class="btn btn-outline sticky right-4 ml-auto h-5 bg-surface px-2 font-sans text-[11.5px]"
            disabled={actions.busy}
            title="{verb} this whole hunk"
            onclick={() => applyHunk(hi)}>{verb} hunk</button
          >
        {/if}
      </div>
      {#each h.lines as l, li (li)}
        {@const isChange = l.kind === '+' || l.kind === '-'}
        <div class="group flex {bg[l.kind]}">
          {#if actions}
            <span class="grid w-6 shrink-0 place-items-center select-none">
              {#if isChange}
                <input
                  type="checkbox"
                  class="size-3 accent-[var(--accent)] {picked.has(key(hi, li))
                    ? ''
                    : 'opacity-0 group-hover:opacity-100'}"
                  checked={picked.has(key(hi, li))}
                  onchange={() => toggleLine(hi, li)}
                  aria-label="Select line"
                />
              {/if}
            </span>
          {/if}
          <span class="w-11 shrink-0 pr-2 text-right text-[11px] text-faint select-none"
            >{l.old_no ?? ''}</span
          >
          <span class="w-11 shrink-0 pr-2 text-right text-[11px] text-faint select-none"
            >{l.new_no ?? ''}</span
          >
          {#if l.kind === '\\'}
            <span class="pr-6 pl-5 text-[11px] whitespace-pre text-faint italic">\ {l.text}</span>
          {:else}
            <span class="w-5 shrink-0 text-center select-none {sign[l.kind]}"
              >{l.kind === ' ' ? '' : l.kind}</span
            >
            <span class="pr-6 whitespace-pre">{l.text}</span>
          {/if}
        </div>
      {/each}
    {/each}
  </div>
{/if}
