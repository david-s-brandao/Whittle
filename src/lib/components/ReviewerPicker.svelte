<script lang="ts">
  import { people } from '../stores/forge.svelte'
  import Avatar from './Avatar.svelte'
  import Icon from './Icon.svelte'

  let {
    selected,
    exclude = [],
    disabled = false,
    onchange,
  }: {
    selected: string[]
    /** People who can't review (the author of the pull request). */
    exclude?: string[]
    disabled?: boolean
    onchange: (next: string[]) => void
  } = $props()

  let open = $state(false)
  let query = $state('')

  // active committers, plus anyone already asked so they can always be removed
  const options = $derived(
    [...new Set([...selected, ...people.list.filter((n) => !exclude.includes(n))])].filter((n) =>
      n.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  )

  function toggle(name: string) {
    onchange(selected.includes(name) ? selected.filter((n) => n !== name) : [...selected, name])
  }
  function show() {
    open = !open
    query = ''
    if (open) void people.load()
  }
</script>

<div class="relative">
  <div class="flex flex-wrap items-center gap-1.5">
    {#each selected as n (n)}
      <span class="inline-flex items-center gap-1.5 rounded-full bg-hover py-0.5 pr-2.5 pl-0.5">
        <Avatar name={n} size={18} /><span class="text-[12px]">{n}</span>
      </span>
    {:else}
      <span class="text-[12px] text-faint">No reviewers requested</span>
    {/each}
    <button class="icon-btn" title="Request reviewers" {disabled} onclick={show}>
      <Icon name="plus" size={14} />
    </button>
  </div>

  {#if open}
    <div class="fixed inset-0 z-40" role="presentation" onmousedown={() => (open = false)}></div>
    <div
      class="absolute z-50 mt-1 w-64 overflow-hidden rounded-lg border border-border bg-surface shadow-xl"
      role="dialog"
      aria-label="Request reviewers"
      tabindex="-1"
      onkeydown={(e) => e.key === 'Escape' && (open = false)}
    >
      <div class="border-b border-border p-2">
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="input h-8"
          placeholder="Filter people…"
          bind:value={query}
          autofocus
          spellcheck="false"
        />
      </div>
      <div class="px-3 pt-2 text-[11px] text-faint">Committed in the last 90 days</div>
      <ul class="max-h-56 overflow-y-auto py-1">
        {#if people.loading && !people.list.length}
          <li class="px-3 py-2 text-[12px] text-muted">Loading…</li>
        {:else if people.error}
          <li class="px-3 py-2 text-[12px] text-del">{people.error}</li>
        {/if}
        {#each options as n (n)}
          <li>
            <button
              class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[12.5px] hover:bg-hover"
              onclick={() => toggle(n)}
            >
              <Avatar name={n} size={20} />
              <span class="min-w-0 flex-1 truncate">{n}</span>
              {#if selected.includes(n)}<Icon name="check" size={14} class="text-accent" />{/if}
            </button>
          </li>
        {:else}
          {#if !people.loading && !people.error}
            <li class="px-3 py-2 text-[12px] text-muted">No one to ask.</li>
          {/if}
        {/each}
      </ul>
    </div>
  {/if}
</div>
