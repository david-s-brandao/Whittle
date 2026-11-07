<script lang="ts">
  import { api } from '../api'
  import { formatIso, relativeIso } from '../format'
  import { detail } from '../stores/forge.svelte'
  import { ui } from '../stores/ui.svelte'
  import CommentCard from './CommentCard.svelte'
  import Composer from './Composer.svelte'
  import Icon from './Icon.svelte'
  import LabelChip from './LabelChip.svelte'
  import StateIcon from './StateIcon.svelte'

  const d = $derived(detail.issue)
</script>

{#if d}
  {@const s = d.summary}
  <div class="flex h-full min-h-0 flex-col">
    <div class="border-b border-border px-6 py-4">
      <div class="flex items-start gap-3">
        <StateIcon kind="issue" state={s.state} size={20} />
        <div class="min-w-0 flex-1">
          <h2 class="selectable text-[17px] leading-snug font-semibold">
            {s.title} <span class="font-normal text-faint">#{s.number}</span>
          </h2>
          <div class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-[12.5px] text-muted">
            <span
              class="rounded-full px-2 py-0.5 text-[11.5px] font-medium text-white {s.state ===
              'open'
                ? 'bg-add'
                : 'bg-[#8250df]'}">{s.state === 'open' ? 'Open' : 'Closed'}</span
            >
            <span
              ><b class="text-fg">{s.author}</b> opened this
              <span title={formatIso(s.created_at)}>{relativeIso(s.created_at)}</span></span
            >
            <span>· {s.comments} comment{s.comments === 1 ? '' : 's'}</span>
            {#if s.assignees.length}<span>· assigned to {s.assignees.join(', ')}</span>{/if}
          </div>
          {#if s.labels.length}
            <div class="mt-2 flex flex-wrap gap-1.5">
              {#each s.labels as l (l.name)}<LabelChip label={l} />{/each}
            </div>
          {/if}
        </div>
        <button
          class="icon-btn"
          title="Open on the website"
          onclick={() =>
            api.openUrl(s.url).catch((e) => (ui.notice = { kind: 'error', text: String(e) }))}
          ><Icon name="external" /></button
        >
      </div>
    </div>

    <div class="min-h-0 flex-1 space-y-4 overflow-y-auto px-6 py-5">
      <CommentCard author={s.author} time={s.created_at} body={d.body} url={s.url} />
      {#each d.comments as c (c.id)}
        <CommentCard
          author={c.author}
          time={c.created_at}
          body={c.body}
          association={c.association}
          url={c.url}
        />
      {/each}

      <div class="max-w-3xl pt-2">
        <Composer busy={detail.busy} onsubmit={(b) => detail.comment(b)}>
          <button
            class="btn btn-outline h-7"
            disabled={detail.busy}
            onclick={() => detail.setState(s.state === 'open' ? 'closed' : 'open')}
          >
            {s.state === 'open' ? 'Close issue' : 'Reopen issue'}
          </button>
        </Composer>
      </div>
    </div>
  </div>
{/if}
