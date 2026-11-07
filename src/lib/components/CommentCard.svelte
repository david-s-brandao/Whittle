<script lang="ts">
  import { api } from '../api'
  import { formatIso, relativeIso } from '../format'
  import { ui } from '../stores/ui.svelte'
  import Avatar from './Avatar.svelte'
  import Icon from './Icon.svelte'
  import Markdown from './Markdown.svelte'

  let {
    author,
    time,
    body,
    association = '',
    url = '',
    badge = null,
  }: {
    author: string
    time: string
    body: string
    association?: string
    url?: string
    /** e.g. a review verdict */
    badge?: { text: string; tone: 'good' | 'bad' | 'neutral' } | null
  } = $props()

  const tone = {
    good: 'bg-add/15 text-add',
    bad: 'bg-del/15 text-del',
    neutral: 'bg-hover text-muted',
  } as const
</script>

<div class="flex gap-3">
  <Avatar name={author} />
  <div class="min-w-0 flex-1 overflow-hidden rounded-lg border border-border">
    <div
      class="flex items-center gap-2 border-b border-border bg-surface-2 px-3 py-1.5 text-[12px]"
    >
      <b class="text-fg">{author}</b>
      <span class="text-muted" title={formatIso(time)}>{relativeIso(time)}</span>
      {#if association && association !== 'NONE'}
        <span class="chip normal-case">{association.toLowerCase()}</span>
      {/if}
      {#if badge}
        <span class="rounded-md px-1.5 py-px text-[11px] font-medium {tone[badge.tone]}"
          >{badge.text}</span
        >
      {/if}
      {#if url}
        <button
          class="icon-btn icon-btn-sm ml-auto"
          title="Open this comment on the website"
          onclick={() =>
            api.openUrl(url).catch((e) => (ui.notice = { kind: 'error', text: String(e) }))}
          ><Icon name="external" size={13} /></button
        >
      {/if}
    </div>
    <div class="px-3 py-2.5">
      {#if body.trim()}
        <Markdown source={body} />
      {:else}
        <span class="text-muted italic">No description provided.</span>
      {/if}
    </div>
  </div>
</div>
