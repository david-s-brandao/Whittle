<script lang="ts">
  import { parseConflicts } from '../conflict'

  let { text, ours, theirs }: { text: string; ours: string; theirs: string } = $props()

  const parsed = $derived(parseConflicts(text))
  const bg = {
    ctx: '',
    ours: 'bg-info/10',
    theirs: 'bg-[var(--add-bg)]',
    base: 'bg-hover',
    marker: 'bg-surface-2 text-faint',
  } as const
  const tag = $derived({ ours, theirs, base: 'common ancestor' } as const)
</script>

<div class="min-w-max py-1">
  <div class="sticky left-0 flex flex-wrap items-center gap-3 px-4 py-2 text-[12px]">
    <span class="font-semibold">
      {parsed.blocks} conflict{parsed.blocks === 1 ? '' : 's'} in this file
    </span>
    <span class="flex items-center gap-1.5"
      ><span class="size-2.5 rounded-sm bg-info/40"></span>{ours}</span
    >
    <span class="flex items-center gap-1.5"
      ><span class="size-2.5 rounded-sm bg-add/40"></span>{theirs}</span
    >
  </div>
  <div class="selectable font-mono text-[12px] leading-[20px]">
    {#each parsed.lines as l (l.no)}
      <div class="flex {bg[l.kind]}">
        <span class="w-11 shrink-0 pr-2 text-right text-[11px] text-faint select-none">{l.no}</span>
        <span
          class="w-28 shrink-0 truncate pr-2 text-[10.5px] whitespace-nowrap text-faint select-none"
          title={l.kind in tag ? tag[l.kind as keyof typeof tag] : ''}
        >
          {l.kind in tag ? tag[l.kind as keyof typeof tag] : ''}
        </span>
        <span class="pr-6 whitespace-pre {l.kind === 'marker' ? 'font-semibold' : ''}"
          >{l.text}</span
        >
      </div>
    {/each}
  </div>
</div>
