<script lang="ts">
  import { api } from '../api'
  import { detail, forge } from '../stores/forge.svelte'
  import { projects } from '../stores/projects.svelte'
  import { repo } from '../stores/repo.svelte'
  import type { RepoBranches } from '../types'
  import ReviewerPicker from './ReviewerPicker.svelte'
  import Icon from './Icon.svelte'

  let { open = $bindable(false) }: { open: boolean } = $props()

  let remote = $state<RepoBranches | null>(null)
  let loadError = $state<string | null>(null)
  let head = $state('')
  let base = $state('')
  let title = $state('')
  let body = $state('')
  let draft = $state(false)
  let reviewers = $state<string[]>([])
  let prev = ''

  $effect(() => {
    if (!open) return
    const path = projects.current?.path
    title = body = ''
    draft = false
    reviewers = []
    remote = null
    loadError = null
    prev = ''
    if (!path) return
    let stale = false
    api
      .forgeBranches(path)
      .then((r) => {
        if (stale) return
        remote = r
        const current = repo.refs.local.find((b) => b.is_head)?.name ?? ''
        base = r.default_branch
        head = current && current !== r.default_branch ? current : ''
      })
      .catch((e) => !stale && (loadError = String(e?.message ?? e)))
    return () => (stale = true)
  })

  // the checked-out branch may not be on GitHub yet; show it, but say so
  const localOnly = $derived(
    repo.refs.local.map((b) => b.name).filter((n) => remote && !remote.branches.includes(n)),
  )
  const same = $derived(!!head && head === base)
  const unpushed = $derived(!!head && localOnly.includes(head))
  const ready = $derived(!!head && !!base && !same && !!title.trim() && !unpushed)

  function pick(h: string) {
    head = h
    if (!title.trim() || title === suggest(prev)) title = suggest(h)
    prev = h
  }
  const suggest = (h: string) =>
    h
      .replace(/^[^/]+\//, '')
      .replace(/[-_]+/g, ' ')
      .replace(/^./, (c) => c.toUpperCase())

  async function submit() {
    if (!ready || detail.busy) return
    if (await detail.createPull(head, base, title, body, draft, reviewers)) open = false
  }
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (open = false)}
  >
    <div
      class="modal w-[560px]"
      role="dialog"
      aria-modal="true"
      aria-label="New pull request"
      tabindex="-1"
      onkeydown={(e) => e.key === 'Escape' && (open = false)}
    >
      <div class="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
        <Icon name="pr" class="text-accent" />
        <div class="text-[15px] font-semibold">New pull request</div>
      </div>

      <div class="space-y-4 px-5 py-5">
        {#if loadError}
          <p class="text-[12.5px] text-del">{loadError}</p>
        {:else if !remote}
          <p class="text-[12.5px] text-muted">Loading branches…</p>
        {:else}
          <div class="grid grid-cols-[1fr_auto_1fr] items-end gap-2">
            <div>
              <label class="label" for="pr-head">From (compare)</label>
              <select
                id="pr-head"
                class="input font-mono"
                value={head}
                onchange={(e) => pick(e.currentTarget.value)}
              >
                <option value="" disabled>Choose a branch…</option>
                {#each remote.branches as b (b)}<option value={b}>{b}</option>{/each}
                {#each localOnly as b (b)}<option value={b}>{b} (not pushed)</option>{/each}
              </select>
            </div>
            <span class="mb-2 text-faint" aria-hidden="true">→</span>
            <div>
              <label class="label" for="pr-base">Into (base)</label>
              <select id="pr-base" class="input font-mono" bind:value={base}>
                {#each remote.branches as b (b)}<option value={b}>{b}</option>{/each}
              </select>
            </div>
          </div>
          {#if same}
            <p class="-mt-2 text-[12px] text-del">Choose two different branches.</p>
          {:else if unpushed}
            <p class="-mt-2 text-[12px] text-del">
              This branch isn't on GitHub yet. Push it first, then come back.
            </p>
          {/if}

          <div>
            <label class="label" for="pr-title">Title</label>
            <input
              id="pr-title"
              class="input"
              bind:value={title}
              placeholder="What does this change?"
            />
          </div>
          <div>
            <label class="label" for="pr-body">Description (optional)</label>
            <textarea
              id="pr-body"
              rows="5"
              bind:value={body}
              class="input h-auto resize-y py-2"
              placeholder="Why, and how to review it…"></textarea>
          </div>
          <div>
            <div class="label">Request reviewers</div>
            <ReviewerPicker
              selected={reviewers}
              exclude={forge.status?.login ? [forge.status.login] : []}
              onchange={(n) => (reviewers = n)}
            />
          </div>
          <label class="flex items-center gap-2 text-[12.5px]">
            <input type="checkbox" class="accent-[var(--accent)]" bind:checked={draft} />
            Open as draft
          </label>
        {/if}
      </div>

      <div class="flex justify-end gap-2 border-t border-border bg-surface-2 px-5 py-3">
        <button class="btn btn-outline h-8" onclick={() => (open = false)}>Cancel</button>
        <button
          class="btn btn-primary h-8 px-4 font-semibold"
          disabled={!ready || detail.busy}
          onclick={submit}>{detail.busy ? 'Creating…' : draft ? 'Create draft' : 'Create'}</button
        >
      </div>
    </div>
  </div>
{/if}
