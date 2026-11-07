/** Parses the unified-diff text GitHub returns for each file of a pull request. */

export interface PatchLine {
  kind: '+' | '-' | ' ' | '\\'
  old_no: number | null
  new_no: number | null
  text: string
}

export interface PatchHunk {
  header: string
  lines: PatchLine[]
}

export function parsePatch(patch: string | null | undefined): PatchHunk[] {
  if (!patch) return []
  const hunks: PatchHunk[] = []
  let cur: PatchHunk | null = null
  let oldNo = 0
  let newNo = 0
  for (const raw of patch.replace(/\r\n?/g, '\n').split('\n')) {
    const m = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(raw)
    if (m) {
      cur = { header: raw, lines: [] }
      hunks.push(cur)
      oldNo = Number(m[1])
      newNo = Number(m[2])
      continue
    }
    if (!cur) continue
    const c = raw[0]
    const text = raw.slice(1)
    if (c === '+') cur.lines.push({ kind: '+', old_no: null, new_no: newNo++, text })
    else if (c === '-') cur.lines.push({ kind: '-', old_no: oldNo++, new_no: null, text })
    else if (c === '\\')
      cur.lines.push({ kind: '\\', old_no: null, new_no: null, text: raw.slice(2) })
    else if (c === ' ') cur.lines.push({ kind: ' ', old_no: oldNo++, new_no: newNo++, text })
    // anything else (blank trailing line) is ignored
  }
  return hunks
}
