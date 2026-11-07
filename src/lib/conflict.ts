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
