import { describe, expect, it } from 'vitest'
import { renderMarkdown as md } from './markdown'

describe('markdown: formatting', () => {
  it('renders the basics', () => {
    expect(md('# Title')).toBe('<h1>Title</h1>')
    expect(md('some **bold** and *it* and ~~gone~~')).toBe(
      '<p>some <strong>bold</strong> and <em>it</em> and <del>gone</del></p>',
    )
    expect(md('use `a < b` here')).toContain('<code>a &lt; b</code>')
    expect(md('line one\nline two')).toBe('<p>line one<br>line two</p>')
    expect(md('---')).toBe('<hr>')
  })
  it('renders lists, task lists and quotes', () => {
    expect(md('- a\n- b')).toBe('<ul><li>a</li><li>b</li></ul>')
    expect(md('1. first\n2. second')).toBe('<ol><li>first</li><li>second</li></ol>')
    expect(md('- [x] done\n- [ ] todo')).toBe(
      '<ul><li><input type="checkbox" disabled checked> done</li><li><input type="checkbox" disabled> todo</li></ul>',
    )
    expect(md('> quoted **text**')).toBe(
      '<blockquote><p>quoted <strong>text</strong></p></blockquote>',
    )
  })
  it('keeps code blocks verbatim and escaped, without formatting inside', () => {
    const html = md('```js\nconst a = "<b>**x**</b>"\n```')
    expect(html).toBe('<pre><code>const a = &quot;&lt;b&gt;**x**&lt;/b&gt;&quot;</code></pre>')
  })
  it('links http(s), auto-links bare urls, shows images as links, mentions', () => {
    expect(md('[docs](https://example.com/a?b=1)')).toContain(
      '<a href="https://example.com/a?b=1" rel="noopener noreferrer" data-ext="1">docs</a>',
    )
    expect(md('see https://example.com/x.')).toContain('<a href="https://example.com/x"')
    expect(md('![diagram](https://example.com/d.png)')).toContain('🖼 diagram</a>')
    expect(md('![diagram](https://example.com/d.png)')).not.toContain('<img')
    expect(md('thanks @ana-b!')).toContain('<span class="md-mention">@ana-b</span>')
  })
  it('drops html comments from issue templates', () => {
    expect(md('before <!-- hidden\nstuff --> after')).toBe('<p>before  after</p>')
  })
  it('does not turn snake_case words or math into emphasis', () => {
    expect(md('use snake_case_name here')).toBe('<p>use snake_case_name here</p>')
    expect(md('2 * 3 * 4')).toBe('<p>2 * 3 * 4</p>')
  })
})

describe('markdown: untrusted input can never inject markup', () => {
  const noInjection = (html: string) => {
    // after rendering there must be no raw tag other than the ones we emit ourselves
    const tags = [...html.matchAll(/<\/?([a-z0-9]+)/gi)].map((m) => m[1].toLowerCase())
    const allowed = new Set([
      'p',
      'br',
      'a',
      'strong',
      'em',
      'del',
      'code',
      'pre',
      'ul',
      'ol',
      'li',
      'h1',
      'h2',
      'h3',
      'h4',
      'h5',
      'h6',
      'hr',
      'blockquote',
      'input',
      'span',
    ])
    return tags.every((t) => allowed.has(t))
  }

  it('escapes raw html and scripts', () => {
    for (const evil of [
      '<script>alert(1)</script>',
      '<img src=x onerror=alert(1)>',
      '<iframe src="//evil"></iframe>',
      '<svg onload=alert(1)>',
      '<a href="javascript:alert(1)">x</a>',
    ]) {
      const html = md(evil)
      expect(noInjection(html), html).toBe(true)
      expect(html).not.toMatch(/<(script|img|iframe|svg)/i)
    }
  })
  it('refuses javascript:, data: and other schemes in links', () => {
    for (const url of [
      'javascript:alert(1)',
      'JaVaScRiPt:alert(1)',
      'data:text/html,<script>alert(1)</script>',
      'vbscript:x',
      'file:///etc/passwd',
      '//evil.com',
    ]) {
      const html = md(`[click](${url})`)
      expect(html, html).not.toContain('<a ')
      expect(html).not.toContain('href=')
    }
  })
  it('cannot break out of the href attribute', () => {
    const html = md('[x](https://example.com/"onmouseover="alert(1))')
    // the whole tag must be exactly the attributes we emit, with no raw quote inside the value
    const tag = /<a ([^>]*)>/.exec(html)
    expect(tag, html).not.toBeNull()
    expect(tag![1]).toMatch(/^href="[^"]*" rel="noopener noreferrer" data-ext="1"$/)
    expect(html.match(/<a /g)).toHaveLength(1)
    const bare = md('https://example.com/"><script>alert(1)</script>')
    expect(bare).not.toContain('<script')
    expect(/<a ([^>]*)>/.exec(bare)![1]).toMatch(
      /^href="[^"]*" rel="noopener noreferrer" data-ext="1"$/,
    )
  })
  it('does not let code spans or fences smuggle markup', () => {
    for (const evil of [
      '`<img src=x onerror=alert(1)>`',
      '```\n</code></pre><script>alert(1)</script>\n```',
    ]) {
      const html = md(evil)
      expect(html).not.toMatch(/<script/i)
      expect(html).not.toMatch(/<img/i)
    }
  })
  it('cannot forge the inline-code placeholders', () => {
    const html = md('`<b>x</b>` and \uE0000\uE001 and \uE0009\uE001')
    expect(html).not.toContain('undefined')
    expect(html.match(/<code>/g)).toHaveLength(1)
  })
  it('never emits any dangerous token for a pile of hostile strings', () => {
    const hostile = [
      '"><img src=x onerror=1>',
      "' onfocus='alert(1)",
      '<<script>script>alert(1)<</script>/script>',
      '[a](<javascript:alert(1)>)',
      '**<b onclick=1>x</b>**',
      '> <script>1</script>',
      '- <img src=x onerror=1>',
      '# <script>1</script>',
    ]
    for (const h of hostile) {
      const html = md(h)
      expect(noInjection(html), `${h} -> ${html}`).toBe(true)
      for (const d of ['<script', '<img', '<iframe', '<svg'])
        expect(html.toLowerCase()).not.toContain(d)
    }
  })
})
