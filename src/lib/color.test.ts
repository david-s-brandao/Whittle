import { describe, expect, it } from 'vitest'
import { contrast, parseHex, readableText, shade } from './color'

describe('color', () => {
  it('parses hex and rejects junk', () => {
    expect(parseHex('#f66008')).toEqual([246, 96, 8])
    expect(parseHex('#F66008')).toEqual([246, 96, 8])
    expect(parseHex('f66008')).toBeNull()
    expect(parseHex('#fff')).toBeNull()
  })
  it('shade darkens and lightens, staying valid hex', () => {
    expect(shade('#ffffff', 0.5)).toBe('#808080')
    expect(shade('#000000', -0.5)).toBe('#808080')
    expect(shade('#f66008', 0)).toBe('#f66008')
    expect(shade('#f66008', 0.1)).toMatch(/^#[0-9a-f]{6}$/)
    expect(shade('nonsense', 0.2)).toBe('nonsense')
  })
  it('contrast follows WCAG', () => {
    expect(contrast('#000000', '#ffffff')).toBeCloseTo(21, 0)
    expect(contrast('#777777', '#777777')).toBeCloseTo(1, 5)
  })
  it('readableText picks black on light and white on dark labels', () => {
    expect(readableText('fbca04')).toBe('#000000') // yellow
    expect(readableText('#d73a4a')).toBe('#ffffff') // red
    expect(readableText('0e8a16')).toBe('#ffffff') // green
    expect(readableText('ffffff')).toBe('#000000')
  })
})
