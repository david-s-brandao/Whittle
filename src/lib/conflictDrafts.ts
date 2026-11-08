import type { ConflictDoc } from './conflict'

// Session-only drafts: never write repository contents to browser storage.
const drafts = new Map<string, { source: string; doc: ConflictDoc }>()

export function readConflictDraft(key: string) {
  const draft = drafts.get(key)
  return draft ? structuredClone(draft) : undefined
}

export function writeConflictDraft(key: string, source: string, doc: ConflictDoc) {
  drafts.delete(key)
  drafts.set(key, { source, doc: structuredClone(doc) })
  // Bound memory when working across many repositories.
  if (drafts.size > 50) drafts.delete(drafts.keys().next().value!)
}

export function clearConflictDraft(key: string) {
  drafts.delete(key)
}
