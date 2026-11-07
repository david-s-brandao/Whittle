<script lang="ts">
  import { tick } from 'svelte'
  import { clone } from '../stores/clone.svelte'
  import Icon from './Icon.svelte'

  let urlInput: HTMLInputElement | undefined = $state()
  let details = $state(false)

  $effect(() => {
    if (clone.open) {
      details = false
      tick().then(() => urlInput?.focus())
    }
  })

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') clone.close()
    else if (e.key === 'Enter' && !(e.target instanceof HTMLButtonElement)) clone.start()
  }

  const hints = {
    ssh: 'SSH: uses the keys in your ssh-agent (run `ssh-add -l` to check).',
    https: 'HTTPS: uses your git credential helper for private repositories.',
    other: 'Local path or other URL.',
  } as const
</script>

{#if clone.open}
  <div
    class="backdrop pt-[10vh]"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && clone.close()}
  >
    <div
      class="modal w-[580px]"
      role="dialog"
      aria-modal="true"
      aria-label="Clone repository"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
        <Icon name="download" class="text-accent" />
        <div class="text-[15px] font-semibold">Clone repository</div>
      </div>
      <div class="space-y-4 px-5 py-5">
        <div>
          <label class="label" for="clone-url">Repository URL</label>
          <input
            id="clone-url"
            bind:this={urlInput}
            value={clone.url}
            oninput={(e) => clone.setUrl(e.currentTarget.value)}
            disabled={clone.running}
            placeholder="git@github.com:owner/repo.git or https://github.com/owner/repo.git"
            spellcheck="false"
            class="input"
          />
          {#if clone.kind}
            <p class="mt-1.5 text-[12px] text-muted">{hints[clone.kind]}</p>
          {/if}
        </div>

        <div class="flex gap-3">
          <div class="min-w-0 flex-1">
            <label class="label" for="clone-parent">Clone into</label>
            <div class="flex gap-2">
              <input
                id="clone-parent"
                bind:value={clone.parent}
                disabled={clone.running}
                spellcheck="false"
                class="input min-w-0 flex-1"
              />
              <button
                class="btn btn-outline h-9"
                disabled={clone.running}
                onclick={() => clone.browse()}>Browse…</button
              >
            </div>
          </div>
          <div class="w-40">
            <label class="label" for="clone-name">Folder name</label>
            <input
              id="clone-name"
              value={clone.name}
              oninput={(e) => clone.setName(e.currentTarget.value)}
              disabled={clone.running}
              spellcheck="false"
              class="input"
            />
          </div>
        </div>
        {#if clone.target}
          <p class="truncate text-[12px] text-muted">
            Will create <span class="font-mono text-fg">{clone.target}</span>
          </p>
        {/if}

        {#if clone.running}
          <div>
            <div class="mb-1.5 flex items-center justify-between text-[12px]">
              <span class="truncate font-mono text-muted">{clone.line}</span>
              {#if clone.percent !== null}<span class="ml-3 text-muted">{clone.percent}%</span>{/if}
            </div>
            <div class="h-1.5 overflow-hidden rounded-full bg-hover">
              <div
                class="h-full rounded-full bg-accent transition-all {clone.percent === null
                  ? 'w-1/3 animate-pulse'
                  : ''}"
                style={clone.percent === null ? '' : `width: ${clone.percent}%`}
              ></div>
            </div>
          </div>
        {/if}

        {#if clone.error}
          <div class="rounded-lg border border-del/40 bg-del/10 p-3 text-[12.5px]">
            <div class="flex items-start gap-2">
              <Icon name="alert" class="mt-0.5 shrink-0 text-del" />
              <div class="min-w-0">
                <div>{clone.error.message}</div>
                {#if clone.error.details}
                  <button
                    class="mt-1.5 text-[12px] text-muted underline"
                    onclick={() => (details = !details)}
                  >
                    {details ? 'Hide details' : 'Show details'}
                  </button>
                {/if}
              </div>
            </div>
            {#if details && clone.error.details}
              <pre
                class="mt-2 max-h-40 overflow-auto rounded-md bg-surface p-2 font-mono text-[11px] whitespace-pre-wrap">{clone
                  .error.details}</pre>
            {/if}
          </div>
        {/if}
      </div>
      <div class="flex justify-end gap-2 border-t border-border bg-surface-2 px-5 py-3">
        <button class="btn btn-outline h-8" disabled={clone.running} onclick={() => clone.close()}
          >Cancel</button
        >
        <button
          class="btn btn-primary h-8 px-4 font-semibold"
          disabled={clone.running || !clone.url.trim() || !clone.name}
          onclick={() => clone.start()}>{clone.running ? 'Cloning…' : 'Clone'}</button
        >
      </div>
    </div>
  </div>
{/if}
