<script lang="ts">
  import { relativeTime } from '../format'
  import { fuzzyFilter } from '../fuzzy'
  import { api } from '../api'
  import { suggestSources } from '../mergeSources'
  import { operations } from '../stores/operations.svelte'
  import { prefs } from '../stores/prefs.svelte'
  import { projects } from '../stores/projects.svelte'
  import { repo } from '../stores/repo.svelte'
  import { sync } from '../stores/sync.svelte'
  import { ui } from '../stores/ui.svelte'
  import type { MergePreview } from '../types'
  import Icon from './Icon.svelte'

  let mode = $state<'merge' | 'rebase'>('merge')
  let source = $state('')
  let query = $state('')
  let listOpen = $state(false)
  let message = $state('')
  let preview = $state<MergePreview | null>(null)
  let previewError = $state<string | null>(null)
  let loading = $state(false)
  let gen = 0

  const d = $derived(ui.mergeDialog)
  const current = $derived(repo.summary?.branch ?? 'the current branch')
  const suggestions = $derived(
    suggestSources(repo.refs.local, repo.refs.remote, repo.summary?.branch ?? null),
  )
  // everything that can be merged: other local branches, remote branches, tags
  const allSources = $derived([
    ...repo.refs.local.filter((b) => !b.is_head).map((b) => b.name),
    ...repo.refs.remote.map((b) => b.name),
    ...repo.refs.tags,
  ])
  const matches = $derived(fuzzyFilter(allSources, query, (n) => n).slice(0, 8))
  const isRemote = $derived(repo.refs.remote.some((b) => b.name === source))
  const alreadyPushed = $derived(
    !!repo.summary?.has_upstream && repo.summary.ahead === 0 && mode === 'rebase',
  )

  // opening the dialog: reset, take the preselected source or the best suggestion
  $effect(() => {
    if (!d) return
    mode = d.mode
    source = d.branch ?? suggestions[0]?.name ?? ''
    query = ''
    listOpen = false
    message = ''
  })

  // whenever the source (or the repo, e.g. after a fetch) changes, ask what a merge would do
  $effect(() => {
    void repo.refs
    const path = projects.current?.path
    const src = source
    if (!d || !path || !src) {
      preview = null
      previewError = null
      return
    }
    const my = ++gen
    loading = true
    api
      .mergePreview(path, src)
      .then((p) => {
        if (my !== gen) return
        preview = p
        previewError = null
      })
      .catch((e) => {
        if (my !== gen) return
        preview = null
        previewError = String(e)
      })
      .finally(() => {
        if (my === gen) loading = false
      })
  })

  const willMakeMergeCommit = $derived(
    mode === 'merge' &&
      !prefs.mergeSquash &&
      !prefs.mergeNoCommit &&
      prefs.mergeFf !== 'only' &&
      !!preview &&
      !preview.up_to_date &&
      (prefs.mergeFf === 'never' || !preview.can_fast_forward),
  )

  const label = $derived.by(() => {
    if (mode === 'rebase') return 'Rebase'
    if (prefs.mergeSquash) return 'Squash and stage'
    if (prefs.mergeNoCommit) return 'Merge without committing'
    if (preview?.can_fast_forward && prefs.mergeFf !== 'never') return 'Fast-forward'
    return 'Merge'
  })
  const nothingToDo = $derived(!source || !preview || preview.up_to_date || !!previewError)

  async function go() {
    if (!d || !source || nothingToDo) return
    const src = source
    const m = mode
    ui.mergeDialog = null
    prefs.save()
    if (m === 'merge') {
      await operations.merge(src, {
        ff: prefs.mergeFf,
        squash: prefs.mergeSquash,
        no_commit: prefs.mergeNoCommit,
        message: willMakeMergeCommit && message.trim() ? message.trim() : null,
      })
    } else {
      await operations.rebase(src)
    }
  }

  function pick(name: string) {
    source = name
    query = ''
    listOpen = false
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (listOpen) listOpen = false
      else ui.mergeDialog = null
    } else if (e.key === 'Enter' && !(e.target instanceof HTMLTextAreaElement) && !listOpen) {
      go()
    }
  }
</script>

{#if d}
  <div
    class="backdrop pt-[7vh]"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (ui.mergeDialog = null)}
  >
    <div
      class="modal w-[600px]"
      role="dialog"
      aria-modal="true"
      aria-label="Merge or rebase"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
        <Icon name="branch" class="text-accent" />
        <div class="text-[15px] font-semibold">Bring changes into <b>{current}</b></div>
        <div class="ml-auto flex rounded-md border border-border p-0.5 text-[12px] font-medium">
          {#each ['merge', 'rebase'] as const as m (m)}
            <button
              class="rounded px-3 py-1 capitalize {mode === m
                ? 'bg-accent text-white'
                : 'text-muted hover:text-fg'}"
              onclick={() => (mode = m)}>{m}</button
            >
          {/each}
        </div>
      </div>

      <div class="max-h-[68vh] space-y-4 overflow-y-auto px-5 py-4">
        <!-- source -->
        <div>
          <label class="label" for="merge-src">
            {mode === 'merge' ? 'Take changes from' : 'Rebase onto'}
          </label>
          <div class="relative">
            <input
              id="merge-src"
              class="input pr-8 font-medium"
              value={listOpen ? query : source}
              placeholder="Search branches and tags…"
              spellcheck="false"
              autocomplete="off"
              onfocus={() => {
                listOpen = true
                query = ''
              }}
              oninput={(e) => (query = e.currentTarget.value)}
            />
            <Icon
              name="chevron-down"
              size={14}
              class="pointer-events-none absolute top-3 right-3 text-faint"
            />
            {#if listOpen}
              <div
                class="absolute z-30 mt-1 max-h-56 w-full overflow-y-auto rounded-lg border border-border bg-surface py-1 shadow-xl"
              >
                {#each matches as name (name)}
                  <button
                    class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[12.5px] hover:bg-hover"
                    onmousedown={(e) => {
                      e.preventDefault()
                      pick(name)
                    }}
                  >
                    <Icon
                      name={repo.refs.tags.includes(name)
                        ? 'tag'
                        : repo.refs.remote.some((b) => b.name === name)
                          ? 'cloud'
                          : 'branch'}
                      size={13}
                      class="text-faint"
                    />
                    {name}
                  </button>
                {/each}
                {#if !matches.length}
                  <div class="px-3 py-2 text-[12px] text-muted">No matching branch.</div>
                {/if}
              </div>
            {/if}
          </div>
          {#if suggestions.length}
            <div class="mt-2 flex flex-wrap items-center gap-1.5">
              <span class="text-[11.5px] text-faint">Quick pick:</span>
              {#each suggestions.slice(0, 4) as s (s.name)}
                <button
                  class="rounded-md border px-2 py-0.5 text-[11.5px] {source === s.name
                    ? 'border-accent bg-accent/10 text-accent'
                    : 'border-border text-muted hover:border-faint hover:text-fg'}"
                  title={s.reason}
                  onclick={() => pick(s.name)}>{s.name}</button
                >
              {/each}
            </div>
          {/if}
        </div>

        <!-- preview -->
        {#if source}
          <div class="rounded-lg border border-border bg-surface-2 p-3 text-[12.5px]">
            {#if loading && !preview}
              <div class="text-muted">Checking what would change…</div>
            {:else if previewError}
              <div class="text-del">{previewError}</div>
            {:else if preview}
              {#if preview.up_to_date}
                <div class="flex items-center gap-2 font-medium text-add">
                  <Icon name="check" size={15} /> Already up to date
                </div>
                <div class="mt-1 text-muted">
                  <b class="text-fg">{current}</b> already contains everything from
                  <b class="text-fg">{source}</b>.
                </div>
              {:else}
                <div class="flex flex-wrap items-center gap-x-3 gap-y-1 font-medium">
                  <span
                    >{preview.incoming} commit{preview.incoming === 1 ? '' : 's'} to bring in</span
                  >
                  <span class="text-muted"
                    >{preview.files} file{preview.files === 1 ? '' : 's'} changed</span
                  >
                  {#if preview.only_here > 0}
                    <span class="text-muted">you have {preview.only_here} the other doesn't</span>
                  {/if}
                </div>
                {#if mode === 'merge'}
                  <div class="mt-1.5">
                    {#if preview.can_fast_forward}
                      <span class="text-add">✓ Can fast-forward: no merge commit needed.</span>
                    {:else}
                      <span class="text-muted"
                        >Both sides have new commits, so git will create a merge commit.</span
                      >
                    {/if}
                  </div>
                {/if}
                <div class="mt-1.5">
                  {#if preview.conflicts === null}
                    <span class="text-muted">Couldn't check for conflicts in advance.</span>
                  {:else if preview.conflicts.length === 0}
                    <span class="text-add">✓ Merges cleanly: no conflicts expected.</span>
                  {:else}
                    <span class="font-medium text-del"
                      >⚠ Likely conflicts in {preview.conflicts.length} file{preview.conflicts
                        .length === 1
                        ? ''
                        : 's'}:</span
                    >
                    <span class="text-muted">
                      {preview.conflicts.slice(0, 4).join(', ')}{preview.conflicts.length > 4
                        ? '…'
                        : ''}
                    </span>
                  {/if}
                </div>
                {#if preview.commits.length}
                  <ul
                    class="mt-2.5 max-h-32 space-y-0.5 overflow-y-auto border-t border-border pt-2"
                  >
                    {#each preview.commits.slice(0, 8) as c (c.id)}
                      <li class="flex items-baseline gap-2">
                        <span class="font-mono text-[11px] text-faint">{c.short_id}</span>
                        <span class="truncate">{c.summary}</span>
                        <span class="ml-auto shrink-0 text-[11px] text-faint"
                          >{relativeTime(c.time)}</span
                        >
                      </li>
                    {/each}
                    {#if preview.incoming > 8}
                      <li class="text-[11.5px] text-faint">…and {preview.incoming - 8} more</li>
                    {/if}
                  </ul>
                {/if}
              {/if}
              {#if isRemote}
                <div
                  class="mt-2.5 flex items-center gap-2 border-t border-border pt-2 text-[11.5px] text-muted"
                >
                  <Icon name="cloud" size={13} />
                  <span>Remote branches show what you had at your last fetch.</span>
                  <button
                    class="btn btn-outline ml-auto h-6 px-2 text-[11.5px]"
                    disabled={sync.running !== null}
                    onclick={() => sync.run('fetch')}
                    >{sync.running === 'fetch' ? 'Fetching…' : 'Fetch now'}</button
                  >
                </div>
              {/if}
            {/if}
          </div>
        {/if}

        <!-- options -->
        {#if mode === 'merge'}
          <fieldset class="space-y-1.5 text-[12.5px]">
            <legend class="label">How to merge</legend>
            {#each [['auto', 'Fast-forward if possible', 'No merge commit when the branch can simply move forward.'], ['never', 'Always create a merge commit', 'Keeps a visible record that this branch was merged.'], ['only', 'Fast-forward only', 'Refuse unless no merge commit is needed.']] as const as [v, t, hint] (v)}
              <label
                class="flex cursor-pointer items-start gap-2 {prefs.mergeSquash
                  ? 'opacity-40'
                  : ''}"
              >
                <input
                  type="radio"
                  name="ff"
                  class="mt-0.5 accent-[var(--accent)]"
                  checked={prefs.mergeFf === v}
                  disabled={prefs.mergeSquash}
                  onchange={() => (prefs.mergeFf = v)}
                />
                <span
                  ><span class="font-medium">{t}</span> <span class="text-muted">{hint}</span></span
                >
              </label>
            {/each}
            <div class="space-y-1.5 border-t border-border pt-2">
              <label class="flex cursor-pointer items-start gap-2">
                <input
                  type="checkbox"
                  class="mt-0.5 size-3.5 accent-[var(--accent)]"
                  bind:checked={prefs.mergeSquash}
                />
                <span
                  ><span class="font-medium">Squash</span>
                  <span class="text-muted"
                    >Combine all changes into staged edits; you make one commit yourself.</span
                  ></span
                >
              </label>
              <label
                class="flex cursor-pointer items-start gap-2 {prefs.mergeSquash
                  ? 'opacity-40'
                  : ''}"
              >
                <input
                  type="checkbox"
                  class="mt-0.5 size-3.5 accent-[var(--accent)]"
                  bind:checked={prefs.mergeNoCommit}
                  disabled={prefs.mergeSquash}
                />
                <span
                  ><span class="font-medium">Don't commit yet</span>
                  <span class="text-muted"
                    >Stop after merging so you can review before finishing.</span
                  ></span
                >
              </label>
            </div>
          </fieldset>
          {#if willMakeMergeCommit}
            <div>
              <label class="label" for="merge-msg">Merge commit message (optional)</label>
              <input
                id="merge-msg"
                class="input"
                bind:value={message}
                placeholder="Merge branch '{source}' into {current}"
              />
            </div>
          {/if}
        {:else}
          <div
            class="rounded-lg border p-3 text-[12.5px] {alreadyPushed
              ? 'border-del/40 bg-del/10'
              : 'border-border bg-surface-2'}"
          >
            <b>A rebase replays your commits on top of {source || 'the other branch'}</b>, giving a
            straight history with no merge commit. This rewrites those commits.
            {#if alreadyPushed}
              <span class="text-del">
                Your commits look already pushed: others using <b>{current}</b> would be affected, and
                you'd need a force push.</span
              >
            {:else}
              Safe for commits only you have.
            {/if}
          </div>
        {/if}
      </div>

      <div
        class="flex items-center justify-end gap-2 border-t border-border bg-surface-2 px-5 py-3"
      >
        <button class="btn btn-outline h-8" onclick={() => (ui.mergeDialog = null)}>Cancel</button>
        <button class="btn btn-primary h-8 px-4 font-semibold" disabled={nothingToDo} onclick={go}>
          {preview?.up_to_date ? 'Nothing to merge' : label}
        </button>
      </div>
    </div>
  </div>
{/if}
