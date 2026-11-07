import { describe, expect, it } from 'vitest'
import { highlightParts, isoToUnix, relativeTime, splitPath } from './format'

describe('format', () => {
  const now = Date.UTC(2026, 10, 7, 12, 0, 0)
  const ago = (s: number) => now / 1000 - s

  it('relativeTime picks a sensible unit', () => {
    expect(relativeTime(ago(10), now)).toBe('now')
    expect(relativeTime(ago(3 * 3600), now)).toMatch(/3/)
    expect(relativeTime(ago(86400), now)).toMatch(/yesterday/i)
    expect(relativeTime(ago(40 * 86400), now)).toMatch(/mo|month/i)
  })

  it('splitPath keeps the folder part', () => {
    expect(splitPath('src/lib/a.ts')).toEqual(['src/lib/', 'a.ts'])
    expect(splitPath('README.md')).toEqual(['', 'README.md'])
  })
  it('highlightParts marks every matching term, case-insensitively', () => {
    expect(highlightParts('Fix Login bug', 'login')).toEqual([
      { text: 'Fix ', hit: false },
      { text: 'Login', hit: true },
      { text: ' bug', hit: false },
    ])
    const two = highlightParts('add login page', 'LOGIN page')
    expect(two.filter((p) => p.hit).map((p) => p.text)).toEqual(['login', 'page'])
    expect(highlightParts('nothing here', 'zzz')).toEqual([{ text: 'nothing here', hit: false }])
    expect(highlightParts('a+b (c)', 'a+b (c)').some((p) => p.hit)).toBe(true) // regex chars are escaped
    expect(highlightParts('text', '   ')).toEqual([{ text: 'text', hit: false }])
  })
  it('isoToUnix reads GitHub timestamps and survives junk', () => {
    expect(isoToUnix('2026-01-01T00:00:00Z')).toBe(1767225600)
    expect(isoToUnix('not a date')).toBe(0)
    expect(isoToUnix('')).toBe(0)
  })
})
