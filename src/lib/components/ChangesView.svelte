<script lang="ts">
  import { plural, splitPath, statusBadge } from '../format'
  import { changes } from '../stores/changes.svelte'
  import { projects } from '../stores/projects.svelte'
  import { repo } from '../stores/repo.svelte'
  import { ui } from '../stores/ui.svelte'
  import type { FileChange } from '../types'
  import ConflictPreview from './ConflictPreview.svelte'
  import DiffView from './DiffView.svelte'
  import Icon from './Icon.svelte'

  const st = $derived(changes.status)
  const branch = $derived(
    repo.summary?.branch ?? projects.summaries[projects.current?.path ?? '']?.branch,
  )
  // line/hunk staging only makes sense for edits to existing files; new, deleted and renamed
  // files are staged as a whole with the row buttons.
  const interactive = $derived.by(() => {
    const sel = changes.selection
    if (!sel) return false
    const list = sel.staged ? st.staged : st.unstaged
    return list.find((f) => f.path === sel.path)?.status === 'modified'
  })
  // git swaps ours/theirs during a rebase, so name the sides by what they mean to the user
  const ours = $derived(st.operation === 'rebase' ? 'Upstream' : 'Current branch')
  const theirs = $derived(
    st.operation === 'rebase'
      ? 'Your commit'
      : st.operation === 'cherry-pick'
        ? 'Picked commit'
        : st.operation === 'revert'
          ? 'Reverted state'
          : 'Incoming',
  )
  const oursBtn = $derived(st.operation === 'rebase' ? 'Keep upstream' : 'Keep current')
  const theirsBtn = $derived(st.operation === 'rebase' ? 'Take yours' : 'Take incoming')
  const subject = $derived(changes.message.split('\n')[0] ?? '')
  const commitLabel = $derived(
    changes.amend
      ? 'Amend last commit'
      : st.staged.length
        ? `Commit ${plural(st.staged.length, 'file')}`
        : 'Commit',
  )

  function onkeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault()
      commit()
    }
  }
  async function commit() {
    await changes.commit()
    repo.refresh() // new commit shows up in History + sidebar
  }
</script>

{#snippet section(title: string, files: FileChange[], staged: boolean)}
  <div class="flex h-9 items-center gap-2 px-4">
    <span class="section-label">{title}</span>
    <span class="chip {files.length ? 'chip-accent' : ''}">{files.length}</span>
    {#if files.length}
      <div class="ml-auto flex items-center gap-1">
        {#if staged}
          <button
            class="btn h-6 px-2 text-[12px]"
            disabled={changes.busy}
            onclick={() => changes.unstage(files.map((f) => f.path))}>Unstage all</button
          >
        {:else}
          <button
            class="btn btn-danger h-6 px-2 text-[12px]"
            disabled={changes.busy}
            onclick={() => changes.discard(files.map((f) => f.path))}>Discard all</button
          >
          <button
            class="btn btn-primary h-6 px-2 text-[12px]"
            disabled={changes.busy}
            onclick={() => changes.stage(files.map((f) => f.path))}>Stage all</button
          >
        {/if}
      </div>
    {/if}
  </div>
  {#each files as f (f.path)}
    {@const [letter, color, label] = statusBadge(f.status)}
    {@const [dir, name] = splitPath(f.path)}
    {@const sel = changes.selection?.path === f.path && changes.selection.staged === staged}
    <div
      class="group flex h-8 items-center gap-1 pr-2 pl-4 text-[12.5px] {sel
        ? 'bg-accent/10 shadow-[inset_2px_0_0_var(--accent)]'
        : 'hover:bg-hover'}"
    >
      <button
        class="flex min-w-0 flex-1 items-center gap-2 text-left"
        title={f.old_path ? `${f.old_path} → ${f.path}` : f.path}
        onclick={() => changes.select(f.path, staged)}
      >
        <span class="w-3 shrink-0 font-mono text-[11px] font-bold {color}" title={label}
          >{letter}</span
        >
        <span class="truncate"
          ><span class="text-faint">{dir}</span><span class="font-medium">{name}</span></span
        >
      </button>
      {#if f.status !== 'untracked' && f.status !== 'added'}
        <button
          class="icon-btn icon-btn-sm invisible group-hover:visible"
          title="Blame / history of this file"
          onclick={() => (ui.fileView = { path: f.path, rev: null, tab: 'blame' })}
          ><Icon name="blame" size={13} /></button
        >
      {/if}
      {#if staged}
        <button
          class="icon-btn icon-btn-sm hover:text-accent"
          title="Unstage this file"
          disabled={changes.busy}
          onclick={() => changes.unstage([f.path])}><Icon name="minus" size={14} /></button
        >
      {:else}
        <button
          class="icon-btn icon-btn-sm invisible group-hover:visible hover:text-del"
          title="Discard changes to this file"
          disabled={changes.busy}
          onclick={() => changes.discard([f.path])}><Icon name="undo" size={14} /></button
        >
        <button
          class="icon-btn icon-btn-sm hover:text-accent"
          title="Stage this file"
          disabled={changes.busy}
          onclick={() => changes.stage([f.path])}><Icon name="plus" size={14} /></button
        >
      {/if}
    </div>
  {/each}
  {#if !files.length}
    <div class="px-4 pb-2 text-[12px] text-faint">
      {staged
        ? 'Nothing staged yet: stage files to include them in the next commit.'
        : 'No unstaged changes.'}
    </div>
  {/if}
{/snippet}

{#snippet conflicts()}
  {#if st.conflicted.length}
    <div class="flex h-9 items-center gap-2 px-4">
      <span class="section-label text-del">Conflicts</span>
      <span class="chip bg-del text-white">{st.conflicted.length}</span>
    </div>
    <div class="mx-4 mb-2 rounded-md bg-del/10 px-3 py-2 text-[11.5px] text-muted">
      <b class="text-fg">{ours}</b> is what you had, <b class="text-fg">{theirs}</b> is what is being
      brought in. Pick a side for a file, or open it, fix the markers yourself and mark it resolved.
    </div>
    {#each st.conflicted as file (file)}
      {@const [dir, name] = splitPath(file)}
      {@const sel = changes.selection?.conflict && changes.selection.path === file}
      <div
        class="mb-1 {sel ? 'bg-accent/10 shadow-[inset_2px_0_0_var(--accent)]' : 'hover:bg-hover'}"
      >
        <button
          class="flex h-7 w-full items-center gap-2 px-4 text-left text-[12.5px]"
          title={file}
          onclick={() => changes.selectConflict(file)}
        >
          <span class="w-3 shrink-0 font-mono text-[11px] font-bold text-del" title="Conflicted"
            >!</span
          >
          <span class="truncate"
            ><span class="text-faint">{dir}</span><span class="font-medium">{name}</span></span
          >
        </button>
        <div class="flex flex-wrap items-center gap-1 px-4 pb-1.5 pl-9">
          <button
            class="btn btn-outline h-6 px-2 text-[11.5px]"
            title="Keep your version of this file ({ours})"
            disabled={changes.busy}
            onclick={() => changes.resolve(file, 'ours')}>{oursBtn}</button
          >
          <button
            class="btn btn-outline h-6 px-2 text-[11.5px]"
            title="Take the incoming version of this file ({theirs})"
            disabled={changes.busy}
            onclick={() => changes.resolve(file, 'theirs')}>{theirsBtn}</button
          >
          <button
            class="btn h-6 px-2 text-[11.5px]"
            title="Open in your editor to fix it by hand"
            onclick={() => changes.openInEditor(file)}
            ><Icon name="external" size={12} /> Open</button
          >
          <button
            class="btn btn-primary h-6 px-2 text-[11.5px]"
            title="I fixed it by hand: mark it as resolved"
            disabled={changes.busy}
            onclick={() => changes.markResolved(file)}>Mark resolved</button
          >
        </div>
      </div>
    {/each}
    <div class="my-1 border-t border-border"></div>
  {/if}
{/snippet}

<div class="flex h-full min-h-0">
  <div class="flex w-[24rem] shrink-0 flex-col border-r border-border bg-surface-2">
    <div class="min-h-0 flex-1 overflow-y-auto pt-1 pb-2">
      {@render conflicts()}
      {@render section('Unstaged', st.unstaged, false)}
      <div class="my-1 border-t border-border"></div>
      {@render section('Staged', st.staged, true)}
    </div>

    <!-- Commit box -->
    <div class="border-t border-border bg-surface p-3">
      <div class="mb-2 flex items-center justify-between text-[12px] text-muted">
        <span class="flex items-center gap-1.5">
          <Icon name="branch" size={13} class="text-accent" />
          Committing to <b class="font-semibold text-fg">{branch ?? 'HEAD'}</b>
        </span>
        <span class={subject.length > 72 ? 'text-del' : subject.length > 50 ? 'text-accent' : ''}
          >{subject.length}/50</span
        >
      </div>
      <textarea
        bind:value={changes.message}
        {onkeydown}
        rows="3"
        placeholder={changes.amend
          ? 'New message (leave empty to keep the old one)'
          : 'Commit message (first line is the summary)'}
        class="input h-auto resize-none py-2 leading-snug"></textarea>
      <label class="mt-2.5 flex cursor-pointer items-center gap-2 text-[12.5px] select-none">
        <input
          type="checkbox"
          class="size-3.5 accent-[var(--accent)]"
          checked={changes.amend}
          onchange={(e) => changes.setAmend(e.currentTarget.checked)}
        />
        Amend last commit
      </label>
      <button
        class="btn btn-primary btn-lg mt-3 w-full justify-center font-semibold"
        disabled={!changes.canCommit}
        onclick={commit}
      >
        <Icon name="commit" />
        {changes.busy ? 'Working…' : commitLabel}
      </button>
      <div class="mt-2 min-h-4 text-[12px]">
        {#if changes.error}
          <span class="text-del">{changes.error}</span>
        {:else if changes.notice}
          <span class="text-add">{changes.notice}</span>
        {:else if changes.blocker}
          <span class="text-muted">{changes.blocker}</span>
        {:else}
          <span class="text-faint"
            ><span class="kbd">Ctrl</span> <span class="kbd">Enter</span> to commit</span
          >
        {/if}
      </div>
    </div>
  </div>

  <div class="min-w-0 flex-1 overflow-auto">
    {#if changes.selection?.conflict && changes.conflictText !== null}
      <ConflictPreview text={changes.conflictText} {ours} {theirs} />
    {:else if changes.diff}
      <DiffView
        diff={changes.diff}
        actions={interactive && changes.selection
          ? {
              staged: changes.selection.staged,
              busy: changes.busy,
              apply: (sels) => changes.applySelection(sels),
            }
          : undefined}
      />
    {:else}
      <div class="grid h-full place-items-center text-muted">
        {changes.total
          ? 'Select a file to preview its changes.'
          : 'Working tree clean. Nothing to commit.'}
      </div>
    {/if}
  </div>
</div>
