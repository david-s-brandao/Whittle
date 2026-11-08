<script lang="ts">
  import { api } from '../api'
  import { formatIso, isoToUnix, relativeIso, splitPath } from '../format'
  import { detail } from '../stores/forge.svelte'
  import { ui } from '../stores/ui.svelte'
  import type { ForgeComment, ForgeReview, ReviewComment } from '../types'
  import CommentCard from './CommentCard.svelte'
  import Composer from './Composer.svelte'
  import Icon from './Icon.svelte'
  import LabelChip from './LabelChip.svelte'
  import PatchView from './PatchView.svelte'
  import ReviewDialog from './ReviewDialog.svelte'
  import StateIcon from './StateIcon.svelte'

  let tab = $state<'conversation' | 'files'>('conversation')
  let reviewOpen = $state(false)
  let selectedFile = $state<string | null>(null)
  let fileQuery = $state('')
  let composer = $state<ReturnType<typeof Composer>>()
  const quote = (author: string, body: string) => {
    void composer?.quote(author, body)
  }

  const d = $derived(detail.pull)
  const threadKey = $derived(d?.summary.url)

  // a new pull request starts on the conversation
  $effect(() => {
    void threadKey
    tab = 'conversation'
    selectedFile = null
    fileQuery = ''
  })
  $effect(() => {
    if (tab === 'files') detail.loadFiles()
  })

  type Item =
    | { kind: 'comment'; time: string; c: ForgeComment }
    | { kind: 'review'; time: string; r: ForgeReview; inline: ReviewComment[] }

  const timeline = $derived.by<Item[]>(() => {
    if (!d) return []
    const inlineBy: Record<number, ReviewComment[]> = {}
    for (const c of d.review_comments) {
      if (c.review_id !== null && c.in_reply_to === null) (inlineBy[c.review_id] ??= []).push(c)
    }
    const items: Item[] = [
      ...d.comments.map((c): Item => ({ kind: 'comment', time: c.created_at, c })),
      ...d.reviews
        .filter(
          (r) =>
            r.state !== 'PENDING' && (r.body.trim() || r.state !== 'COMMENTED' || inlineBy[r.id]),
        )
        .map((r): Item => ({
          kind: 'review',
          time: r.submitted_at,
          r,
          inline: inlineBy[r.id] ?? [],
        })),
    ]
    return items.sort((a, b) => isoToUnix(a.time) - isoToUnix(b.time))
  })

  const verdict = {
    APPROVED: { text: 'approved', tone: 'good' },
    CHANGES_REQUESTED: { text: 'requested changes', tone: 'bad' },
    COMMENTED: { text: 'reviewed', tone: 'neutral' },
    DISMISSED: { text: 'dismissed', tone: 'neutral' },
    PENDING: { text: 'pending', tone: 'neutral' },
  } as const

  const mergeInfo = $derived.by(() => {
    switch (d?.mergeable_state) {
      case 'clean':
        return { text: 'Ready to merge', cls: 'text-add' }
      case 'dirty':
        return { text: 'Has merge conflicts', cls: 'text-del' }
      case 'unstable':
        return { text: 'Some checks are failing', cls: 'text-del' }
      case 'blocked':
        return { text: 'Blocked: needs review or checks', cls: 'text-muted' }
      case 'behind':
        return { text: 'Behind the base branch', cls: 'text-muted' }
      case 'draft':
        return { text: 'Draft', cls: 'text-muted' }
      default:
        return null
    }
  })

  const files = $derived(detail.files)
  const visibleFiles = $derived(
    files?.filter((f) => f.filename.toLowerCase().includes(fileQuery.trim().toLowerCase())) ?? [],
  )
  const current = $derived(
    visibleFiles.find((f) => f.filename === selectedFile) ?? visibleFiles[0] ?? null,
  )
  const countFor = (path: string) =>
    d?.review_comments.filter((c) => c.path === path && c.in_reply_to === null).length ?? 0
</script>

{#if d}
  {@const s = d.summary}
  <div class="flex h-full min-h-0 flex-col">
    <div class="border-b border-border px-6 pt-4">
      <div class="flex flex-wrap items-start gap-3">
        <StateIcon kind="pr" state={s.state} draft={s.draft} size={20} />
        <div class="min-w-0 flex-1">
          <h2 class="selectable text-[17px] leading-snug font-semibold">
            {s.title} <span class="font-normal text-faint">#{s.number}</span>
          </h2>
          <div class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-[12.5px] text-muted">
            <span
              class="rounded-full px-2 py-0.5 text-[11.5px] font-medium text-white {s.state ===
              'open'
                ? s.draft
                  ? 'bg-faint'
                  : 'bg-add'
                : s.state === 'merged'
                  ? 'bg-[#8250df]'
                  : 'bg-del'}"
              >{s.draft && s.state === 'open'
                ? 'Draft'
                : s.state[0].toUpperCase() + s.state.slice(1)}</span
            >
            <span><b class="text-fg">{s.author}</b> wants to merge</span>
            <span class="rounded bg-hover px-1.5 py-px font-mono text-[11.5px] text-fg"
              >{s.head_ref}</span
            >
            <span>into</span>
            <span class="rounded bg-hover px-1.5 py-px font-mono text-[11.5px] text-fg"
              >{s.base_ref}</span
            >
            <span title={formatIso(s.created_at)}>· {relativeIso(s.created_at)}</span>
          </div>
          <div class="mt-1.5 flex flex-wrap items-center gap-x-3 text-[12px] text-muted">
            <span>{d.commits} commit{d.commits === 1 ? '' : 's'}</span>
            <span>{d.changed_files} file{d.changed_files === 1 ? '' : 's'}</span>
            <span class="text-add">+{d.additions}</span>
            <span class="text-del">−{d.deletions}</span>
            {#if mergeInfo && s.state === 'open'}<span class="font-medium {mergeInfo.cls}"
                >{mergeInfo.text}</span
              >{/if}
          </div>
          {#if s.labels.length}
            <div class="mt-2 flex flex-wrap gap-1.5">
              {#each s.labels as l (l.name)}<LabelChip label={l} />{/each}
            </div>
          {/if}
        </div>
        <div class="flex w-full flex-wrap items-center justify-end gap-1.5">
          {#if tab === 'conversation'}<button class="btn h-8" onclick={() => composer?.focus()}
              >Write a comment</button
            >{/if}
          <button
            class="icon-btn"
            title="Refresh discussion"
            disabled={detail.refreshing || detail.busy}
            onclick={() => detail.refresh()}
            ><Icon name="refresh" class={detail.refreshing ? 'animate-spin' : ''} /></button
          >
          {#if s.state === 'open'}
            <button class="btn btn-primary h-8 font-semibold" onclick={() => (reviewOpen = true)}
              >Review changes</button
            >
          {/if}
          <button
            class="btn btn-outline h-8"
            title="Fetch this pull request and switch to a local branch for it"
            disabled={detail.busy}
            onclick={() => detail.checkout()}>Check out</button
          >
          <button
            class="icon-btn"
            title="Open on the website"
            onclick={() =>
              api.openUrl(s.url).catch((e) => (ui.notice = { kind: 'error', text: String(e) }))}
            ><Icon name="external" /></button
          >
        </div>
      </div>
      <div class="mt-3 flex gap-1">
        {#each [['conversation', 'Conversation', 'message'], ['files', 'Files changed', 'file']] as const as [id, label, icon] (id)}
          <button
            class="-mb-px flex h-9 items-center gap-2 border-b-2 px-3 text-[13px] font-medium {tab ===
            id
              ? 'border-accent text-fg'
              : 'border-transparent text-muted hover:text-fg'}"
            onclick={() => (tab = id)}
          >
            <Icon name={icon} size={14} />{label}
            {#if id === 'files'}<span class="chip">{d.changed_files}</span>{/if}
          </button>
        {/each}
      </div>
    </div>

    {#if tab === 'conversation'}
      <div class="min-h-0 flex-1 overflow-y-auto px-4 py-5 lg:px-6">
        <div class="mx-auto w-full max-w-3xl space-y-4">
          <CommentCard
            author={s.author}
            time={s.created_at}
            body={d.body}
            url={s.url}
            onquote={quote}
          />
          {#each timeline as it (it.kind + (it.kind === 'comment' ? it.c.id : it.r.id))}
            {#if it.kind === 'comment'}
              <CommentCard
                author={it.c.author}
                time={it.c.created_at}
                body={it.c.body}
                association={it.c.association}
                url={it.c.url}
                onquote={quote}
              />
            {:else}
              <div class="space-y-2">
                <CommentCard
                  author={it.r.author}
                  time={it.r.submitted_at}
                  body={it.r.body}
                  url={it.r.url}
                  badge={verdict[it.r.state]}
                  onquote={quote}
                />
                {#each it.inline as c (c.id)}
                  <button
                    class="ml-11 block w-[calc(100%-2.75rem)] rounded-lg border border-border bg-surface-2 px-3 py-2 text-left hover:border-faint"
                    title="Show in the diff"
                    onclick={() => {
                      selectedFile = c.path
                      tab = 'files'
                    }}
                  >
                    <div class="font-mono text-[11.5px] text-muted">
                      {c.path}{c.line ? `:${c.line}` : ''}
                    </div>
                    <div class="mt-0.5 line-clamp-2 text-[12.5px]">{c.body}</div>
                  </button>
                {/each}
              </div>
            {/if}
          {/each}
          <div class="mx-auto w-full pt-2">
            {#key s.url}
              <Composer
                bind:this={composer}
                draftKey={s.url}
                busy={detail.busy || detail.refreshing}
                onsubmit={(b) => detail.comment(b)}
              >
                <button
                  class="btn btn-outline h-7"
                  disabled={detail.busy || s.state === 'merged'}
                  onclick={() => detail.setState(s.state === 'open' ? 'closed' : 'open')}
                >
                  {s.state === 'open'
                    ? 'Close pull request'
                    : s.state === 'merged'
                      ? 'Merged'
                      : 'Reopen'}
                </button>
              </Composer>
            {/key}
          </div>
        </div>
      </div>
    {:else}
      <div class="flex min-h-0 flex-1">
        <div class="flex w-72 shrink-0 flex-col border-r border-border bg-surface-2">
          <div class="section-label px-4 py-2">{files?.length ?? d.changed_files} files</div>
          <div class="px-3 pb-2">
            <input
              class="input h-8"
              aria-label="Filter pull request files"
              placeholder="Filter files by path…"
              bind:value={fileQuery}
            />
          </div>
          <ul class="min-h-0 flex-1 overflow-y-auto pb-2">
            {#if detail.filesLoading && !files}
              <li class="px-4 py-2 text-[12px] text-muted">Loading files…</li>
            {/if}
            {#each visibleFiles as f (f.filename)}
              {@const [dir, name] = splitPath(f.filename)}
              {@const n = countFor(f.filename)}
              <li>
                <button
                  class="flex w-full items-center gap-2 px-4 py-1 text-left text-[12.5px] {current?.filename ===
                  f.filename
                    ? 'bg-accent/10'
                    : 'hover:bg-hover'}"
                  title={f.previous_filename
                    ? `${f.previous_filename} → ${f.filename}`
                    : f.filename}
                  onclick={() => (selectedFile = f.filename)}
                >
                  <span class="min-w-0 flex-1 truncate"
                    ><span class="text-faint">{dir}</span><span class="font-medium">{name}</span
                    ></span
                  >
                  {#if n}<span
                      class="chip chip-accent"
                      title="{n} comment thread{n === 1 ? '' : 's'}">{n}</span
                    >{/if}
                  <span class="shrink-0 text-[11px] text-add">+{f.additions}</span>
                  <span class="shrink-0 text-[11px] text-del">−{f.deletions}</span>
                </button>
              </li>
            {/each}
          </ul>
        </div>
        <div class="min-w-0 flex-1 overflow-auto">
          {#if current}
            {#key current.filename}
              <PatchView
                file={current}
                comments={d.review_comments.filter((c) => c.path === current.filename)}
                busy={detail.busy}
                oncomment={(line, side, body, replyTo) =>
                  detail.lineComment(current.filename, line, side, body, replyTo)}
              />
            {/key}
          {:else if !detail.filesLoading}
            <div class="grid h-full place-items-center text-muted">
              {fileQuery ? 'No files match this filter.' : 'No files.'}
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
  <ReviewDialog bind:open={reviewOpen} />
{/if}
