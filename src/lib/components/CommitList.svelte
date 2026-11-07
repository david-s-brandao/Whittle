<script lang="ts">
  import { formatDate, relativeTime } from '../format'
  import { laneColor, lanePalette } from '../graphColors'
  import { repo } from '../stores/repo.svelte'
  import { theme } from '../stores/theme.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'
  import RefPill from './RefPill.svelte'

  const ROW = 34
  const OVERSCAN = 8
  const LANE_W = 14
  const MAX_LANES = 12 // wider graphs are squeezed into the last column rather than eating the list
  const PAD = 10

  let scrollTop = $state(0)
  let height = $state(400)
  let canvas: HTMLCanvasElement | undefined = $state()
  let scroller: HTMLDivElement | undefined = $state()
  const MAX_PILLS = 3

  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN))
  const last = $derived(
    Math.min(repo.commits.length, Math.ceil((scrollTop + height) / ROW) + OVERSCAN),
  )
  const visible = $derived(repo.commits.slice(first, last))
  const graphW = $derived(Math.min(repo.graphLanes, MAX_LANES) * LANE_W + PAD * 2)

  function onscroll(e: Event) {
    const el = e.currentTarget as HTMLElement
    scrollTop = el.scrollTop
    // prefetch the next page when within ~2 screens of the end
    if (el.scrollHeight - el.scrollTop - el.clientHeight < height * 2) repo.loadMore()
  }

  // If the first page doesn't fill the viewport, keep loading.
  $effect(() => {
    if (repo.commits.length * ROW < height * 2) repo.loadMore()
  })

  // "Go to HEAD": scroll the row of the requested commit into view, roughly centred
  $effect(() => {
    const r = repo.reveal
    if (!r || !scroller) return
    const i = repo.commits.findIndex((c) => c.id === r.id)
    if (i >= 0) scroller.scrollTop = Math.max(0, i * ROW - height / 2 + ROW / 2)
  })

  const x = (lane: number) => PAD + Math.min(lane, MAX_LANES - 1) * LANE_W + LANE_W / 2

  /** Repaints the visible slice of the graph. Cheap: only rows on screen are touched. */
  function draw() {
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const dpr = window.devicePixelRatio || 1
    const w = graphW
    const h = height
    if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
      canvas.width = Math.round(w * dpr)
      canvas.height = Math.round(h * dpr)
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    ctx.clearRect(0, 0, w, h)

    const palette = lanePalette(theme.dark)
    const bg = getComputedStyle(document.documentElement).getPropertyValue('--surface').trim()
    const col = (i: number) => laneColor(palette, i)
    ctx.lineWidth = 2
    ctx.lineCap = 'round'

    const from = Math.max(0, Math.floor(scrollTop / ROW) - 1)
    const to = Math.min(repo.graph.length, Math.ceil((scrollTop + h) / ROW) + 1)

    // lines first, so nodes always sit on top
    for (let i = from; i < to; i++) {
      const row = repo.graph[i]
      const top = i * ROW - scrollTop
      const mid = top + ROW / 2
      const bottom = top + ROW
      for (const e of row.edges) {
        ctx.strokeStyle = col(e.color)
        ctx.beginPath()
        if (e.kind === 'pass') {
          ctx.moveTo(x(e.from), top)
          ctx.lineTo(x(e.from), bottom)
        } else if (e.kind === 'in') {
          ctx.moveTo(x(e.from), top)
          if (e.from === e.to) ctx.lineTo(x(e.to), mid)
          else
            ctx.bezierCurveTo(x(e.from), top + ROW * 0.45, x(e.to), top + ROW * 0.05, x(e.to), mid)
        } else {
          ctx.moveTo(x(e.from), mid)
          if (e.from === e.to) ctx.lineTo(x(e.to), bottom)
          else
            ctx.bezierCurveTo(
              x(e.from),
              mid + ROW * 0.45,
              x(e.to),
              bottom - ROW * 0.45,
              x(e.to),
              bottom,
            )
        }
        ctx.stroke()
      }
    }
    const accent = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()
    const headId = repo.refs.head_id
    for (let i = from; i < to; i++) {
      const row = repo.graph[i]
      const cx = x(row.lane)
      const cy = i * ROW - scrollTop + ROW / 2
      if (headId && repo.commits[i]?.id === headId) {
        // "you are here": an accent halo and ring around the commit HEAD points at
        ctx.beginPath()
        ctx.arc(cx, cy, 10, 0, Math.PI * 2)
        ctx.fillStyle = accent + '2e'
        ctx.fill()
        ctx.beginPath()
        ctx.arc(cx, cy, 8, 0, Math.PI * 2)
        ctx.strokeStyle = accent
        ctx.lineWidth = 2
        ctx.stroke()
        ctx.lineWidth = 2
      }
      ctx.beginPath()
      ctx.arc(cx, cy, row.merge ? 3.5 : 4.5, 0, Math.PI * 2)
      if (row.merge) {
        // merge commits: hollow ring
        ctx.fillStyle = bg
        ctx.fill()
        ctx.strokeStyle = col(row.color)
        ctx.lineWidth = 2
        ctx.stroke()
      } else {
        ctx.fillStyle = col(row.color)
        ctx.fill()
        ctx.strokeStyle = bg
        ctx.lineWidth = 1.5
        ctx.stroke()
      }
      ctx.lineWidth = 2
    }
  }

  $effect(() => {
    // re-run whenever anything the picture depends on changes
    void [scrollTop, height, graphW, repo.graph, theme.dark, canvas, repo.refs.head_id]
    draw()
  })
</script>

<div class="flex h-full min-h-0 flex-col">
  <div class="section-label flex h-8 shrink-0 items-center gap-4 border-b border-border px-4">
    <span class="shrink-0" style="width: {graphW - 16}px">Graph</span>
    <span class="flex-1">Message</span>
    <span class="w-36">Author</span>
    <span class="w-24 text-right">Date</span>
    <span class="w-16">Hash</span>
  </div>
  <div class="relative min-h-0 flex-1">
    <canvas
      bind:this={canvas}
      class="pointer-events-none absolute top-0 left-0 z-10"
      style="width: {graphW}px; height: {height}px"
    ></canvas>
    <div
      class="absolute inset-0 overflow-y-auto"
      bind:this={scroller}
      bind:clientHeight={height}
      {onscroll}
      role="listbox"
      tabindex="0"
      aria-label="Commits"
    >
      <div style="height: {repo.commits.length * ROW}px">
        {#each visible as c, i (c.id)}
          {@const sel = repo.selected?.id === c.id}
          {@const labels = repo.labelsById[c.id] ?? []}
          <div
            role="option"
            aria-selected={sel}
            tabindex="-1"
            class="absolute right-0 left-0 flex cursor-default items-center gap-4 pr-4 text-[13px] {sel
              ? 'bg-accent/10 shadow-[inset_2px_0_0_var(--accent)]'
              : 'hover:bg-hover'}"
            style="top: {(first + i) * ROW}px; height: {ROW}px; padding-left: {graphW}px"
            onclick={() => repo.selectCommit(c)}
            oncontextmenu={(e) => {
              e.preventDefault()
              ui.commitMenu = { commit: c, x: e.clientX, y: e.clientY }
            }}
            onkeydown={null}
          >
            {#if labels.length}
              <span class="flex shrink-0 items-center gap-1">
                {#each labels.slice(0, MAX_PILLS) as l (l.kind + l.name)}<RefPill
                    label={l}
                  />{/each}
                {#if labels.length > MAX_PILLS}
                  <span
                    class="chip"
                    title={labels
                      .slice(MAX_PILLS)
                      .map((l) => l.name)
                      .join(', ')}>+{labels.length - MAX_PILLS}</span
                  >
                {/if}
              </span>
            {/if}
            <span class="flex-1 truncate" title={c.summary}>{c.summary}</span>
            {#if c.signed}
              <Icon name="shield-check" size={13} class="-mx-2 shrink-0 text-add" />
            {/if}
            <span class="w-36 truncate text-muted" title={c.email}>{c.author}</span>
            <span class="w-24 text-right text-[12px] text-muted" title={formatDate(c.time)}
              >{relativeTime(c.time)}</span
            >
            <span class="w-16 font-mono text-[11.5px] text-faint">{c.short_id}</span>
          </div>
        {/each}
      </div>
      {#if !repo.commits.length && !repo.loadingCommits}
        <div class="absolute inset-0 grid place-items-center text-muted">No commits yet.</div>
      {/if}
    </div>
  </div>
</div>
