import { expect, it } from 'vitest'
import {
  completeCommentDraft,
  formatSelection,
  readCommentDraft,
  saveCommentDraft,
} from './commentDrafts'

it('isolates thread drafts and preserves newer text when an earlier post completes', () => {
  saveCommentDraft('repo-a/issues/1', 'first draft')
  saveCommentDraft('repo-b/issues/1', 'other project')
  saveCommentDraft('repo-a/issues/1', 'new draft')
  completeCommentDraft('repo-a/issues/1', 'first draft')
  expect(readCommentDraft('repo-a/issues/1')).toBe('new draft')
  completeCommentDraft('repo-a/issues/1', 'new draft')
  expect(readCommentDraft('repo-a/issues/1')).toBe('')
  expect(readCommentDraft('repo-b/issues/1')).toBe('other project')
  saveCommentDraft('repo-b/issues/1', '')
  expect(readCommentDraft('repo-b/issues/1')).toBe('')
})

it('formats a selection without losing surrounding text and selects the editable content', () => {
  expect(formatSelection('hello world!', 6, 11, 'bold')).toEqual({
    body: 'hello **world**!',
    start: 8,
    end: 13,
  })
  expect(formatSelection('', 0, 0, 'link')).toEqual({
    body: '[link text](https://)',
    start: 1,
    end: 10,
  })
})

it('makes multiline task lists and inserts a newline after existing text', () => {
  expect(formatSelection('intro', 5, 5, 'task').body).toBe('intro\n- [ ] Task')
  expect(formatSelection('one\ntwo', 0, 7, 'task').body).toBe('- [ ] one\n- [ ] two')
})
