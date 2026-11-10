<script lang="ts">
  import { onMount } from 'svelte'
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import '@xterm/xterm/css/xterm.css'
  import { api } from '../api'
  import { theme } from '../stores/theme.svelte'
  import { terminals } from '../stores/terminal.svelte'

  let {
    tabKey,
    path,
    visible,
    onexit,
  }: { tabKey: number; path: string; visible: boolean; onexit: () => void } = $props()

  let host: HTMLDivElement | undefined = $state()
  let term: Terminal | undefined
  let fit: FitAddon | undefined
  let id: number | null = null
  let exited = $state(false)
  let error = $state<string | null>(null)

  function css(name: string): string {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  }

  function palette() {
    return {
      background: css('--surface'),
      foreground: css('--text'),
      cursor: css('--accent'),
      cursorAccent: css('--surface'),
      selectionBackground: 'rgb(246 96 8 / 0.3)',
    }
  }

  function refit() {
    if (!term || !fit || !host || host.clientWidth === 0 || host.clientHeight === 0) return
    fit.fit()
    if (id !== null) void api.ptyResize(id, term.cols, term.rows).catch(() => {})
  }

  onMount(() => {
    terminals.listen()
    const t = new Terminal({
      fontFamily: css('--font-mono'),
      fontSize: 12.5,
      cursorBlink: true,
      scrollback: 5000,
      allowProposedApi: true,
      theme: palette(),
    })
    const f = new FitAddon()
    t.loadAddon(f)
    t.open(host!)
    term = t
    fit = f
    f.fit()

    t.onData((d) => {
      if (id !== null) void api.ptyWrite(id, d).catch(() => {})
    })
    // Ctrl+Shift+C / V copy and paste like other Linux terminals; plain Ctrl+C stays an interrupt.
    t.attachCustomKeyEventHandler((e) => {
      if (e.type === 'keydown' && e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'c') {
        const sel = t.getSelection()
        if (sel) void navigator.clipboard.writeText(sel).catch(() => {})
        return false
      }
      // keep the app-wide shortcuts working from inside the terminal
      if (
        e.ctrlKey &&
        !e.shiftKey &&
        ['k', 'p', 'b', 't', 'w', 'j', '`'].includes(e.key.toLowerCase())
      ) {
        return false
      }
      return true
    })

    let disposed = false
    api
      .ptyOpen(path, t.cols, t.rows)
      .then((newId) => {
        if (disposed) {
          void api.ptyClose(newId)
          return
        }
        id = newId
        terminals.attach(newId, {
          write: (d) => t.write(d),
          exit: () => {
            exited = true
            onexit()
          },
        })
        refit()
      })
      .catch((e) => (error = String(e)))

    const ro = new ResizeObserver(() => refit())
    ro.observe(host!)

    return () => {
      disposed = true
      ro.disconnect()
      if (id !== null) {
        terminals.detach(id)
        void api.ptyClose(id).catch(() => {})
      }
      t.dispose()
    }
  })

  // colours follow the light / dark theme
  $effect(() => {
    void theme.rev
    if (term) requestAnimationFrame(() => term && (term.options.theme = palette()))
  })

  // focus the shell when its tab is shown
  $effect(() => {
    if (visible && term) {
      queueMicrotask(() => {
        refit()
        term?.focus()
      })
    }
  })
</script>

<div class="relative h-full w-full bg-surface" class:hidden={!visible} data-tab={tabKey}>
  <div bind:this={host} class="h-full w-full px-3 pt-2"></div>
  {#if error}
    <div class="absolute inset-x-0 top-0 p-4 text-[12.5px] text-del">{error}</div>
  {:else if exited}
    <div class="absolute right-3 bottom-2 rounded bg-hover px-2 py-1 text-[11.5px] text-muted">
      Shell exited. Close this tab or open a new one.
    </div>
  {/if}
</div>

<style>
  :global(.xterm) {
    height: 100%;
  }
  :global(.xterm-viewport) {
    background-color: transparent !important;
  }
</style>
