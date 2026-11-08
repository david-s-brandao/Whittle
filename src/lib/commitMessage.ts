export const commitTypes = [
  'feat',
  'fix',
  'docs',
  'refactor',
  'test',
  'chore',
  'perf',
  'ci',
] as const

/** Change a conventional commit's type without losing its scope, body or breaking-change flag. */
export function withCommitType(message: string, type: string): string {
  const newline = message.indexOf('\n')
  const subject = newline < 0 ? message : message.slice(0, newline)
  const body = newline < 0 ? '' : message.slice(newline)
  const match = subject.match(/^[a-z]+(\([^\r\n)]+\))?(!)?:\s*(.*)$/)
  return match
    ? `${type}${match[1] ?? ''}${match[2] ?? ''}: ${match[3]}${body}`
    : `${type}: ${subject}${body}`
}
