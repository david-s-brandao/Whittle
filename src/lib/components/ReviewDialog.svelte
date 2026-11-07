<script lang="ts">
  import { detail } from '../stores/forge.svelte'
  import type { ReviewEvent } from '../types'
  import Icon from './Icon.svelte'

  let { open = $bindable(false) }: { open: boolean } = $props()

  let event = $state<ReviewEvent>('COMMENT')
  let body = $state('')

  $effect(() => {
    if (open) {
      event = 'COMMENT'
      body = ''
    }
  })

  const options: { id: ReviewEvent; title: string; hint: string }[] = [
    {
      id: 'COMMENT',
      title: 'Comment',
      hint: 'Leave general feedback without approving or blocking.',
    },
    { id: 'APPROVE', title: 'Approve', hint: 'Looks good: this can be merged.' },
    {
      id: 'REQUEST_CHANGES',
      title: 'Request changes',
      hint: 'This needs to be fixed before it can be merged.',
    },
  ]
  const needsText = $derived(event !== 'APPROVE')
  const label = $derived(
    event === 'APPROVE'
      ? 'Approve'
      : event === 'REQUEST_CHANGES'
        ? 'Request changes'
        : 'Submit review',
  )

  async function submit() {
    if ((needsText && !body.trim()) || detail.busy) return
    if (await detail.review(event, body)) open = false
  }
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (open = false)}
  >
    <div
      class="modal w-[520px]"
      role="dialog"
      aria-modal="true"
      aria-label="Review changes"
      tabindex="-1"
      onkeydown={(e) => e.key === 'Escape' && (open = false)}
    >
      <div class="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
        <Icon name="pr" class="text-accent" />
        <div class="text-[15px] font-semibold">Review changes</div>
        <div class="text-[12px] text-muted">#{detail.pull?.summary.number}</div>
      </div>
      <div class="space-y-4 px-5 py-5">
        <div class="space-y-1.5">
          {#each options as o (o.id)}
            <label
              class="flex cursor-pointer items-start gap-2.5 rounded-lg border p-2.5 {event === o.id
                ? o.id === 'REQUEST_CHANGES'
                  ? 'border-del bg-del/10'
                  : 'border-accent bg-accent/10'
                : 'border-border hover:bg-hover'}"
            >
              <input
                type="radio"
                name="review"
                class="mt-0.5 accent-[var(--accent)]"
                checked={event === o.id}
                onchange={() => (event = o.id)}
              />
              <span>
                <span class="block text-[13px] font-semibold">{o.title}</span>
                <span class="block text-[12px] text-muted">{o.hint}</span>
              </span>
            </label>
          {/each}
        </div>
        <div>
          <label class="label" for="review-body"
            >{needsText ? 'Your review (required)' : 'Comment (optional)'}</label
          >
          <textarea
            id="review-body"
            rows="5"
            bind:value={body}
            class="input h-auto resize-y py-2"
            placeholder="Leave a comment…"></textarea>
        </div>
      </div>
      <div class="flex justify-end gap-2 border-t border-border bg-surface-2 px-5 py-3">
        <button class="btn btn-outline h-8" onclick={() => (open = false)}>Cancel</button>
        <button
          class="btn h-8 px-4 font-semibold {event === 'REQUEST_CHANGES'
            ? 'bg-del text-white hover:opacity-90'
            : 'btn-primary'}"
          disabled={(needsText && !body.trim()) || detail.busy}
          onclick={submit}>{detail.busy ? 'Sending…' : label}</button
        >
      </div>
    </div>
  </div>
{/if}
