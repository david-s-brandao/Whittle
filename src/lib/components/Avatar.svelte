<script lang="ts">
  /** A coloured initial: no images are downloaded, so nothing is tracked. */
  let { name, size = 24 }: { name: string; size?: number } = $props()
  const hue = $derived([...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7))
</script>

<span
  class="inline-grid shrink-0 place-items-center rounded-full font-semibold text-white select-none"
  style="width: {size}px; height: {size}px; font-size: {size *
    0.45}px; background: hsl({hue} 55% 42%)"
  title={name}>{(name[0] ?? '?').toUpperCase()}</span
>
