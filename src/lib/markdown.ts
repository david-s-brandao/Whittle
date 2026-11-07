/**
 * A small, safe Markdown renderer for issue and pull-request text.
 *
 * Everything that comes in is untrusted. The rule that keeps this safe: ALL input is
 * HTML-escaped first, and only then do we add our own tags. Links are limited to http(s) and
 * mailto, images are never loaded (they become links, so nothing is fetched or tracked).
 */

const escapeHtml = (s: string) =>
  s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')

const SAFE_URL = /^(https?:\/\/|mailto:)[^\s<>"']+$/i

/** Inline formatting on text that has ALREADY been escaped. */
function inline(escaped: string): string {
  const codes: string[] = []
  // inline code first, so nothing inside it is formatted
  let s = escaped.replace(/`([^`\n]+)`/g, (_m, c: string) => {
    codes.push(`<code>${c}</code>`)
    return `\uE000${codes.length - 1}\uE001`
  })
  const link = (text: string, url: string) =>
    SAFE_URL.test(url)
      ? `<a href="${url}" rel="noopener noreferrer" data-ext="1">${text}</a>`
      : text
  // images: shown as a link, never loaded
  s = s.replace(/!\[([^\]\n]*)\]\(([^)\s]+)\)/g, (_m, alt: string, url: string) =>
    link(`🖼 ${alt || 'image'}`, url),
  )
  s = s.replace(/\[([^\]\n]+)\]\(([^)\s]+)\)/g, (_m, text: string, url: string) => link(text, url))
  // bare links (not already inside an href)
  s = s.replace(
    /(^|[\s(])(https?:\/\/[^\s<>"']+[^\s<>"'.,;:!?)])/g,
    (_m, pre: string, url: string) => `${pre}${link(url, url)}`,
  )
  // emphasis never starts or ends with a space: "2 * 3 * 4" stays as written
  s = s
    .replace(/\*\*(?!\s)([^*\n]*[^*\s])\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[^\w])__(?!\s)([^_\n]*[^_\s])__(?=[^\w]|$)/g, '$1<strong>$2</strong>')
    .replace(/(^|[^*\w])\*(?!\s)([^*\n]*[^*\s])\*(?=[^*\w]|$)/g, '$1<em>$2</em>')
    .replace(/(^|[^\w])_(?!\s)([^_\n]*[^_\s])_(?=[^\w]|$)/g, '$1<em>$2</em>')
    .replace(/~~(?!\s)([^~\n]*[^~\s])~~/g, '<del>$1</del>')
    .replace(/(^|[\s(])@([A-Za-z0-9][A-Za-z0-9-]*)/g, '$1<span class="md-mention">@$2</span>')
  return s.replace(/\uE000(\d+)\uE001/g, (_m, i: string) => codes[Number(i)])
}

export function renderMarkdown(source: string): string {
  // HTML comments (issue templates are full of them) are dropped, not shown
  // private-use characters are our inline-code placeholders: input must not contain them
  const text = source
    .replace(/[\uE000\uE001]/g, '')
    .replace(/\r\n?/g, '\n')
    .replace(/<!--[\s\S]*?-->/g, '')
  const out: string[] = []
  const lines = text.split('\n')
  let i = 0
  let paragraph: string[] = []
  const flush = () => {
    if (paragraph.length)
      out.push(`<p>${paragraph.map((l) => inline(escapeHtml(l))).join('<br>')}</p>`)
    paragraph = []
  }

  while (i < lines.length) {
    const line = lines[i]

    // fenced code block
    const fence = /^\s*(```|~~~)/.exec(line)
    if (fence) {
      flush()
      const code: string[] = []
      i++
      while (i < lines.length && !lines[i].trimStart().startsWith(fence[1])) code.push(lines[i++])
      i++ // closing fence
      out.push(`<pre><code>${escapeHtml(code.join('\n'))}</code></pre>`)
      continue
    }
    if (!line.trim()) {
      flush()
      i++
      continue
    }
    const heading = /^(#{1,6})\s+(.*?)\s*#*\s*$/.exec(line)
    if (heading) {
      flush()
      const level = heading[1].length
      out.push(`<h${level}>${inline(escapeHtml(heading[2]))}</h${level}>`)
      i++
      continue
    }
    if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) {
      flush()
      out.push('<hr>')
      i++
      continue
    }
    if (/^\s*>/.test(line)) {
      flush()
      const quote: string[] = []
      while (i < lines.length && /^\s*>/.test(lines[i]))
        quote.push(lines[i++].replace(/^\s*>\s?/, ''))
      out.push(`<blockquote>${renderMarkdown(quote.join('\n'))}</blockquote>`)
      continue
    }
    const listMatch = /^\s*([-*+]|\d+[.)])\s+/.exec(line)
    if (listMatch) {
      flush()
      const ordered = /\d/.test(listMatch[1])
      const items: string[] = []
      while (i < lines.length) {
        const m = /^\s*([-*+]|\d+[.)])\s+(.*)$/.exec(lines[i])
        if (!m || /\d/.test(m[1]) !== ordered) break
        let body = m[2]
        let box = ''
        const task = /^\[( |x|X)\]\s+(.*)$/.exec(body)
        if (task) {
          box = `<input type="checkbox" disabled${task[1] !== ' ' ? ' checked' : ''}> `
          body = task[2]
        }
        items.push(`<li>${box}${inline(escapeHtml(body))}</li>`)
        i++
      }
      out.push(`<${ordered ? 'ol' : 'ul'}>${items.join('')}</${ordered ? 'ol' : 'ul'}>`)
      continue
    }
    paragraph.push(line)
    i++
  }
  flush()
  return out.join('')
}
