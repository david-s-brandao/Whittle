/** Session-only drafts, keyed by the canonical issue/PR URL. */
const drafts = new Map<string, string>()

export function readCommentDraft(key: string): string {
  return drafts.get(key) ?? ''
}

export function saveCommentDraft(key: string, body: string) {
  drafts.delete(key)
  if (body) drafts.set(key, body)
  if (drafts.size > 100) drafts.delete(drafts.keys().next().value!)
}

/** Do not erase a newer draft if an earlier submission finishes after navigation. */
export function completeCommentDraft(key: string, submitted: string) {
  if (drafts.get(key) === submitted) drafts.delete(key)
}

export type MarkdownAction = 'bold' | 'italic' | 'code' | 'link' | 'task'

export function formatSelection(body: string, start: number, end: number, action: MarkdownAction) {
  const selected = body.slice(start, end)
  const wrappers = {
    bold: ['**', '**'],
    italic: ['_', '_'],
    code: ['`', '`'],
    link: ['[', '](https://)'],
    task: ['', ''],
  }
  const [before, after] = wrappers[action]
  const text = selected || (action === 'link' ? 'link text' : action === 'task' ? 'Task' : 'text')
  const replacement =
    action === 'task'
      ? `${start > 0 && body[start - 1] !== '\n' ? '\n' : ''}${text
          .split('\n')
          .map((line) => `- [ ] ${line}`)
          .join('\n')}`
      : `${before}${text}${after}`
  return {
    body: body.slice(0, start) + replacement + body.slice(end),
    start: start + (action === 'task' ? replacement.length : before.length),
    end: start + (action === 'task' ? replacement.length : before.length + text.length),
  }
}
