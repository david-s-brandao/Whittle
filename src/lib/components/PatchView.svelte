<script lang="ts">
  import { parsePatch, type PatchLine } from '../patch'
  import type { PullFile, ReviewComment } from '../types'
  import CommentCard from './CommentCard.svelte'
  import Composer from './Composer.svelte'
  import Icon from './Icon.svelte'

  let {
    file,
    comments,
    busy,
    oncomment,
  }: {
    file: PullFile
    comments: ReviewComment[]
    busy: boolean
    /** Post a line comment (or a reply when `replyTo` is set). Resolves true on success. */
    oncomment: (
      line: number,
      side: 'LEFT' | 'RIGHT',
      body: string,
      replyTo: number | null,
    ) => Promise<boolean>
  } = $props()

  const hunks = $derived(parsePatch(file.patch))
  /** The key of the line a new comment is being written on, e.g. "RIGHT:12". */
  let composing = $state<string | null>(null)
  let replying = $state<number | null>(null)

  const bg = { '+': 'bg-[var(--add-bg)]', '-': 'bg-[var(--del-bg)]', ' ': '', '\\': '' } as const
  const sign = { '+': 'text-add', '-': 'text-del', ' ': '', '\\': '' } as const

  const sideOf = (l: PatchLine): 'LEFT' | 'RIGHT' => (l.kind === '-' ? 'LEFT' : 'RIGHT')
  const lineOf = (l: PatchLine) => (l.kind === '-' ? l.old_no : l.new_no)
  const keyOf = (side: string, line: number) => `${side}:${line}`

  // root comments by the line they sit on; replies by their root
  const threads = $derived.by(() => {
    const roots: Record<string, ReviewComment[]> = {}
    for (const c of comments) {
      if (c.in_reply_to !== null || c.line === null) continue
      ;(roots[keyOf(c.side, c.line)] ??= []).push(c)
    }
    return roots
  })
  const replies = $derived.by(() => {
    const r: Record<number, ReviewComment[]> = {}
    for (const c of comments) if (c.in_reply_to !== null) (r[c.in_reply_to] ??= []).push(c)
    return r
  })
  const outdated = $derived(comments.filter((c) => c.in_reply_to === null && c.line === null))
</script>

{#snippet thread(root: ReviewComment)}
  <div class="space-y-2 rounded-lg border border-border bg-surface p-3">
    <CommentCard author={root.author} time={root.created_at} body={root.body} url={root.url} />
    {#each replies[root.id] ?? [] as r (r.id)}
      <div class="ml-6">
        <CommentCard author={r.author} time={r.created_at} body={r.body} url={r.url} />
      </div>
    {/each}
    {#if replying === root.id}
      <div class="ml-6">
        <Composer
          placeholder="Reply…"
          submitLabel="Reply"
          {busy}
          autofocus
          oncancel={() => (replying = null)}
          onsubmit={async (b) => {
            const ok = await oncomment(root.line ?? 0, root.side, b, root.id)
            if (ok) replying = null
            return ok
          }}
        />
      </div>
    {:else}
      <button
        class="ml-6 text-[12px] text-accent hover:underline"
        onclick={() => (replying = root.id)}>Reply</button
      >
    {/if}
  </div>
{/snippet}

{#if !file.patch}
  <div class="grid h-full place-items-center p-6 text-center text-muted">
    No text diff is available for this file (it may be binary, renamed without changes, or too
    large).
  </div>
{:else}
  {#if outdated.length}
    <div class="space-y-2 border-b border-border bg-surface-2 p-3">
      <div class="section-label">Comments on code that has since changed</div>
      {#each outdated as c (c.id)}{@render thread(c)}{/each}
    </div>
  {/if}
  <div class="selectable min-w-max py-1 font-mono text-[12px] leading-[20px]">
    {#each hunks as h, hi (hi)}
      <div class="mt-1 bg-surface-2 px-4 text-[11px] text-faint first:mt-0">{h.header}</div>
      {#each h.lines as l, li (li)}
        {@const line = lineOf(l)}
        {@const side = sideOf(l)}
        {@const key = line === null ? '' : keyOf(side, line)}
        <div class="group flex {bg[l.kind]}">
          <span class="grid w-6 shrink-0 place-items-center select-none">
            {#if l.kind !== '\\' && line !== null}
              <button
                class="invisible size-4 rounded bg-accent text-white group-hover:visible"
                title="Comment on this line"
                aria-label="Comment on this line"
                onclick={() => (composing = key)}><Icon name="plus" size={12} /></button
              >
            {/if}
          </span>
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
        {#if key && (threads[key]?.length || composing === key)}
          <div class="sticky left-0 max-w-[52rem] space-y-2 px-4 py-2 font-sans text-[13px]">
            {#each threads[key] ?? [] as root (root.id)}{@render thread(root)}{/each}
            {#if composing === key && line !== null}
              <Composer
                placeholder="Comment on line {line}…"
                {busy}
                autofocus
                oncancel={() => (composing = null)}
                onsubmit={async (b) => {
                  const ok = await oncomment(line, side, b, null)
                  if (ok) composing = null
                  return ok
                }}
              />
            {/if}
          </div>
        {/if}
      {/each}
    {/each}
  </div>
{/if}
