<script lang="ts">
  import { tick, untrack } from 'svelte'
  import {
    completeCommentDraft,
    formatSelection,
    readCommentDraft,
    saveCommentDraft,
    type MarkdownAction,
  } from '../commentDrafts'
  import Markdown from './Markdown.svelte'

  let {
    placeholder = 'Leave a comment…',
    submitLabel = 'Comment',
    busy = false,
    autofocus = false,
    onsubmit,
    oncancel,
    children,
    draftKey,
  }: {
    placeholder?: string
    submitLabel?: string
    busy?: boolean
    autofocus?: boolean
    /** Return true when the text was posted, so the box can be cleared. */
    onsubmit: (body: string) => Promise<boolean>
    oncancel?: () => void
    children?: import('svelte').Snippet
    draftKey?: string
  } = $props()

  const key = untrack(() => draftKey)
  let body = $state(key ? readCommentDraft(key) : '')
  let preview = $state(false)
  let sending = $state(false)
  let error = $state('')
  const disabled = $derived(busy || sending)
  let box: HTMLTextAreaElement | undefined = $state()

  $effect(() => {
    if (autofocus) box?.focus()
  })

  async function submit() {
    if (!body.trim() || disabled) return
    const submitted = body
    sending = true
    error = ''
    try {
      if (await onsubmit(submitted)) {
        if (key) completeCommentDraft(key, submitted)
        body = ''
        preview = false
      }
    } catch (e) {
      error = String(e)
    } finally {
      sending = false
    }
  }

  function update(value: string) {
    body = value
    if (key) saveCommentDraft(key, value)
  }

  export async function focus() {
    preview = false
    await tick()
    box?.scrollIntoView({ block: 'center', behavior: 'smooth' })
    box?.focus({ preventScroll: true })
  }

  export async function quote(author: string, text: string) {
    if (disabled) return
    update(
      `${body}${body ? '\n\n' : ''}@${author} wrote:\n${text
        .split('\n')
        .map((line) => `> ${line}`)
        .join('\n')}\n\n`,
    )
    await focus()
    box?.setSelectionRange(body.length, body.length)
  }

  async function format(action: MarkdownAction) {
    if (disabled || preview) return
    const formatted = formatSelection(
      body,
      box?.selectionStart ?? body.length,
      box?.selectionEnd ?? body.length,
      action,
    )
    update(formatted.body)
    await tick()
    box?.focus()
    box?.setSelectionRange(formatted.start, formatted.end)
  }
</script>

<div class="rounded-lg border border-border bg-surface focus-within:border-accent">
  <div class="flex flex-wrap items-center gap-1 border-b border-border px-2 pt-1.5">
    {#each [false, true] as p (p)}
      <button
        class="-mb-px rounded-t px-2.5 py-1 text-[12px] font-medium {preview === p
          ? 'border-b-2 border-accent text-fg'
          : 'text-muted hover:text-fg'}"
        onclick={() => (preview = p)}>{p ? 'Preview' : 'Write'}</button
      >
    {/each}
    <div class="ml-auto flex items-center gap-0.5 pb-1">
      {#each [['bold', 'B', 'Bold (Ctrl+B)'], ['italic', 'I', 'Italic (Ctrl+I)'], ['code', '</>', 'Inline code'], ['link', 'Link', 'Insert link'], ['task', '☑', 'Task list']] as const as [action, label, title] (action)}
        <button
          class="btn h-6 px-2 text-[11px]"
          {title}
          aria-label={title}
          disabled={disabled || preview}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => format(action)}>{label}</button
        >
      {/each}
    </div>
  </div>
  {#if preview}
    <div class="min-h-24 px-3 py-2">
      {#if body.trim()}<Markdown source={body} />{:else}<span class="text-muted"
          >Nothing to preview.</span
        >{/if}
    </div>
  {:else}
    <textarea
      bind:this={box}
      value={body}
      oninput={(e) => update(e.currentTarget.value)}
      {disabled}
      aria-label={placeholder}
      rows="4"
      {placeholder}
      class="block w-full resize-y bg-transparent px-3 py-2 text-[13px] outline-none placeholder:text-faint"
      onkeydown={(e) => {
        if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
          e.preventDefault()
          submit()
        } else if ((e.ctrlKey || e.metaKey) && ['b', 'i'].includes(e.key.toLowerCase())) {
          e.preventDefault()
          e.stopPropagation()
          void format(e.key.toLowerCase() === 'b' ? 'bold' : 'italic')
        }
      }}></textarea>
  {/if}
  {#if error}<p role="alert" class="px-3 py-2 text-[12px] text-del">{error}</p>{/if}
  <div class="flex flex-wrap items-center gap-2 border-t border-border px-3 py-2">
    <span class="text-[11px] text-faint"
      >{key && body ? 'Draft kept for this session · ' : ''}Ctrl+Enter to send</span
    >
    <div class="ml-auto flex flex-wrap items-center gap-2">
      {@render children?.()}
      {#if oncancel}<button class="btn h-7" onclick={oncancel}>Cancel</button>{/if}
      <button
        class="btn btn-primary h-7 px-3 font-semibold"
        disabled={!body.trim() || disabled}
        onclick={submit}
      >
        {sending ? 'Sending…' : submitLabel}
      </button>
    </div>
  </div>
</div>
