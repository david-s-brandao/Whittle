<script lang="ts">
  import { onDestroy, untrack } from 'svelte'
  import { api } from '../api'
  import { clearConflictDraft, readConflictDraft, writeConflictDraft } from '../conflictDrafts'
  import {
    conflictCount,
    parseConflictDoc,
    renderResult,
    resolveAll,
    resolvedLines,
    unresolvedCount,
    type Choice,
    type ConflictSegment,
  } from '../conflict'
  import ConflictPreview from './ConflictPreview.svelte'
  import Icon from './Icon.svelte'

  let {
    text,
    draftKey,
    filename,
    ours,
    theirs,
    busy,
    onsave,
    onopen,
  }: {
    text: string
    draftKey: string
    filename: string
    ours: string
    theirs: string
    busy: boolean
    /** Receives the merged file; the parent writes it to disk and stages it. */
    onsave: (content: string) => Promise<boolean>
    onopen: () => void
  } = $props()

  const sessionKey = untrack(() => draftKey)
  const initial = readConflictDraft(sessionKey)
  let source = $state(untrack(() => initial?.source ?? text))
  let doc = $state(untrack(() => initial?.doc ?? parseConflictDoc(source)))
  let undo = $state<string[]>([])
  let redo = $state<string[]>([])
  let preview = $state(false)
  let saved = false
  const stale = $derived(source !== text)
  const result = $derived(doc ? renderResult(doc) : '')

  onDestroy(() => {
    if (doc && !saved) writeConflictDraft(sessionKey, source, $state.snapshot(doc))
  })

  function checkpoint() {
    undo = [...undo.slice(-99), JSON.stringify(doc)]
    redo = []
  }

  function travel(direction: 'undo' | 'redo') {
    const stack = direction === 'undo' ? undo : redo
    const previous = stack.at(-1)
    if (!previous || busy) return
    if (direction === 'undo') {
      redo = [...redo, JSON.stringify(doc)]
      undo = undo.slice(0, -1)
    } else {
      undo = [...undo, JSON.stringify(doc)]
      redo = redo.slice(0, -1)
    }
    doc = JSON.parse(previous)
  }

  function pickAll(choice: Choice) {
    if (!doc || busy) return
    checkpoint()
    resolveAll(doc, choice)
  }
  /** Unchanged-line runs the user expanded, by segment index. */
  let expanded = $state<Record<number, boolean>>({})
  let root = $state<HTMLElement>()

  const total = $derived(doc ? conflictCount(doc) : 0)
  const open = $derived(doc ? unresolvedCount(doc) : 0)
  const CONTEXT = 3

  const choices: { id: Choice; label: string; title: string }[] = $derived([
    { id: 'ours', label: ours, title: `Keep only ${ours}` },
    { id: 'theirs', label: theirs, title: `Keep only ${theirs}` },
    { id: 'both', label: 'Both', title: 'Keep both, current first' },
    { id: 'both-rev', label: 'Both, reversed', title: 'Keep both, incoming first' },
    { id: 'none', label: 'Neither', title: 'Drop both sides' },
  ])

  function pick(c: ConflictSegment, choice: Choice) {
    if (busy) return
    checkpoint()
    // clicking the active choice again un-resolves it
    c.choice = c.choice === choice ? null : choice
  }

  function edit(c: ConflictSegment, value: string) {
    checkpoint()
    c.custom = value
    c.choice = 'custom'
  }

  function startEdit(c: ConflictSegment) {
    if (c.choice === 'custom') return
    checkpoint()
    c.custom = (resolvedLines(c) ?? [...c.ours, ...c.theirs]).join('\n')
    c.choice = 'custom'
  }

  async function reset() {
    if (
      !(await api.confirm(
        'Discard the choices in this draft and reload the latest file?',
        'Reset conflict draft',
      ))
    )
      return
    checkpoint()
    source = text
    doc = parseConflictDoc(text)
    undo = []
    redo = []
    clearConflictDraft(draftKey)
    expanded = {}
  }

  function jump(dir: 1 | -1) {
    const cards = [...(root?.querySelectorAll<HTMLElement>('[data-conflict]') ?? [])]
    if (!cards.length) return
    const top = (root?.getBoundingClientRect().top ?? 0) + 60
    const y = (el: HTMLElement) => el.getBoundingClientRect().top - top
    const target =
      dir === 1
        ? (cards.find((el) => y(el) > 8) ?? cards[0])
        : ([...cards].reverse().find((el) => y(el) < -8) ?? cards[cards.length - 1])
    target.scrollIntoView({ block: 'start', behavior: 'smooth' })
  }

  async function save() {
    if (doc && !open && !busy && !stale) {
      const key = draftKey
      saved = await onsave(renderResult(doc))
      if (saved) clearConflictDraft(key)
    }
  }

  function gotoFirstOpen() {
    const el = root?.querySelector<HTMLElement>('[data-conflict][data-open="true"]')
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' })
  }

  function onkeydown(event: KeyboardEvent) {
    if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 's') return
    // Do not consume shortcuts while another dialog is in front of the editor.
    if (document.querySelector('[role="dialog"]')) return
    event.preventDefault()
    void save()
  }
</script>

<svelte:window {onkeydown} />

{#if !doc || total === 0}
  <ConflictPreview {text} {ours} {theirs} />
{:else}
  <div class="flex h-full min-h-0 flex-col" bind:this={root}>
    <div class="flex items-center gap-3 border-b border-border px-4 py-2 text-[12px]">
      <span class="min-w-0 flex-1 truncate font-mono" title={filename}>{filename}</span>
      <button class="btn h-6 px-2" disabled={busy || !undo.length} onclick={() => travel('undo')}
        >Undo</button
      >
      <button class="btn h-6 px-2" disabled={busy || !redo.length} onclick={() => travel('redo')}
        >Redo</button
      >
      <button class="btn h-6 px-2" aria-pressed={preview} onclick={() => (preview = !preview)}
        >{preview ? 'Back to conflicts' : 'Preview full result'}</button
      >
    </div>
    {#if stale}
      <div role="status" class="border-b border-border bg-del/10 px-4 py-2 text-[12px]">
        The file changed on disk. Your draft is preserved; reset to load the latest version before
        saving.
      </div>
    {/if}
    <!-- toolbar -->
    <div
      class="flex flex-wrap items-center gap-2 border-b border-border bg-surface px-4 py-2 text-[12px]"
    >
      <span class="font-semibold">
        {#if open}
          {open} of {total} conflict{total === 1 ? '' : 's'} left
        {:else}
          All {total} conflict{total === 1 ? '' : 's'} resolved
        {/if}
      </span>
      <button class="icon-btn icon-btn-sm" title="Previous conflict" onclick={() => jump(-1)}
        ><span class="rotate-180"><Icon name="chevron-down" size={14} /></span></button
      >
      <button class="icon-btn icon-btn-sm" title="Next conflict" onclick={() => jump(1)}
        ><Icon name="chevron-down" size={14} /></button
      >
      {#if open}
        <button class="btn h-6 px-2 text-[11.5px]" onclick={gotoFirstOpen}>Go to unresolved</button>
      {/if}
      <span class="mx-1 h-4 w-px bg-border"></span>
      <button
        class="btn h-6 px-2 text-[11.5px]"
        title="Resolve every remaining conflict with your side"
        disabled={busy || !open}
        onclick={() => pickAll('ours')}>All {ours}</button
      >
      <button
        class="btn h-6 px-2 text-[11.5px]"
        title="Resolve every remaining conflict with the incoming side"
        disabled={busy || !open}
        onclick={() => pickAll('theirs')}>All {theirs}</button
      >
      <button
        class="btn h-6 px-2 text-[11.5px]"
        title="Discard your choices and start over from the file on disk"
        disabled={busy}
        onclick={reset}>Reset</button
      >
      <button class="btn h-6 px-2 text-[11.5px]" onclick={onopen}
        ><Icon name="external" size={12} /> Open in editor</button
      >
      <button
        class="btn btn-primary ml-auto h-7 px-3 text-[12px] font-semibold"
        disabled={busy || stale || open > 0}
        title={open ? 'Resolve every conflict first' : 'Write the result and stage the file'}
        onclick={save}>{busy ? 'Saving…' : 'Save and mark resolved'}</button
      >
    </div>

    <!-- body -->
    {#if preview}
      <div class="selectable min-h-0 flex-1 overflow-auto bg-surface-2 p-4">
        <p class="mb-3 text-[12px] text-muted">
          Live result · {open
            ? 'Unresolved blocks retain their conflict markers.'
            : 'Ready to save and stage.'}
        </p>
        <pre class="font-mono text-[12px] leading-5">{result}</pre>
      </div>
    {:else}
      <div class="selectable min-h-0 flex-1 overflow-auto font-mono text-[12px] leading-[20px]">
        {#each doc.segments as seg, i (i)}
          {#if seg.type === 'text'}
            {@const n = seg.lines.length}
            {@const long = n > CONTEXT * 2 + 1 && !expanded[i]}
            {@const first = i === 0}
            {@const last = i === doc.segments.length - 1}
            <div class="min-w-max px-4 py-1">
              {#if long}
                {#each first ? [] : seg.lines.slice(0, CONTEXT) as line, k (k)}
                  <div class="whitespace-pre text-muted">{line || ' '}</div>
                {/each}
                <button
                  class="my-0.5 rounded bg-surface-2 px-2 text-[11px] text-faint hover:text-fg"
                  onclick={() => (expanded[i] = true)}
                >
                  ⋯ {n - (first ? 0 : CONTEXT) - (last ? 0 : CONTEXT)} unchanged lines
                </button>
                {#each last ? [] : seg.lines.slice(n - CONTEXT) as line, k (k)}
                  <div class="whitespace-pre text-muted">{line || ' '}</div>
                {/each}
              {:else}
                {#each seg.lines as line, k (k)}
                  <div class="whitespace-pre text-muted">{line || ' '}</div>
                {/each}
              {/if}
            </div>
          {:else}
            {@const idx = doc.segments.slice(0, i).filter((s) => s.type === 'conflict').length + 1}
            {@const res = resolvedLines(seg)}
            <div
              data-conflict
              data-open={seg.choice === null}
              class="mx-3 my-2 rounded-md border {seg.choice === null
                ? 'border-del/50'
                : 'border-border'} bg-surface"
            >
              <div
                class="flex flex-wrap items-center gap-1.5 border-b border-border px-3 py-1.5 font-sans text-[12px]"
              >
                <span class="font-semibold">Conflict {idx} of {total}</span>
                {#if seg.choice === null}
                  <span class="chip bg-del text-white">unresolved</span>
                {:else}
                  <span class="chip chip-accent">resolved</span>
                {/if}
                <span class="ml-auto flex flex-wrap items-center gap-1">
                  {#each choices as c (c.id)}
                    <button
                      class="btn h-6 px-2 text-[11.5px] {seg.choice === c.id ? 'btn-primary' : ''}"
                      title={c.title}
                      disabled={busy}
                      aria-pressed={seg.choice === c.id}
                      onclick={() => pick(seg, c.id)}>{c.label}</button
                    >
                  {/each}
                </span>
              </div>

              <div class="grid grid-cols-2 divide-x divide-border">
                {#each [{ id: 'ours', label: ours, name: seg.oursLabel, lines: seg.ours, bg: 'bg-info/10', dot: 'bg-info/50' }, { id: 'theirs', label: theirs, name: seg.theirsLabel, lines: seg.theirs, bg: 'bg-[var(--add-bg)]', dot: 'bg-add/50' }] as side (side.id)}
                  <button
                    class="min-w-0 text-left {seg.choice === side.id
                      ? 'ring-2 ring-accent ring-inset'
                      : ''}"
                    title="Use this side"
                    disabled={busy}
                    aria-pressed={seg.choice === side.id}
                    onclick={() => pick(seg, side.id as Choice)}
                  >
                    <div
                      class="flex items-center gap-1.5 border-b border-border px-3 py-1 font-sans text-[11px] text-muted"
                    >
                      <span class="size-2.5 shrink-0 rounded-sm {side.dot}"></span>
                      <b class="text-fg">{side.label}</b>
                      <span class="truncate text-faint">{side.name}</span>
                    </div>
                    <div class="{side.bg} min-h-[28px] overflow-x-auto px-3 py-1">
                      {#each side.lines as line, k (k)}
                        <div class="whitespace-pre">{line || ' '}</div>
                      {:else}
                        <div class="font-sans text-faint italic">(empty)</div>
                      {/each}
                    </div>
                  </button>
                {/each}
              </div>

              {#if seg.base}
                <details class="border-t border-border px-3 py-1 font-sans text-[11px] text-muted">
                  <summary class="cursor-pointer select-none">Common ancestor</summary>
                  <div class="mt-1 font-mono text-[12px]">
                    {#each seg.base as line, k (k)}
                      <div class="whitespace-pre">{line || ' '}</div>
                    {:else}
                      <div class="font-sans text-faint italic">(empty)</div>
                    {/each}
                  </div>
                </details>
              {/if}

              <div class="border-t border-border">
                <div class="flex items-center gap-2 px-3 py-1 font-sans text-[11px] text-muted">
                  <b class="text-fg">Result</b>
                  {#if res === null}
                    <span class="text-faint">Pick a side above, or write it yourself.</span>
                  {/if}
                  {#if seg.choice !== 'custom'}
                    <button
                      class="ml-auto hover:text-fg"
                      disabled={busy}
                      onclick={() => startEdit(seg)}>Edit by hand</button
                    >
                  {:else}
                    <span class="text-faint">Edited by hand</span>
                  {/if}
                </div>
                {#if seg.choice === 'custom'}
                  <textarea
                    class="block w-full resize-y bg-surface-2 px-3 py-1 font-mono text-[12px] leading-[20px] whitespace-pre outline-none"
                    rows={Math.max(1, seg.custom.split('\n').length)}
                    spellcheck="false"
                    aria-label={`Result for conflict ${idx}`}
                    disabled={busy}
                    value={seg.custom}
                    oninput={(e) => edit(seg, e.currentTarget.value)}></textarea>
                {:else if res !== null}
                  <div class="overflow-x-auto bg-surface-2 px-3 py-1">
                    {#each res as line, k (k)}
                      <div class="whitespace-pre">{line || ' '}</div>
                    {:else}
                      <div class="font-sans text-faint italic">(nothing: both sides dropped)</div>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
{/if}
