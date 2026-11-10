<script lang="ts">
  import { api } from '../api'
  import { formatDate } from '../format'
  import { projects } from '../stores/projects.svelte'
  import { repo } from '../stores/repo.svelte'
  import type { CommitMessage } from '../types'
  import Icon from './Icon.svelte'

  let info = $state<CommitMessage | null>(null)
  let error = $state<string | null>(null)

  $effect(() => {
    const c = repo.selected
    const path = projects.current?.path
    info = null
    error = null
    if (!c || !path) return
    api
      .commitMessage(path, c.id)
      .then((m) => {
        if (repo.selected?.id === c.id) info = m
      })
      .catch((e) => {
        if (repo.selected?.id === c.id) error = String(e)
      })
  })

  const lines = $derived((info?.message ?? '').split('\n'))
  const title = $derived(lines[0] ?? '')
  const body = $derived(lines.slice(1).join('\n').replace(/^\n+/, ''))
  const same = $derived(
    !!info &&
      info.committer === repo.selected?.author &&
      info.committer_time === repo.selected?.time,
  )
</script>

<div class="h-full overflow-y-auto bg-surface">
  {#if !repo.selected}
    <div class="grid h-full place-items-center text-muted">
      Select a commit to read its description.
    </div>
  {:else if error}
    <div class="p-4 text-del">{error}</div>
  {:else}
    <div class="mx-auto max-w-3xl space-y-3 px-5 py-4">
      <h2 class="selectable text-[15px] leading-snug font-semibold">
        {title || repo.selected.summary}
      </h2>
      {#if body}
        <pre
          class="selectable font-sans text-[13px] leading-relaxed whitespace-pre-wrap text-fg">{body}</pre>
      {:else if info}
        <p class="text-[12.5px] text-faint">This commit has no description beyond its title.</p>
      {/if}

      <dl
        class="grid grid-cols-[6rem_1fr] gap-x-3 gap-y-1.5 border-t border-border pt-3 text-[12.5px]"
      >
        <dt class="text-muted">Author</dt>
        <dd class="selectable">
          {repo.selected.author} <span class="text-faint">&lt;{repo.selected.email}&gt;</span>
          · {formatDate(repo.selected.time)}
        </dd>
        {#if info && !same}
          <dt class="text-muted">Committer</dt>
          <dd class="selectable">
            {info.committer} <span class="text-faint">&lt;{info.committer_email}&gt;</span>
            · {formatDate(info.committer_time)}
          </dd>
        {/if}
        <dt class="text-muted">Commit</dt>
        <dd class="selectable font-mono text-[12px]">{repo.selected.id}</dd>
        {#if repo.selected.parents.length}
          <dt class="text-muted">{repo.selected.parents.length > 1 ? 'Parents' : 'Parent'}</dt>
          <dd class="flex flex-wrap gap-1.5 font-mono text-[12px]">
            {#each repo.selected.parents as p (p)}<span class="selectable rounded bg-hover px-1"
                >{p.slice(0, 7)}</span
              >{/each}
          </dd>
        {/if}
        {#if info?.refs.length}
          <dt class="text-muted">Refs</dt>
          <dd class="flex flex-wrap gap-1.5">
            {#each info.refs as r (r)}<span class="chip"><Icon name="branch" size={11} />{r}</span
              >{/each}
          </dd>
        {/if}
        <dt class="text-muted">Changes</dt>
        <dd>{repo.files.length} file{repo.files.length === 1 ? '' : 's'}</dd>
      </dl>
    </div>
  {/if}
</div>
