import { describe, expect, it } from 'vitest'
import { parsePatch } from './patch'

describe('parsePatch', () => {
  const patch = [
    '@@ -10,4 +10,5 @@ fn main() {',
    ' keep',
    '-old line',
    '+new line',
    '+extra',
    ' tail',
    '@@ -40 +41 @@',
    '-x',
    '+y',
    '\\ No newline at end of file',
  ].join('\n')

  it('numbers old and new lines independently', () => {
    const [h1, h2] = parsePatch(patch)
    expect(h1.header).toBe('@@ -10,4 +10,5 @@ fn main() {')
    expect(h1.lines.map((l) => [l.kind, l.old_no, l.new_no])).toEqual([
      [' ', 10, 10],
      ['-', 11, null],
      ['+', null, 11],
      ['+', null, 12],
      [' ', 12, 13],
    ])
    expect(h2.lines.map((l) => [l.kind, l.old_no, l.new_no])).toEqual([
      ['-', 40, null],
      ['+', null, 41],
      ['\\', null, null],
    ])
    expect(h1.lines[1].text).toBe('old line')
  })
  it('handles empty / missing patches and garbage before the first hunk', () => {
    expect(parsePatch(null)).toEqual([])
    expect(parsePatch('')).toEqual([])
    expect(parsePatch('index abc\n--- a\n+++ b')).toEqual([])
  })
  it('keeps blank context lines and CRLF input', () => {
    const [h] = parsePatch('@@ -1,2 +1,2 @@\r\n \r\n-a\r\n+b')
    expect(h.lines).toHaveLength(3)
    expect(h.lines[0]).toMatchObject({ kind: ' ', old_no: 1, new_no: 1, text: '' })
  })
})
