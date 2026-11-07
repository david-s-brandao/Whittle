import { describe, expect, it } from 'vitest'
import { suggestSources } from './mergeSources'
import type { BranchInfo } from './types'

const b = (name: string, upstream: string | null = null): BranchInfo => ({
  name,
  is_head: false,
  upstream,
})

describe('suggestSources', () => {
  const local = [b('feature', 'origin/feature'), b('main', 'origin/main'), b('scratch')]
  const remote = [b('origin/feature'), b('origin/main'), b('origin/other')]

  it('puts the tracked branch first, then remote defaults, then local defaults', () => {
    const names = suggestSources(local, remote, 'feature').map((s) => s.name)
    expect(names).toEqual(['origin/feature', 'origin/main', 'main'])
  })

  it('never suggests the current branch and does not duplicate', () => {
    const names = suggestSources(local, remote, 'main').map((s) => s.name)
    expect(names).toEqual(['origin/main'])
    expect(names).not.toContain('main')
  })

  it('works without any remote or upstream', () => {
    expect(suggestSources([b('main'), b('topic')], [], 'topic').map((s) => s.name)).toEqual([
      'main',
    ])
    expect(suggestSources([b('topic')], [], 'topic')).toEqual([])
  })

  it('recognises master and nested remote names', () => {
    const names = suggestSources(
      [b('topic')],
      [b('upstream/master'), b('origin/master')],
      'topic',
    ).map((s) => s.name)
    expect(names).toEqual(['upstream/master', 'origin/master'])
  })
})
