import { describe, expect, it } from 'vitest'
import { cycle, parseSaved, restore, withTab, withoutTab } from './tabs'

describe('tabs', () => {
  it('adds a tab once', () => {
    expect(withTab(['a'], 'b')).toEqual(['a', 'b'])
    expect(withTab(['a', 'b'], 'a')).toEqual(['a', 'b'])
  })

  it('closing picks the right neighbour', () => {
    expect(withoutTab(['a', 'b', 'c'], 'b')).toEqual({ tabs: ['a', 'c'], next: 'c' })
    expect(withoutTab(['a', 'b', 'c'], 'c')).toEqual({ tabs: ['a', 'b'], next: 'b' })
    expect(withoutTab(['a'], 'a')).toEqual({ tabs: [], next: null })
    expect(withoutTab(['a', 'b'], 'zzz')).toEqual({ tabs: ['a', 'b'], next: null })
  })

  it('cycles with wrap-around in both directions', () => {
    const t = ['a', 'b', 'c']
    expect(cycle(t, 'a', 1)).toBe('b')
    expect(cycle(t, 'c', 1)).toBe('a')
    expect(cycle(t, 'a', -1)).toBe('c')
    expect(cycle(t, null, 1)).toBe('a')
    expect(cycle([], 'a', 1)).toBeNull()
  })

  it('restore drops unknown and duplicate tabs and repairs the active one', () => {
    expect(restore({ open: ['a', 'x', 'b', 'a'], active: 'b' }, ['a', 'b'])).toEqual({
      open: ['a', 'b'],
      active: 'b',
    })
    expect(restore({ open: ['a', 'b'], active: 'gone' }, ['a', 'b'])).toEqual({
      open: ['a', 'b'],
      active: 'a',
    })
    expect(restore({ open: ['x'], active: 'x' }, ['a'])).toEqual({ open: [], active: null })
  })

  it('parseSaved survives junk', () => {
    expect(parseSaved(null)).toEqual({ open: [], active: null })
    expect(parseSaved('not json')).toEqual({ open: [], active: null })
    expect(parseSaved('{"open":[1,"a"],"active":5}')).toEqual({ open: ['a'], active: null })
    expect(parseSaved('{"open":["a"],"active":"a"}')).toEqual({ open: ['a'], active: 'a' })
  })
})
