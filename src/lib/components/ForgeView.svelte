<script lang="ts">
  import { untrack } from 'svelte'
  import { relativeIso } from '../format'
  import { detail, forge, issues, pulls } from '../stores/forge.svelte'
  import ConnectPanel from './ConnectPanel.svelte'
  import Icon from './Icon.svelte'
  import IssueDetailView from './IssueDetailView.svelte'
  import LabelChip from './LabelChip.svelte'
  import PullDetailView from './PullDetailView.svelte'
  import StateIcon from './StateIcon.svelte'

  let { kind }: { kind: 'issues' | 'pulls' } = $props()
  let showConnect = $state(false)

  const list = $derived(kind === 'issues' ? issues : pulls)
  const noun = $derived(kind === 'issues' ? 'issues' : 'pull requests')
  const s = $derived(forge.status)

  // load the list the first time this tab is opened (and when the connection becomes ready)
  $effect(() => {
    const ready = forge.ready
    const l = list
    if (ready && !l.loaded && !l.loading && !l.error) untrack(() => l.reload())
  })
  // a failed request that sign-in would fix: offer it right away
  $effect(() => {
    if (list.error && s && !s.connected) showConnect = true
  })
  // after signing in, load again
  let wasConnected = false
  $effect(() => {
    const c = !!s?.connected
    if (c && !wasConnected) {
      untrack(() => {
        showConnect = false
        void list.reload()
      })
    }
    wasConnected = c
  })

  function open(n: number) {
    if (kind === 'issues') void detail.openIssue(n)
    else void detail.openPull(n)
  }
  const filters = [
    ['open', 'Open'],
    ['closed', 'Closed'],
    ['all', 'All'],
  ] as const
</script>

{#if forge.loading && !s}
  <div class="grid h-full place-items-center text-muted">Connecting…</div>
{:else if forge.error}
  <div class="grid h-full place-items-center px-6 text-center text-del">{forge.error}</div>
{:else if s && !s.supported}
  <div class="grid h-full place-items-center px-8 text-center">
    <div class="max-w-md">
      <Icon name={kind === 'issues' ? 'issue' : 'pr'} size={32} class="mx-auto text-faint" />
      <p class="mt-3 text-muted">{s.message}</p>
    </div>
  </div>
{:else if s}
  <div class="flex h-full min-h-0">
    <aside class="flex w-[25rem] shrink-0 flex-col border-r border-border bg-surface-2">
      <div class="space-y-2 border-b border-border p-3">
        <div class="flex items-center gap-2">
          <div
            class="flex rounded-md border border-border bg-surface p-0.5 text-[12px] font-medium"
          >
            {#each filters as [id, label] (id)}
              <button
                class="rounded px-2.5 py-1 {list.state === id
                  ? 'bg-accent text-white'
                  : 'text-muted hover:text-fg'}"
                onclick={() => list.setFilter(id)}>{label}</button
              >
            {/each}
          </div>
          <button
            class="icon-btn ml-auto"
            title="Reload"
            disabled={list.loading}
            onclick={() => list.reload()}
          >
            <Icon name="refresh" size={14} class={list.loading ? 'animate-spin' : ''} />
          </button>
        </div>
        <div class="relative">
          <Icon
            name="search"
            size={14}
            class="pointer-events-none absolute top-2.5 left-2.5 text-faint"
          />
          <input
            class="input h-8 pl-8"
            placeholder="Search {noun}…"
            value={list.query}
            oninput={(e) => list.setQuery(e.currentTarget.value)}
            spellcheck="false"
          />
        </div>
        <div class="space-y-0.5 text-[11.5px] text-muted">
          <div class="truncate font-medium text-fg">{s.owner}/{s.repo}</div>
          <div>
            {#if s.connected}
              Signed in as <b class="text-fg">{s.login ?? '…'}</b>
              <span class="text-faint">({s.source === 'gh' ? 'GitHub CLI' : 'saved token'})</span>
              {#if s.source === 'keyring'}
                <button class="ml-1 underline hover:text-fg" onclick={() => forge.disconnect()}
                  >Sign out</button
                >
              {/if}
            {:else}
              <button
                class="font-medium text-accent hover:underline"
                onclick={() => (showConnect = !showConnect)}>Not signed in: Sign in</button
              >
            {/if}
          </div>
        </div>
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto">
        {#if showConnect && !s.connected}
          <ConnectPanel />
        {:else if list.error}
          <div class="p-4 text-[12.5px] text-del">{list.error}</div>
        {:else if list.loading && !list.items.length}
          <div class="p-4 text-[12.5px] text-muted">Loading {noun}…</div>
        {:else if !list.items.length}
          <div class="p-6 text-center text-[12.5px] text-muted">
            No {list.state === 'all' ? '' : list.state + ' '}{noun}{list.query.trim()
              ? ` match “${list.query.trim()}”`
              : ''}.
          </div>
        {/if}

        <ul class="divide-y divide-border">
          {#each list.items as it (it.number)}
            <li>
              <button
                class="flex w-full gap-2.5 px-3 py-2.5 text-left {list.selected === it.number
                  ? 'bg-accent/10 shadow-[inset_2px_0_0_var(--accent)]'
                  : 'hover:bg-hover'}"
                onclick={() => open(it.number)}
              >
                <span class="mt-0.5">
                  {#if 'head_ref' in it}
                    <StateIcon kind="pr" state={it.state} draft={it.draft} />
                  {:else}
                    <StateIcon kind="issue" state={it.state} />
                  {/if}
                </span>
                <span class="min-w-0 flex-1">
                  <span class="line-clamp-2 text-[13px] leading-snug font-medium">{it.title}</span>
                  <span
                    class="mt-0.5 flex flex-wrap items-center gap-x-1.5 text-[11.5px] text-muted"
                  >
                    <span>#{it.number}</span>
                    <span>·</span>
                    <span>{it.author}</span>
                    <span>·</span>
                    <span>{relativeIso(it.updated_at)}</span>
                    {#if 'comments' in it && it.comments}
                      <span class="ml-auto inline-flex items-center gap-1"
                        ><Icon name="message" size={11} />{it.comments}</span
                      >
                    {/if}
                  </span>
                  {#if 'head_ref' in it}
                    <span class="mt-1 flex items-center gap-1 text-[11px] text-muted">
                      <span class="max-w-36 truncate rounded bg-hover px-1 font-mono"
                        >{it.head_ref}</span
                      >→<span class="max-w-24 truncate rounded bg-hover px-1 font-mono"
                        >{it.base_ref}</span
                      >
                      {#if it.draft}<span class="chip">draft</span>{/if}
                    </span>
                  {/if}
                  {#if it.labels.length}
                    <span class="mt-1.5 flex flex-wrap gap-1">
                      {#each it.labels.slice(0, 3) as l (l.name)}<LabelChip label={l} />{/each}
                      {#if it.labels.length > 3}<span class="chip">+{it.labels.length - 3}</span
                        >{/if}
                    </span>
                  {/if}
                </span>
              </button>
            </li>
          {/each}
        </ul>
        {#if list.nextPage !== null && !list.error}
          <div class="p-3 text-center">
            <button class="btn btn-outline h-8" disabled={list.loading} onclick={() => list.more()}>
              {list.loading ? 'Loading…' : 'Load more'}
            </button>
          </div>
        {/if}
      </div>
    </aside>

    <section class="min-w-0 flex-1">
      {#if detail.loading}
        <div class="grid h-full place-items-center text-muted">Loading…</div>
      {:else if detail.error}
        <div class="grid h-full place-items-center px-8 text-center text-del">{detail.error}</div>
      {:else if kind === 'issues' && detail.issue}
        <IssueDetailView />
      {:else if kind === 'pulls' && detail.pull}
        <PullDetailView />
      {:else}
        <div class="grid h-full place-items-center px-8 text-center text-muted">
          <div>
            <Icon name={kind === 'issues' ? 'issue' : 'pr'} size={36} class="mx-auto text-faint" />
            <p class="mt-3">
              Select {kind === 'issues' ? 'an issue' : 'a pull request'} to read it, comment{kind ===
              'pulls'
                ? ', and review it'
                : ' or close it'}.
            </p>
          </div>
        </div>
      {/if}
    </section>
  </div>
{/if}
