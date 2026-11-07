const fmt = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' })

export function formatDate(unixSeconds: number): string {
  return fmt.format(new Date(unixSeconds * 1000))
}

const badges: Record<string, [string, string, string]> = {
  added: ['A', 'text-green-500', 'Added'],
  untracked: ['U', 'text-green-500', 'New file (untracked)'],
  modified: ['M', 'text-accent', 'Modified'],
  deleted: ['D', 'text-red-500', 'Deleted'],
  renamed: ['R', 'text-blue-500', 'Renamed'],
  copied: ['C', 'text-blue-500', 'Copied'],
  typechange: ['T', 'text-muted', 'Type changed'],
}

/** [letter, tailwind color class, label] for a file status. */
export function statusBadge(status: string): [string, string, string] {
  return badges[status] ?? ['?', 'text-muted', status]
}

export function plural(n: number, one: string, many = one + 's'): string {
  return `${n} ${n === 1 ? one : many}`
}

const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto', style: 'short' })
const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 31536000],
  ['month', 2592000],
  ['week', 604800],
  ['day', 86400],
  ['hour', 3600],
  ['minute', 60],
]

/** "3 hr. ago", "yesterday", "2 wk. ago". Falls back to "now" under a minute. */
export function relativeTime(unixSeconds: number, nowMs = Date.now()): string {
  const diff = unixSeconds - nowMs / 1000 // negative = past
  const abs = Math.abs(diff)
  for (const [unit, secs] of UNITS) {
    if (abs >= secs) return rtf.format(Math.round(diff / secs), unit)
  }
  return 'now'
}

/** Splits "src/lib/a.ts" into ["src/lib/", "a.ts"] for two-tone file rows. */
export function splitPath(path: string): [string, string] {
  const i = path.lastIndexOf('/')
  return i < 0 ? ['', path] : [path.slice(0, i + 1), path.slice(i + 1)]
}

/**
 * Splits `text` into segments, marking the parts that match any whitespace-separated term of
 * `query` (case-insensitive). Used to highlight search hits.
 */
export function highlightParts(text: string, query: string): { text: string; hit: boolean }[] {
  const terms = query
    .split(/\s+/)
    .filter(Boolean)
    .map((t) => t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
  if (!terms.length) return [{ text, hit: false }]
  // one capture group: split() puts the matched text at every odd index
  return text
    .split(new RegExp(`(${terms.join('|')})`, 'i'))
    .map((part, i) => ({ text: part, hit: i % 2 === 1 }))
    .filter((p) => p.text !== '')
}

/** ISO timestamps from the hosting service ("2026-11-01T10:00:00Z") to unix seconds. */
export function isoToUnix(iso: string): number {
  const t = Date.parse(iso)
  return Number.isNaN(t) ? 0 : t / 1000
}

/** "3 hr. ago" for an ISO timestamp, or an empty string if it can't be read. */
export function relativeIso(iso: string): string {
  const t = isoToUnix(iso)
  return t ? relativeTime(t) : ''
}

export function formatIso(iso: string): string {
  const t = isoToUnix(iso)
  return t ? formatDate(t) : ''
}
