<script lang="ts">
  import { onMount } from 'svelte'
  import { projects } from '../stores/projects.svelte'
  import { repo } from '../stores/repo.svelte'
  import { settings } from '../stores/settings.svelte'
  import { sync } from '../stores/sync.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  let now = $state(Date.now())
  let online = $state(navigator.onLine)
  onMount(() => {
    const timer = setInterval(() => (now = Date.now()), 15_000)
    return () => clearInterval(timer)
  })
  const fetch = $derived(sync.fetches[projects.current?.path ?? ''])
  const age = $derived(fetch?.at ? Math.max(0, Math.floor((now - fetch.at) / 60_000)) : null)
  const running = $derived(sync.running === 'fetch' && sync.activePath === projects.current?.path)
</script>

<svelte:window ononline={() => (online = true)} onoffline={() => (online = false)} />

<footer
  class="flex h-8 shrink-0 items-center gap-3 border-t border-border bg-surface-2 px-3 text-[11px] text-muted"
>
  <span class="flex min-w-0 items-center gap-1.5"
    ><Icon name="branch" size={12} /><span class="truncate"
      >{repo.summary?.branch ?? 'Detached HEAD'}</span
    ></span
  >
  {#if repo.summary?.has_upstream}
    <span title="Outgoing / incoming commits">↑ {repo.summary.ahead} ↓ {repo.summary.behind}</span>
  {/if}
  <button
    class="ml-auto truncate hover:text-fg {fetch?.error ? 'text-del' : ''}"
    disabled={!!sync.running}
    title={fetch?.error ?? 'Fetch all remotes now'}
    onclick={() => sync.run('fetch')}
  >
    {running
      ? 'Fetching…'
      : fetch?.error
        ? 'Fetch failed · click to retry'
        : age === null
          ? 'Not fetched this session'
          : age === 0
            ? 'Fetched just now'
            : `Fetched ${age}m ago`}
  </button>
  {#if !online}<span>Offline</span>{/if}
  {#if settings.value.auto_fetch_minutes > 0}
    <button
      class="hover:text-fg"
      title="Pause or resume automatic fetch for this session"
      aria-pressed={sync.paused}
      onclick={() => (sync.paused = !sync.paused)}
    >
      {sync.paused ? 'Auto-fetch paused' : `Auto-fetch · ${settings.value.auto_fetch_minutes}m`}
    </button>
  {:else}
    <button class="hover:text-fg" onclick={() => (ui.settingsOpen = true)}>Auto-fetch off</button>
  {/if}
</footer>
