/**
 * Lane colour palettes, generated in OKLCH so every colour has the same perceived lightness
 * and chroma and only the hue changes.
 */

/**
 * Hues in lane order. Consecutive entries are far apart (>= 70 degrees, including the wrap
 * from last to first) and the orange band (about 20-70 degrees) is skipped on purpose: orange
 * is the UI accent, so a lane should never be mistaken for "selected".
 */
const HUES = [250, 150, 340, 90, 290, 190, 12, 120, 320]

/** [lightness, chroma] tuned so lines read on each background. */
const TUNING = { light: [0.58, 0.16], dark: [0.74, 0.14] } as const

export function oklchToRgb(l: number, c: number, hDeg: number): [number, number, number] {
  const h = (hDeg * Math.PI) / 180
  const a = c * Math.cos(h)
  const b = c * Math.sin(h)
  const l_ = (l + 0.3963377774 * a + 0.2158037573 * b) ** 3
  const m_ = (l - 0.1055613458 * a - 0.0638541728 * b) ** 3
  const s_ = (l - 0.0894841775 * a - 1.291485548 * b) ** 3
  const lin = [
    4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_,
    -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_,
    -0.0041960863 * l_ - 0.7034186147 * m_ + 1.707614701 * s_,
  ]
  const enc = (v: number) => {
    const x = Math.min(1, Math.max(0, v))
    return x <= 0.0031308 ? 12.92 * x : 1.055 * x ** (1 / 2.4) - 0.055
  }
  return [enc(lin[0]), enc(lin[1]), enc(lin[2])]
}

const hex = (rgb: [number, number, number]) =>
  '#' +
  rgb
    .map((v) =>
      Math.round(v * 255)
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')

export function lanePalette(dark: boolean): string[] {
  const [l, c] = dark ? TUNING.dark : TUNING.light
  return HUES.map((h) => hex(oklchToRgb(l, c, h)))
}

/** Palette colour for a lane colour index (indices grow forever; the palette repeats). */
export function laneColor(palette: string[], index: number): string {
  return palette[index % palette.length]
}
