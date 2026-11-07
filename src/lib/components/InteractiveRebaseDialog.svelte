<script lang="ts">
  import { api } from '../api'
  import { operations } from '../stores/operations.svelte'
  import { projects } from '../stores/projects.svelte'
  import { repo } from '../stores/repo.svelte'
  import { ui } from '../stores/ui.svelte'
  import type { CommitInfo, RebaseAction } from '../types'
  import Icon from './Icon.svelte'

  interface Row {
    commit: CommitInfo
    action: RebaseAction
    message: string
  }

  /** Newest first, like the history list. */
  let rows = $state<Row[]>([])
  let loading = $state(false)
  let error = $state<string | null>(null)
  let dragIndex = $state<number | null>(null)
  let overIndex = $state<number | null>(null)

  const from = $derived(ui.interactiveRebase)

  const actions: { id: RebaseAction; label: string; hint: string }[] = [
    { id: 'pick', label: 'Pick', hint: 'Keep the commit as it is' },
    { id: 'reword', label: 'Reword', hint: 'Keep the changes, write a new message' },
    { id: 'squash', label: 'Squash', hint: 'Fold into the older commit below, combining messages' },
    {
      id: 'fixup',
      label: 'Fixup',
      hint: 'Fold into the older commit below, discarding this message',
    },
    { id: 'drop', label: 'Drop', hint: 'Remove the commit and its changes' },
  ]

  $effect(() => {
    const c = ui.interactiveRebase
    if (!c) return
    rows = []
    error = null
    const path = projects.current?.path
    if (!path) return
    loading = true
    api
      .rebasePlan(path, c.id)
      .then((list) => {
        rows = list
          .map((commit) => ({ commit, action: 'pick' as RebaseAction, message: commit.summary }))
          .reverse()
        original = rows.map((r) => r.commit.id)
      })
      .catch((e) => (error = String(e)))
      .finally(() => (loading = false))
  })

  // git applies oldest first: the oldest kept commit has nothing to squash into
  const oldestKept = $derived([...rows].reverse().find((r) => r.action !== 'drop'))
  const invalid = $derived(
    oldestKept && (oldestKept.action === 'squash' || oldestKept.action === 'fixup')
      ? 'The oldest commit can’t be squashed or fixed up: there is nothing older to fold it into.'
      : rows.length && rows.every((r) => r.action === 'drop')
        ? 'Dropping every commit would leave nothing to rebase.'
        : rows.some((r) => r.action === 'reword' && !r.message.trim())
          ? 'A reworded commit needs a message.'
          : '',
  )
  const remaining = $derived(
    rows.filter((r) => r.action !== 'drop' && r.action !== 'squash' && r.action !== 'fixup').length,
  )
  /** ids in the order they were loaded, to tell whether anything was changed */
  let original = $state<string[]>([])
  const changed = $derived(
    rows.some((r) => r.action !== 'pick') || rows.some((r, i) => r.commit.id !== original[i]),
  )
  const unpushed = $derived(Math.min(repo.summary?.ahead ?? 0, rows.length))
  const pushedCount = $derived(repo.summary?.has_upstream ? rows.length - unpushed : 0)

  function move(i: number, delta: number) {
    const j = i + delta
    if (j < 0 || j >= rows.length) return
    const next = [...rows]
    ;[next[i], next[j]] = [next[j], next[i]]
    rows = next
  }
  function drop(to: number) {
    if (dragIndex === null || dragIndex === to) return
    const next = [...rows]
    const [moved] = next.splice(dragIndex, 1)
    next.splice(to, 0, moved)
    rows = next
  }

  async function go() {
    const c = from
    if (!c || invalid) return
    ui.interactiveRebase = null
    // the dialog shows newest first; git wants oldest first
    const items = [...rows].reverse().map((r) => ({
      id: r.commit.id,
      action: r.action,
      message: r.action === 'reword' ? r.message : null,
    }))
    await operations.interactive(c.id, items)
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') ui.interactiveRebase = null
  }
</script>

{#if from}
  <div
    class="backdrop pt-[6vh]"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (ui.interactiveRebase = null)}
  >
    <div
      class="modal flex max-h-[86vh] w-[760px] flex-col"
      role="dialog"
      aria-modal="true"
      aria-label="Interactive rebase"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
        <Icon name="branch" class="text-accent" />
        <div class="text-[15px] font-semibold">Interactive rebase</div>
        <div class="text-[12px] text-muted">
          {rows.length} commit{rows.length === 1 ? '' : 's'} from
          <span class="font-mono">{from.short_id}</span> to HEAD
        </div>
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto px-3 py-3">
        {#if loading}
          <div class="py-10 text-center text-muted">Loading…</div>
        {:else if error}
          <div class="mx-2 rounded-lg border border-del/40 bg-del/10 p-3 text-[12.5px] text-del">
            {error}
          </div>
        {:else}
          <p class="px-2 pb-2 text-[12px] text-muted">
            Newest at the top. Drag a row or use the arrows to reorder; “squash” and “fixup” fold a
            commit into the older one below it.
          </p>
          <ul class="space-y-1">
            {#each rows as r, i (r.commit.id)}
              <li
                class="rounded-lg border {overIndex === i && dragIndex !== null && dragIndex !== i
                  ? 'border-accent'
                  : 'border-border'} {r.action === 'drop' ? 'opacity-50' : ''} bg-surface"
                draggable="true"
                ondragstart={(e) => {
                  dragIndex = i
                  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
                  e.dataTransfer?.setData('text/plain', String(i))
                }}
                ondragover={(e) => {
                  e.preventDefault()
                  overIndex = i
                }}
                ondragleave={() => overIndex === i && (overIndex = null)}
                ondrop={(e) => {
                  e.preventDefault()
                  drop(i)
                  dragIndex = overIndex = null
                }}
                ondragend={() => (dragIndex = overIndex = null)}
              >
                <div class="flex items-center gap-2 px-2 py-1.5">
                  <span class="cursor-grab px-1 text-faint select-none" title="Drag to reorder"
                    >⋮⋮</span
                  >
                  <div class="flex flex-col">
                    <button
                      class="icon-btn icon-btn-sm h-4"
                      title="Move up (newer)"
                      disabled={i === 0}
                      onclick={() => move(i, -1)}
                      ><Icon name="chevron-down" size={12} class="rotate-180" /></button
                    >
                    <button
                      class="icon-btn icon-btn-sm h-4"
                      title="Move down (older)"
                      disabled={i === rows.length - 1}
                      onclick={() => move(i, 1)}><Icon name="chevron-down" size={12} /></button
                    >
                  </div>
                  <span class="w-14 shrink-0 font-mono text-[11.5px] text-faint"
                    >{r.commit.short_id}</span
                  >
                  {#if r.action === 'reword'}
                    <input
                      class="input h-7 flex-1 py-0"
                      bind:value={r.message}
                      placeholder="New commit message"
                    />
                  {:else}
                    <span
                      class="min-w-0 flex-1 truncate text-[13px] {r.action === 'drop'
                        ? 'line-through'
                        : ''}"
                      title={r.commit.summary}>{r.commit.summary}</span
                    >
                  {/if}
                  <select
                    class="h-7 shrink-0 rounded-md border border-border bg-surface px-2 text-[12px] font-medium outline-none focus:border-accent {r.action ===
                    'drop'
                      ? 'text-del'
                      : r.action === 'pick'
                        ? ''
                        : 'text-accent'}"
                    value={r.action}
                    onchange={(e) => (r.action = e.currentTarget.value as RebaseAction)}
                    title={actions.find((a) => a.id === r.action)?.hint}
                  >
                    {#each actions as a (a.id)}<option value={a.id}>{a.label}</option>{/each}
                  </select>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="space-y-2 border-t border-border bg-surface-2 px-5 py-3">
        {#if pushedCount > 0}
          <div class="rounded-md border border-del/40 bg-del/10 px-3 py-2 text-[12px]">
            <b
              >{pushedCount} of these commit{pushedCount === 1 ? ' is' : 's are'} already on the remote.</b
            >
            Rewriting them changes their hashes, so you'd need to force-push, and anyone else using this
            branch would be affected.
          </div>
        {/if}
        {#if invalid}
          <div class="text-[12px] text-del">{invalid}</div>
        {:else if !loading && !error}
          <div class="text-[12px] text-muted">
            {rows.length} commit{rows.length === 1 ? '' : 's'} become {remaining}. You can abort at
            any point if there are conflicts.
          </div>
        {/if}
        <div class="flex justify-end gap-2">
          <button class="btn btn-outline h-8" onclick={() => (ui.interactiveRebase = null)}
            >Cancel</button
          >
          <button
            class="btn btn-primary h-8 px-4 font-semibold"
            disabled={!!invalid || loading || !!error || !changed}
            onclick={go}>Start rebase</button
          >
        </div>
      </div>
    </div>
  </div>
{/if}
