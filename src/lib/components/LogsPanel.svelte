<script lang="ts">
  import { tick } from 'svelte'
  import { changes } from '../stores/changes.svelte'
  import { dock } from '../stores/dock.svelte'
  import { logs } from '../stores/logs.svelte'
  import { projects } from '../stores/projects.svelte'
  import Icon from './Icon.svelte'

  const run = $derived(logs.current)
  let body: HTMLDivElement | undefined = $state()
  let follow = $state(true)
  let now = $state(Date.now())

  const seconds = $derived(run ? Math.max(0, ((run.endedAt ?? now) - run.startedAt) / 1000) : 0)
  const label = $derived(
    run?.status === 'running'
      ? 'Running…'
      : run?.status === 'passed'
        ? 'Passed'
        : run?.status === 'failed'
          ? 'Failed'
          : 'No hook',
  )
  const tone = $derived(
    run?.status === 'failed' ? 'text-del' : run?.status === 'passed' ? 'text-add' : 'text-muted',
  )

  $effect(() => {
    if (run?.status !== 'running') return
    const t = setInterval(() => (now = Date.now()), 250)
    return () => clearInterval(t)
  })

  // keep the newest line in view while following
  $effect(() => {
    void run?.lines.length
    if (follow) void tick().then(() => body && (body.scrollTop = body.scrollHeight))
  })

  function onscroll() {
    if (!body) return
    follow = body.scrollHeight - body.scrollTop - body.clientHeight < 24
  }

  // strip terminal colour codes: the log is plain text
  // eslint-disable-next-line no-control-regex
  const plain = (line: string) => line.replace(/\u001b\[[0-9;?]*[A-Za-z]/g, '')

  function close() {
    logs.clear(projects.current?.path)
    dock.select('terminal')
  }
</script>

<div class="flex h-full min-h-0 flex-col bg-surface">
  <div class="flex h-8 shrink-0 items-center gap-2 border-b border-border px-3 text-[12px]">
    {#if run?.status === 'running'}
      <span class="size-2 animate-pulse rounded-full bg-accent"></span>
    {:else if run?.status === 'passed'}
      <Icon name="check" size={13} class="text-add" />
    {:else if run?.status === 'failed'}
      <Icon name="x" size={13} class="text-del" />
    {/if}
    <span class="font-medium">{run?.title ?? 'pre-commit hook'}</span>
    <span class={tone}>{label}</span>
    {#if run}<span class="text-faint">{seconds.toFixed(1)}s · {run.lines.length} lines</span>{/if}
    <div class="ml-auto flex items-center gap-1">
      <button
        class="btn h-6 px-2 text-[11.5px]"
        disabled={changes.busy}
        onclick={() => changes.runChecks()}><Icon name="refresh" size={12} /> Run again</button
      >
      <button
        class="btn h-6 px-2 text-[11.5px]"
        disabled={run?.status === 'running'}
        onclick={close}>Clear</button
      >
    </div>
  </div>
  <div
    bind:this={body}
    {onscroll}
    class="min-h-0 flex-1 overflow-auto px-3 py-2 font-mono text-[12px] leading-relaxed"
    role="log"
    aria-live="polite"
  >
    {#if run}
      {#each run.lines as line, i (i)}
        <div
          class="break-words whitespace-pre-wrap {run.status === 'failed' &&
          i === run.lines.length - 1
            ? 'text-del'
            : ''}"
        >
          {plain(line)}
        </div>
      {:else}
        <div class="text-faint">
          {run.status === 'running' ? 'Waiting for output…' : 'No output.'}
        </div>
      {/each}
    {/if}
  </div>
</div>
