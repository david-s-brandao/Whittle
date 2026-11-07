<script lang="ts">
  import { readableText } from '../color'
  import type { ForgeLabel } from '../types'

  let { label }: { label: ForgeLabel } = $props()
  const bg = $derived(label.color ? `#${label.color}` : 'var(--hover)')
  const fg = $derived(label.color ? readableText(label.color) : 'var(--text)')
</script>

<span
  class="inline-flex h-[18px] max-w-40 shrink-0 items-center truncate rounded-full px-2 text-[11px] leading-none font-medium"
  style="background: {bg}; color: {fg}"
  title={label.name}>{label.name}</span
>
