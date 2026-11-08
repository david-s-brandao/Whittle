import { expect, it } from 'vitest'
import { githubLinks } from './forgeLinks'
import type { ForgeStatus } from './types'

const status: ForgeStatus = {
  supported: true,
  provider: 'github',
  host: 'github.example.com',
  owner: 'team',
  repo: 'app',
  connected: false,
  login: null,
  source: 'none',
  message: null,
}

it('supports enterprise hosts and encodes branch names in PR links without requiring login', () => {
  const links = githubLinks(status, 'feat/auto-fetch#1')!
  expect(links.newPull).toBe(
    'https://github.example.com/team/app/compare/feat%2Fauto-fetch%231?expand=1',
  )
  expect(links.actions).toBe('https://github.example.com/team/app/actions')
})

it('does not offer a PR for detached HEAD or GitHub actions for other providers', () => {
  expect(githubLinks(status, null)?.newPull).toBeNull()
  expect(githubLinks(null, 'main')).toBeNull()
  expect(githubLinks({ ...status, provider: 'gitlab' }, 'main')).toBeNull()
})
