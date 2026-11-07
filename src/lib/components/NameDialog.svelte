<script lang="ts">
  import { tick } from 'svelte'
  import { commits } from '../stores/commits.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  let name = $state('')
  let message = $state('')
  let flag = $state(true) // branch: switch to it; tag: push it
  let input: HTMLInputElement | undefined = $state()

  const d = $derived(ui.nameDialog)
  const isTag = $derived(d?.kind === 'tag')

  $effect(() => {
    if (d) {
      name = ''
      message = ''
      flag = d.kind === 'branch'
      tick().then(() => input?.focus())
    }
  })

  async function go() {
    if (!d || !name.trim()) return
    const { kind, commit } = d
    ui.nameDialog = null
    if (kind === 'branch') await commits.createBranchAt(commit, name.trim(), flag)
    else await commits.createTag(commit, name.trim(), message, flag)
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') ui.nameDialog = null
    else if (e.key === 'Enter') go()
  }
</script>

{#if d}
  <div
    class="backdrop"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (ui.nameDialog = null)}
  >
    <div
      class="modal w-[460px]"
      role="dialog"
      aria-modal="true"
      aria-label="Create {d.kind}"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
        <Icon name={isTag ? 'tag' : 'branch'} class="text-accent" />
        <div class="text-[15px] font-semibold">
          Create {d.kind} at <span class="font-mono">{d.commit.short_id}</span>
        </div>
      </div>
      <div class="space-y-4 px-5 py-5">
        <p class="truncate text-[12.5px] text-muted">{d.commit.summary}</p>
        <div>
          <label class="label" for="name-input">{isTag ? 'Tag name' : 'Branch name'}</label>
          <input
            id="name-input"
            bind:this={input}
            bind:value={name}
            class="input"
            spellcheck="false"
            placeholder={isTag ? 'v1.0.0' : 'feature/my-idea'}
          />
        </div>
        {#if isTag}
          <div>
            <label class="label" for="tag-msg">Message (optional: makes it an annotated tag)</label>
            <input
              id="tag-msg"
              class="input"
              bind:value={message}
              placeholder="Release notes headline"
            />
          </div>
        {/if}
        <label class="flex cursor-pointer items-center gap-2 text-[12.5px] select-none">
          <input type="checkbox" class="size-3.5 accent-[var(--accent)]" bind:checked={flag} />
          {isTag ? 'Push the tag to the remote right away' : 'Switch to the new branch'}
        </label>
      </div>
      <div class="flex justify-end gap-2 border-t border-border bg-surface-2 px-5 py-3">
        <button class="btn btn-outline h-8" onclick={() => (ui.nameDialog = null)}>Cancel</button>
        <button class="btn btn-primary h-8 px-4 font-semibold" disabled={!name.trim()} onclick={go}
          >Create {d.kind}</button
        >
      </div>
    </div>
  </div>
{/if}
