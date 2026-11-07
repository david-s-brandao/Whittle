<script lang="ts">
  import Markdown from './Markdown.svelte'

  let {
    placeholder = 'Leave a comment…',
    submitLabel = 'Comment',
    busy = false,
    autofocus = false,
    onsubmit,
    oncancel,
    children,
  }: {
    placeholder?: string
    submitLabel?: string
    busy?: boolean
    autofocus?: boolean
    /** Return true when the text was posted, so the box can be cleared. */
    onsubmit: (body: string) => Promise<boolean>
    oncancel?: () => void
    children?: import('svelte').Snippet
  } = $props()

  let body = $state('')
  let preview = $state(false)
  let box: HTMLTextAreaElement | undefined = $state()

  $effect(() => {
    if (autofocus) box?.focus()
  })

  async function submit() {
    if (!body.trim() || busy) return
    if (await onsubmit(body)) {
      body = ''
      preview = false
    }
  }
</script>

<div class="rounded-lg border border-border bg-surface focus-within:border-accent">
  <div class="flex items-center gap-1 border-b border-border px-2 pt-1.5">
    {#each [false, true] as p (p)}
      <button
        class="-mb-px rounded-t px-2.5 py-1 text-[12px] font-medium {preview === p
          ? 'border-b-2 border-accent text-fg'
          : 'text-muted hover:text-fg'}"
        onclick={() => (preview = p)}>{p ? 'Preview' : 'Write'}</button
      >
    {/each}
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
      bind:value={body}
      rows="4"
      {placeholder}
      class="block w-full resize-y bg-transparent px-3 py-2 text-[13px] outline-none placeholder:text-faint"
      onkeydown={(e) => {
        if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
          e.preventDefault()
          submit()
        }
      }}></textarea>
  {/if}
  <div class="flex items-center gap-2 border-t border-border px-2 py-1.5">
    <span class="text-[11px] text-faint">Markdown supported · Ctrl+Enter to send</span>
    <div class="ml-auto flex items-center gap-2">
      {@render children?.()}
      {#if oncancel}<button class="btn h-7" onclick={oncancel}>Cancel</button>{/if}
      <button
        class="btn btn-primary h-7 px-3 font-semibold"
        disabled={!body.trim() || busy}
        onclick={submit}
      >
        {busy ? 'Sending…' : submitLabel}
      </button>
    </div>
  </div>
</div>
