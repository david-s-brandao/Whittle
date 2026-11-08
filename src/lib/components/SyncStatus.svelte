<script lang="ts">
  import { opLabel, sync } from '../stores/sync.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  let details = $state(false)
  $effect(() => {
    if (!sync.error) details = false
  })

  // Success messages fade on their own; errors stay until dismissed.
  $effect(() => {
    const t = sync.done
    if (!t) return
    const id = setTimeout(() => sync.dismiss(), 4000)
    return () => clearTimeout(id)
  })
  $effect(() => {
    const n = ui.notice
    if (!n || n.kind !== 'ok') return
    const id = setTimeout(() => (ui.notice = null), 5000)
    return () => clearTimeout(id)
  })

  const opName = { fetch: 'Fetch', pull: 'Pull', push: 'Push' } as const
</script>

<div class="pointer-events-none fixed right-4 bottom-4 z-40 flex flex-col items-end gap-2">
  {#if sync.running && !sync.background}
    <div class="toast">
      <div class="flex items-center gap-2 font-medium">
        <span class="text-accent">{opLabel[sync.running]}…</span>
        {#if sync.percent !== null}<span class="ml-auto text-muted">{sync.percent}%</span>{/if}
      </div>
      <div class="mt-0.5 truncate font-mono text-[11px] text-muted">{sync.line}</div>
      <div class="mt-2 h-1 overflow-hidden rounded-full bg-hover">
        <div
          class="h-full rounded-full bg-accent transition-all {sync.percent === null
            ? 'w-1/3 animate-pulse'
            : ''}"
          style={sync.percent === null ? '' : `width: ${sync.percent}%`}
        ></div>
      </div>
    </div>
  {/if}

  {#if sync.error}
    <div class="toast border-del/40">
      <div class="flex items-start gap-2.5">
        <Icon name="alert" class="mt-0.5 shrink-0 text-del" />
        <div class="min-w-0 flex-1">
          <div class="font-semibold">{opName[sync.error.op]} failed</div>
          <div class="mt-0.5 text-muted">{sync.error.message}</div>
          <div class="mt-2 flex items-center gap-2">
            {#if sync.error.kind === 'rejected' && sync.error.op === 'push'}
              <button class="btn btn-danger btn-outline h-6" onclick={() => sync.run('push', true)}
                >Force push…</button
              >
            {/if}
            {#if sync.error.details}
              <button class="text-[12px] text-muted underline" onclick={() => (details = !details)}>
                {details ? 'Hide details' : 'Show details'}
              </button>
            {/if}
          </div>
        </div>
        <button class="icon-btn icon-btn-sm" title="Dismiss" onclick={() => sync.dismiss()}
          ><Icon name="x" size={14} /></button
        >
      </div>
      {#if details}
        <pre
          class="mt-2 max-h-40 overflow-auto rounded-md bg-surface-2 p-2 font-mono text-[11px] whitespace-pre-wrap">{sync
            .error.details}</pre>
      {/if}
    </div>
  {/if}

  {#if sync.done && !sync.running}
    <div class="toast flex items-center gap-2.5">
      <Icon name="check" class="text-add" />
      <span class="flex-1 font-medium">{sync.done}</span>
      <button class="icon-btn icon-btn-sm" onclick={() => sync.dismiss()}
        ><Icon name="x" size={14} /></button
      >
    </div>
  {/if}

  {#if ui.notice}
    <div class="toast flex items-start gap-2.5 {ui.notice.kind === 'error' ? 'border-del/40' : ''}">
      <Icon
        name={ui.notice.kind === 'ok' ? 'check' : 'alert'}
        class="mt-0.5 shrink-0 {ui.notice.kind === 'ok' ? 'text-add' : 'text-del'}"
      />
      <span class="min-w-0 flex-1 whitespace-pre-wrap">{ui.notice.text}</span>
      <button class="icon-btn icon-btn-sm" onclick={() => (ui.notice = null)}
        ><Icon name="x" size={14} /></button
      >
    </div>
  {/if}
</div>
