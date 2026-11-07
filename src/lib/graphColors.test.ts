import { describe, expect, it } from 'vitest'
import { lanePalette } from './graphColors'

const rgb = (h: string): [number, number, number] =>
  [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255) as [number, number, number]
const lin = (v: number) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)
const lum = (h: string) => {
  const [r, g, b] = rgb(h).map(lin)
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}
const contrast = (a: string, b: string) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p)
  return (x + 0.05) / (y + 0.05)
}
// distance in (approximate) perceptual space: redmean weighted RGB
const dist = (a: string, b: string) => {
  const [r1, g1, b1] = rgb(a).map((v) => v * 255)
  const [r2, g2, b2] = rgb(b).map((v) => v * 255)
  const rm = (r1 + r2) / 2
  return Math.sqrt(
    (2 + rm / 256) * (r1 - r2) ** 2 + 4 * (g1 - g2) ** 2 + (2 + (255 - rm) / 256) * (b1 - b2) ** 2,
  )
}

describe('lane palettes', () => {
  for (const [name, dark, bg] of [
    ['light', false, '#ffffff'],
    ['dark', true, '#111113'],
  ] as const) {
    it(`${name}: valid, unique, readable on the background`, () => {
      const p = lanePalette(dark)
      expect(p).toHaveLength(9)
      expect(new Set(p).size).toBe(9)
      for (const c of p) {
        expect(c).toMatch(/^#[0-9a-f]{6}$/)
        expect(contrast(c, bg), `${c} on ${bg}`).toBeGreaterThanOrEqual(2.6)
      }
    })
    it(`${name}: neighbouring lanes are clearly different`, () => {
      const p = lanePalette(dark)
      for (let i = 0; i < p.length; i++) {
        const next = p[(i + 1) % p.length]
        expect(dist(p[i], next), `${p[i]} vs ${next}`).toBeGreaterThan(110)
      }
    })
  }
})
