import { describe, expect, it } from 'vitest'
import { parseConflictDoc } from './conflict'
import { clearConflictDraft, readConflictDraft, writeConflictDraft } from './conflictDrafts'

describe('session conflict drafts', () => {
  it('isolates repositories and retains the original source for stale detection', () => {
    const source = '<<<<<<< HEAD\na\n=======\nb\n>>>>>>> incoming\n'
    const doc = parseConflictDoc(source)!
    const key = JSON.stringify(['/repo-a', 'file'])
    writeConflictDraft(key, source, doc)
    doc.segments = []
    const restored = readConflictDraft(key)!
    expect(restored.source).toBe(source)
    expect(restored.doc.segments).toHaveLength(1)
    restored.doc.segments = []
    expect(readConflictDraft(key)!.doc.segments).toHaveLength(1)
    expect(readConflictDraft(JSON.stringify(['/repo-b', 'file']))).toBeUndefined()
    clearConflictDraft(key)
    expect(readConflictDraft(key)).toBeUndefined()
  })
})
