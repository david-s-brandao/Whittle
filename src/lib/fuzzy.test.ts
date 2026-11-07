import { describe, expect, it } from 'vitest'
import { fuzzyFilter, fuzzyScore } from './fuzzy'

describe('fuzzy', () => {
  it('matches subsequences, rejects others', () => {
    expect(fuzzyScore('knf', 'knife')).not.toBeNull()
    expect(fuzzyScore('xyz', 'knife')).toBeNull()
  })
  it('ranks tighter matches first', () => {
    const r = fuzzyFilter(['k-n-i-f-e', 'knife'], 'knife', (s) => s)
    expect(r[0]).toBe('knife')
  })
  it('empty query keeps order', () => {
    expect(fuzzyFilter(['b', 'a'], '', (s) => s)).toEqual(['b', 'a'])
  })
})
