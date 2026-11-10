<script lang="ts">
  import CommitList from './CommitList.svelte'
  import Icon from './Icon.svelte'
  import SearchResults from './SearchResults.svelte'
  import { repo } from '../stores/repo.svelte'
  import { search } from '../stores/search.svelte'

  let searchInput: HTMLInputElement | undefined = $state()

  function onkeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
      e.preventDefault()
      searchInput?.focus()
      searchInput?.select()
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="flex h-full min-h-0 flex-col">
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
</div>
