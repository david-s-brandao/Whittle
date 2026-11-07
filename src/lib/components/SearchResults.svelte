<script lang="ts">
  import { formatDate, highlightParts, relativeTime } from '../format'
  import { repo } from '../stores/repo.svelte'
  import { search } from '../stores/search.svelte'
  import { ui } from '../stores/ui.svelte'

  const rows = $derived(search.results)
</script>

{#snippet hl(text: string)}
  {#each highlightParts(text, search.query) as p, i (i)}
    {#if p.hit}<mark class="rounded-sm bg-accent/25 px-0.5 text-inherit">{p.text}</mark
      >{:else}{p.text}{/if}
  {/each}
{/snippet}

<div class="flex h-full min-h-0 flex-col">
  <div class="section-label flex h-8 shrink-0 items-center gap-4 border-b border-border px-4">
    <span class="flex-1">
      {#if search.loading}Searching…{:else}{rows.length}{rows.length >= 300 ? '+' : ''} match{rows.length ===
        1
          ? ''
          : 'es'}{/if}
    </span>
    <span class="w-36">Author</span>
    <span class="w-24 text-right">Date</span>
    <span class="w-16">Hash</span>
  </div>
  <div class="min-h-0 flex-1 overflow-y-auto" role="listbox" aria-label="Search results">
    {#each rows as c (c.id)}
      {@const sel = repo.selected?.id === c.id}
      <div
        role="option"
        aria-selected={sel}
        tabindex="-1"
        class="flex h-[34px] cursor-default items-center gap-4 px-4 text-[13px] {sel
          ? 'bg-accent/10 shadow-[inset_2px_0_0_var(--accent)]'
          : 'hover:bg-hover'}"
        onclick={() => repo.selectCommit(c)}
        oncontextmenu={(e) => {
          e.preventDefault()
          ui.commitMenu = { commit: c, x: e.clientX, y: e.clientY }
        }}
        onkeydown={null}
      >
        <span class="flex-1 truncate" title={c.summary}>{@render hl(c.summary)}</span>
        <span class="w-36 truncate text-muted" title={c.email}>{@render hl(c.author)}</span>
        <span class="w-24 text-right text-[12px] text-muted" title={formatDate(c.time)}
          >{relativeTime(c.time)}</span
        >
        <span class="w-16 font-mono text-[11.5px] text-faint">{@render hl(c.short_id)}</span>
      </div>
    {/each}
    {#if !rows.length && !search.loading}
      <div class="grid h-full place-items-center px-6 text-center text-muted">
        {search.error ?? `No commits match “${search.query.trim()}”.`}
      </div>
    {/if}
  </div>
</div>
