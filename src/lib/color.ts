/** Small colour helpers for the accent setting. */

export function parseHex(hex: string): [number, number, number] | null {
  const m = /^#([0-9a-f]{6})$/i.exec(hex.trim())
  if (!m) return null
  const n = parseInt(m[1], 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}

const toHex = (r: number, g: number, b: number) =>
  '#' +
  [r, g, b]
    .map((v) =>
      Math.round(Math.min(255, Math.max(0, v)))
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')

/** Mix with black (amount > 0) or white (amount < 0). Used for the hover shade. */
export function shade(hex: string, amount: number): string {
  const rgb = parseHex(hex)
  if (!rgb) return hex
  const target = amount >= 0 ? 0 : 255
  const t = Math.abs(amount)
  return toHex(...(rgb.map((v) => v + (target - v) * t) as [number, number, number]))
}

/** WCAG contrast ratio between two colours. */
export function contrast(a: string, b: string): number {
  const lum = (hex: string) => {
    const rgb = parseHex(hex) ?? [0, 0, 0]
    const [r, g, bl] = rgb.map((v) => {
      const c = v / 255
      return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
    })
    return 0.2126 * r + 0.7152 * g + 0.0722 * bl
  }
  const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x)
  return (hi + 0.05) / (lo + 0.05)
}

/**
 * Black or white text for a label background, by perceived brightness (the same rule GitHub
 * uses, so labels look the way they do on the website).
 */
export function readableText(bg: string): '#000000' | '#ffffff' {
  const rgb = parseHex(bg.startsWith('#') ? bg : `#${bg}`)
  if (!rgb) return '#000000'
  const [r, g, b] = rgb
  return (r * 299 + g * 587 + b * 114) / 1000 >= 150 ? '#000000' : '#ffffff'
}
