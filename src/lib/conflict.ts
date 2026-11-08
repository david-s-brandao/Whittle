/** Splits a file containing git conflict markers into tagged lines for display. */

export type ConflictLineKind = 'ctx' | 'ours' | 'theirs' | 'base' | 'marker'

export interface ConflictLine {
  no: number
  text: string
  kind: ConflictLineKind
}

export function parseConflicts(text: string): { lines: ConflictLine[]; blocks: number } {
  const lines: ConflictLine[] = []
  let section: 'ctx' | 'ours' | 'base' | 'theirs' = 'ctx'
  let blocks = 0
  text.split('\n').forEach((raw, i) => {
    const text = raw.replace(/\r$/, '')
    let kind: ConflictLineKind = section
    if (section === 'ctx' && text.startsWith('<<<<<<<')) {
      section = 'ours'
      kind = 'marker'
      blocks++
    } else if (section === 'ours' && text.startsWith('|||||||')) {
      section = 'base'
      kind = 'marker'
    } else if ((section === 'ours' || section === 'base') && text === '=======') {
      section = 'theirs'
      kind = 'marker'
    } else if (section === 'theirs' && text.startsWith('>>>>>>>')) {
      section = 'ctx'
      kind = 'marker'
    }
    lines.push({ no: i + 1, text, kind })
  })
  // a trailing newline yields one empty phantom line
  if (lines.length && lines[lines.length - 1].text === '' && text.endsWith('\n')) lines.pop()
  return { lines, blocks }
}

/* ------------------------------------------------------------------------------------------
 * Editable model, used by the merge conflict editor.
 * ---------------------------------------------------------------------------------------- */

/** How a conflict is resolved. `custom` means the user edited the result by hand. */
export type Choice = 'ours' | 'theirs' | 'both' | 'both-rev' | 'none' | 'custom'

export interface TextSegment {
  type: 'text'
  lines: string[]
}

export interface ConflictSegment {
  type: 'conflict'
  ours: string[]
  base: string[] | null
  theirs: string[]
  oursLabel: string
  theirsLabel: string
  /** `null` while unresolved. */
  choice: Choice | null
  /** The hand-edited result (only meaningful when `choice` is `custom`). */
  custom: string
}

export type Segment = TextSegment | ConflictSegment

export interface ConflictDoc {
  segments: Segment[]
  /** Line ending the file uses, kept when saving. */
  eol: '\n' | '\r\n'
  finalNewline: boolean
}

/** Parses a file with conflict markers. Returns `null` if the markers are malformed. */
export function parseConflictDoc(text: string): ConflictDoc | null {
  const eol = text.includes('\r\n') ? '\r\n' : '\n'
  const finalNewline = text.endsWith('\n')
  const raw = text.split('\n').map((l) => l.replace(/\r$/, ''))
  if (finalNewline) raw.pop()
  const segments: Segment[] = []
  let cur: TextSegment | null = null
  let conflict: ConflictSegment | null = null
  let part: 'ours' | 'base' | 'theirs' = 'ours'
  for (const line of raw) {
    if (!conflict) {
      if (line.startsWith('<<<<<<<')) {
        if (cur) segments.push(cur)
        cur = null
        conflict = {
          type: 'conflict',
          ours: [],
          base: null,
          theirs: [],
          oursLabel: line.slice(7).trim(),
          theirsLabel: '',
          choice: null,
          custom: '',
        }
        part = 'ours'
      } else {
        if (!cur) cur = { type: 'text', lines: [] }
        cur.lines.push(line)
      }
      continue
    }
    if (line.startsWith('<<<<<<<')) return null
    if (part === 'ours' && line.startsWith('|||||||')) {
      part = 'base'
      conflict.base = []
    } else if (part !== 'theirs' && line === '=======') {
      part = 'theirs'
    } else if (part === 'theirs' && line.startsWith('>>>>>>>')) {
      conflict.theirsLabel = line.slice(7).trim()
      segments.push(conflict)
      conflict = null
    } else if (part === 'base') {
      conflict.base!.push(line)
    } else {
      conflict[part].push(line)
    }
  }
  if (conflict) return null
  if (cur) segments.push(cur)
  return { segments, eol, finalNewline }
}

export function conflictCount(doc: ConflictDoc): number {
  return doc.segments.filter((s) => s.type === 'conflict').length
}

export function unresolvedCount(doc: ConflictDoc): number {
  return doc.segments.filter((s) => s.type === 'conflict' && s.choice === null).length
}

/** The lines a conflict currently contributes to the result (`null` while unresolved). */
export function resolvedLines(c: ConflictSegment): string[] | null {
  switch (c.choice) {
    case null:
      return null
    case 'ours':
      return c.ours
    case 'theirs':
      return c.theirs
    case 'both':
      return [...c.ours, ...c.theirs]
    case 'both-rev':
      return [...c.theirs, ...c.ours]
    case 'none':
      return []
    case 'custom':
      return c.custom === '' ? [] : c.custom.split('\n')
  }
}

/** The merged file. Unresolved conflicts keep their markers so nothing is silently lost. */
export function renderResult(doc: ConflictDoc): string {
  const out: string[] = []
  for (const s of doc.segments) {
    if (s.type === 'text') {
      out.push(...s.lines)
      continue
    }
    const lines = resolvedLines(s)
    if (lines) {
      out.push(...lines)
    } else {
      out.push(`<<<<<<< ${s.oursLabel}`.trimEnd(), ...s.ours)
      if (s.base) out.push('||||||| base', ...s.base)
      out.push('=======', ...s.theirs, `>>>>>>> ${s.theirsLabel}`.trimEnd())
    }
  }
  if (!out.length) return ''
  return out.join(doc.eol) + (doc.finalNewline ? doc.eol : '')
}

/** Sets the same choice on every unresolved (or, with `all`, every) conflict. */
export function resolveAll(doc: ConflictDoc, choice: Choice, all = false) {
  for (const s of doc.segments) {
    if (s.type === 'conflict' && (all || s.choice === null)) s.choice = choice
  }
}
