import type { ForgeStatus } from './types'

export function githubLinks(status: ForgeStatus | null, branch: string | null) {
  if (!status?.supported || status.provider !== 'github') return null
  const base = `https://${status.host}/${encodeURIComponent(status.owner)}/${encodeURIComponent(status.repo)}`
  return {
    repository: base,
    actions: `${base}/actions`,
    newIssue: `${base}/issues/new`,
    newPull: branch ? `${base}/compare/${encodeURIComponent(branch)}?expand=1` : null,
  }
}
