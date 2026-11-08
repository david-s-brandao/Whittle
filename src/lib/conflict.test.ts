import { describe, expect, it } from 'vitest'
import {
  conflictCount,
  parseConflictDoc,
  parseConflicts,
  renderResult,
  resolveAll,
  unresolvedCount,
  type ConflictSegment,
} from './conflict'

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

describe('conflict editor model', () => {
  const sample = [
    'top',
    '<<<<<<< HEAD',
    'mine',
    '=======',
    'yours',
    '>>>>>>> feature',
    'mid',
    '<<<<<<< HEAD',
    'a',
    '||||||| base',
    'b',
    '=======',
    'c',
    '>>>>>>> feature',
    'end',
    '',
  ].join('\n')

  it('parses segments with labels and base', () => {
    const doc = parseConflictDoc(sample)!
    expect(doc.segments.map((s) => s.type)).toEqual([
      'text',
      'conflict',
      'text',
      'conflict',
      'text',
    ])
    expect(conflictCount(doc)).toBe(2)
    const c = doc.segments[3] as ConflictSegment
    expect(c.base).toEqual(['b'])
    expect(c.oursLabel).toBe('HEAD')
    expect(c.theirsLabel).toBe('feature')
  })

  it('round-trips unresolved text exactly (without a base section it is dropped from nothing)', () => {
    const noBase = sample.replace('||||||| base\nb\n', '')
    expect(renderResult(parseConflictDoc(noBase)!)).toBe(noBase)
  })

  it('renders each choice', () => {
    const doc = parseConflictDoc(sample)!
    resolveAll(doc, 'ours')
    expect(renderResult(doc)).toBe('top\nmine\nmid\na\nend\n')
    resolveAll(doc, 'theirs', true)
    expect(renderResult(doc)).toBe('top\nyours\nmid\nc\nend\n')
    resolveAll(doc, 'both', true)
    expect(renderResult(doc)).toBe('top\nmine\nyours\nmid\na\nc\nend\n')
    resolveAll(doc, 'both-rev', true)
    expect(renderResult(doc)).toBe('top\nyours\nmine\nmid\nc\na\nend\n')
    resolveAll(doc, 'none', true)
    expect(renderResult(doc)).toBe('top\nmid\nend\n')
    expect(unresolvedCount(doc)).toBe(0)
  })

  it('uses hand-edited text and keeps CRLF and a missing final newline', () => {
    const doc = parseConflictDoc('x\r\n<<<<<<< a\r\n1\r\n=======\r\n2\r\n>>>>>>> b\r\ny')!
    expect(doc.eol).toBe('\r\n')
    const c = doc.segments[1] as ConflictSegment
    c.choice = 'custom'
    c.custom = 'one\ntwo'
    expect(renderResult(doc)).toBe('x\r\none\r\ntwo\r\ny')
    expect(unresolvedCount(doc)).toBe(0)
  })

  it('rejects malformed markers', () => {
    expect(parseConflictDoc('<<<<<<< a\nx\n=======\ny\n')).toBeNull()
    expect(parseConflictDoc('<<<<<<< a\n<<<<<<< b\n')).toBeNull()
  })
})
