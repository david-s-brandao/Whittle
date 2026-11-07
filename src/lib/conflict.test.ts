import { describe, expect, it } from 'vitest'
import { parseConflicts } from './conflict'

const kinds = (t: string) => parseConflicts(t).lines.map((l) => l.kind)

describe('parseConflicts', () => {
  it('tags ours / theirs / markers / context', () => {
    const t = ['a', '<<<<<<< HEAD', 'mine', '=======', 'yours', '>>>>>>> feature', 'b', ''].join(
      '\n',
    )
    expect(kinds(t)).toEqual(['ctx', 'marker', 'ours', 'marker', 'theirs', 'marker', 'ctx'])
    expect(parseConflicts(t).blocks).toBe(1)
  })

  it('understands diff3 base sections and several blocks', () => {
    const t = [
      '<<<<<<< HEAD',
      'o',
      '||||||| base',
      'b',
      '=======',
      't',
      '>>>>>>> x',
      'mid',
      '<<<<<<< HEAD',
      'o2',
      '=======',
      't2',
      '>>>>>>> x',
    ].join('\n')
    expect(kinds(t)).toEqual([
      'marker',
      'ours',
      'marker',
      'base',
      'marker',
      'theirs',
      'marker',
      'ctx',
      'marker',
      'ours',
      'marker',
      'theirs',
      'marker',
    ])
    expect(parseConflicts(t).blocks).toBe(2)
  })

  it('leaves marker-like text alone outside a conflict and keeps line numbers', () => {
    const r = parseConflicts('=======\n>>>>>>> not a conflict\nplain')
    expect(r.lines.map((l) => l.kind)).toEqual(['ctx', 'ctx', 'ctx'])
    expect(r.lines.map((l) => l.no)).toEqual([1, 2, 3])
    expect(r.blocks).toBe(0)
  })

  it('handles CRLF and an unterminated conflict', () => {
    const r = parseConflicts('<<<<<<< HEAD\r\nmine\r\n=======\r\nyours')
    expect(r.lines.map((l) => l.kind)).toEqual(['marker', 'ours', 'marker', 'theirs'])
    expect(r.lines[1].text).toBe('mine')
  })
})
