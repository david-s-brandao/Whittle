import { expect, it } from 'vitest'
import { withCommitType } from './commitMessage'

it('adds a type without replacing the subject or body', () => {
  expect(withCommitType('Improve fetching\n\nDetails', 'feat')).toBe(
    'feat: Improve fetching\n\nDetails',
  )
  expect(withCommitType('', 'fix')).toBe('fix: ')
})

it('changes an existing type while preserving scope and breaking-change markers', () => {
  expect(
    withCommitType('feat(sync)!: change defaults\n\nBREAKING CHANGE: new defaults', 'fix'),
  ).toBe('fix(sync)!: change defaults\n\nBREAKING CHANGE: new defaults')
  expect(withCommitType('docs: update notes', 'chore')).toBe('chore: update notes')
})
