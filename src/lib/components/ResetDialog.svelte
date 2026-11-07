<script lang="ts">
  import { commits } from '../stores/commits.svelte'
  import { repo } from '../stores/repo.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  let mode = $state<'soft' | 'mixed' | 'hard'>('mixed')
  const c = $derived(ui.resetDialog)
  const current = $derived(repo.summary?.branch ?? 'the current branch')

  $effect(() => {
    if (c) mode = 'mixed'
  })

  const modes = [
    {
      id: 'soft',
      title: 'Soft',
      hint: 'Only move the branch. Everything since stays staged, ready to re-commit.',
    },
    {
      id: 'mixed',
      title: 'Mixed',
      hint: 'Move the branch and unstage. Your file changes are kept in the working tree.',
    },
    {
      id: 'hard',
      title: 'Hard',
      hint: 'Move the branch and throw away every change since that commit. Cannot be undone from the working tree.',
    },
  ] as const

  async function go() {
    if (!c) return
    const commit = c
    ui.resetDialog = null
    await commits.reset(commit, mode)
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') ui.resetDialog = null
  }
</script>

{#if c}
  <div
    class="backdrop"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (ui.resetDialog = null)}
  >
    <div
      class="modal w-[520px]"
      role="dialog"
      aria-modal="true"
      aria-label="Reset branch"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
        <Icon name="history" class="text-accent" />
        <div class="text-[15px] font-semibold">
          Reset <b>{current}</b> to <span class="font-mono">{c.short_id}</span>
        </div>
      </div>
      <div class="space-y-3 px-5 py-5 text-[13px]">
        <p class="truncate text-muted">{c.summary}</p>
        <div class="space-y-1.5">
          {#each modes as m (m.id)}
            <label
              class="flex cursor-pointer items-start gap-2.5 rounded-lg border p-2.5 {mode === m.id
                ? m.id === 'hard'
                  ? 'border-del bg-del/10'
                  : 'border-accent bg-accent/10'
                : 'border-border hover:bg-hover'}"
            >
              <input
                type="radio"
                name="reset"
                class="mt-0.5 accent-[var(--accent)]"
                checked={mode === m.id}
                onchange={() => (mode = m.id)}
              />
              <span>
                <span class="block font-semibold {m.id === 'hard' ? 'text-del' : ''}"
                  >{m.title}</span
                >
                <span class="block text-[12px] text-muted">{m.hint}</span>
              </span>
            </label>
          {/each}
        </div>
        <p class="text-[12px] text-muted">
          Afterwards the message tells you where the branch was, so you can move back if you change
          your mind.
        </p>
      </div>
      <div class="flex justify-end gap-2 border-t border-border bg-surface-2 px-5 py-3">
        <button class="btn btn-outline h-8" onclick={() => (ui.resetDialog = null)}>Cancel</button>
        <button
          class="btn h-8 px-4 font-semibold {mode === 'hard'
            ? 'bg-del text-white hover:opacity-90'
            : 'btn-primary'}"
          onclick={go}>{mode === 'hard' ? 'Reset and discard changes' : `Reset (${mode})`}</button
        >
      </div>
    </div>
  </div>
{/if}
