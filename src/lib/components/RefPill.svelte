<script lang="ts">
  import type { RefLabel } from '../types'
  import Icon from './Icon.svelte'

  let { label }: { label: RefLabel } = $props()

  const icon = $derived(
    label.kind === 'remote'
      ? 'cloud'
      : label.kind === 'tag'
        ? 'tag'
        : label.kind === 'head'
          ? 'commit'
          : 'branch',
  )
  const style = $derived(
    label.current
      ? 'bg-accent text-white'
      : label.tracked_by_current
        ? 'border border-accent/60 bg-accent/10 text-accent'
        : label.kind === 'local'
          ? 'border border-border bg-hover text-fg'
          : 'border border-border bg-surface-2 text-muted',
  )
  const title = $derived(
    label.kind === 'head'
      ? 'HEAD (detached): you are on this commit, not on a branch'
      : label.current
        ? `${label.name}: the branch you are on (HEAD)`
        : label.tracked_by_current
          ? `${label.name}: the remote branch your current branch tracks`
          : label.kind === 'remote'
            ? `${label.name} (remote branch)`
            : label.kind === 'tag'
              ? `tag ${label.name}`
              : label.name,
  )
</script>

<span
  class="inline-flex h-[18px] max-w-44 shrink-0 items-center gap-1 rounded-md px-1.5 text-[11px] leading-none font-medium {style}"
  {title}
>
  <Icon name={icon} size={11} class="shrink-0" />
  <span class="truncate">{label.name}</span>
</span>
