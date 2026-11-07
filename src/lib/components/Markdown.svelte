<script lang="ts">
  import { api } from '../api'
  import { renderMarkdown } from '../markdown'
  import { ui } from '../stores/ui.svelte'

  let { source, class: cls = '' }: { source: string; class?: string } = $props()

  // The renderer escapes all input before adding its own tags (see markdown.ts).
  const html = $derived(renderMarkdown(source))

  /** Links leave the app: they open in the default browser, never inside the window. */
  function onclick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest('a[data-ext]') as HTMLAnchorElement | null
    if (!a) return
    e.preventDefault()
    api.openUrl(a.href).catch((err) => (ui.notice = { kind: 'error', text: String(err) }))
  }
</script>

<div class="md selectable {cls}" {onclick} role="presentation">
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html html}
</div>
