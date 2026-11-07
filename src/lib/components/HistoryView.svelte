<script lang="ts">
  import CommitDetails from './CommitDetails.svelte'
  import CommitList from './CommitList.svelte'
  import Icon from './Icon.svelte'
  import SearchResults from './SearchResults.svelte'
  import { repo } from '../stores/repo.svelte'
  import { search } from '../stores/search.svelte'

  const KEY = 'whittle.detailsHeight'
  const MIN = 140

  function load(): number {
    try {
      const v = Number(localStorage.getItem(KEY))
      return v >= MIN ? v : 320
    } catch {
      return 320
    }
  }

  let height = $state(load())
  let container: HTMLDivElement | undefined = $state()
  let dragging = $state(false)
  let searchInput: HTMLInputElement | undefined = $state()

  function onkeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
      e.preventDefault()
      searchInput?.focus()
      searchInput?.select()
    }
  }

  function start(e: PointerEvent) {
    dragging = true
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }
  function move(e: PointerEvent) {
    if (!dragging || !container) return
    const rect = container.getBoundingClientRect()
    height = Math.round(Math.min(rect.height - 120, Math.max(MIN, rect.bottom - e.clientY)))
  }
  function end() {
    if (!dragging) return
    dragging = false
    try {
      localStorage.setItem(KEY, String(height))
    } catch {
      // size just won't be remembered
    }
  }
</script>

<svelte:window {onkeydown} />

<div bind:this={container} class="flex h-full min-h-0 flex-col">
  <div class="flex h-10 shrink-0 items-center gap-2 border-b border-border px-3">
    <Icon name="search" size={15} class="text-faint" />
    <input
      bind:this={searchInput}
      value={search.query}
      oninput={(e) => search.set(e.currentTarget.value)}
      onkeydown={(e) => e.key === 'Escape' && (search.clear(), searchInput?.blur())}
      placeholder="Search commits by message, author or hash…"
      spellcheck="false"
      class="h-full min-w-0 flex-1 bg-transparent text-[13px] outline-none placeholder:text-faint"
    />
    <button
      class="btn btn-outline h-6 px-2 text-[12px]"
      title="Scroll to the commit you are on (HEAD)"
      disabled={!repo.refs.head_id}
      onclick={() => {
        search.clear()
        repo.revealHead()
      }}><Icon name="commit" size={13} class="text-accent" /> Go to HEAD</button
    >
    {#if search.active}
      <button class="icon-btn icon-btn-sm" title="Clear search (Esc)" onclick={() => search.clear()}
        ><Icon name="x" size={14} /></button
      >
    {:else}
      <span class="kbd">Ctrl</span><span class="kbd -ml-1">F</span>
    {/if}
  </div>
  <div class="min-h-0 flex-1">
    {#if search.active}<SearchResults />{:else}<CommitList />{/if}
  </div>
  <!-- drag handle: a hairline with a wider invisible hit area; turns orange on hover / drag -->
  <div
    role="separator"
    aria-orientation="horizontal"
    aria-label="Resize details panel"
    class="group relative z-10 h-px shrink-0 bg-border"
  >
    <div
      role="presentation"
      class="absolute inset-x-0 -top-1.5 -bottom-1.5 cursor-row-resize touch-none"
      onpointerdown={start}
      onpointermove={move}
      onpointerup={end}
      onpointercancel={end}
    >
      <div
        class="absolute inset-x-0 top-1/2 h-0.5 -translate-y-1/2 bg-accent transition-opacity {dragging
          ? 'opacity-100'
          : 'opacity-0 group-hover:opacity-100'}"
      ></div>
    </div>
  </div>
  <div class="shrink-0" style="height: {height}px"><CommitDetails /></div>
</div>
