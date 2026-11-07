<script lang="ts">
  import Icon from './Icon.svelte'

  /** The little status symbol of an issue or pull request, coloured like on GitHub. */
  let {
    kind,
    state,
    draft = false,
    size = 16,
  }: { kind: 'issue' | 'pr'; state: string; draft?: boolean; size?: number } = $props()

  const spec = $derived.by(() => {
    if (kind === 'issue') {
      return state === 'open'
        ? { icon: 'issue', color: 'text-add', label: 'Open' }
        : { icon: 'issue-closed', color: 'text-[#8250df]', label: 'Closed' }
    }
    if (state === 'merged') return { icon: 'merge', color: 'text-[#8250df]', label: 'Merged' }
    if (state === 'closed') return { icon: 'pr', color: 'text-del', label: 'Closed' }
    if (draft) return { icon: 'pr', color: 'text-faint', label: 'Draft' }
    return { icon: 'pr', color: 'text-add', label: 'Open' }
  })
</script>

<span class="shrink-0 {spec.color}" title={spec.label}>
  <Icon name={spec.icon as 'issue'} {size} />
</span>
