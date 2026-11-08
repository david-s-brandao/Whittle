import { describe, expect, it } from 'vitest'
import { FetchSchedule } from './fetchSchedule'

describe('automatic fetch scheduling', () => {
  it('waits five minutes per repository and does not reset on tab switching', () => {
    const clock = new FetchSchedule()
    expect(clock.due('/a', 5, 0)).toBe(false)
    expect(clock.due('/b', 5, 60_000)).toBe(false)
    expect(clock.due('/a', 5, 299_999)).toBe(false)
    expect(clock.due('/a', 5, 300_000)).toBe(true)
    expect(clock.due('/b', 5, 300_000)).toBe(false)
  })

  it('backs off failures and resets the clock after a successful manual fetch', () => {
    const clock = new FetchSchedule()
    clock.finished('/a', false, 0)
    expect(clock.due('/a', 5, 300_000)).toBe(false)
    expect(clock.due('/a', 5, 600_000)).toBe(true)
    clock.finished('/a', true, 600_000)
    expect(clock.due('/a', 5, 899_999)).toBe(false)
    expect(clock.due('/a', 5, 900_000)).toBe(true)
  })

  it('respects disabling and interval changes; caps retries at an hour', () => {
    const clock = new FetchSchedule()
    clock.finished('/a', true, 0)
    expect(clock.due('/a', 0, 999_999)).toBe(false)
    expect(clock.due('/a', NaN, 999_999)).toBe(false)
    expect(clock.due('/a', 1, 60_000)).toBe(true)
    for (let i = 0; i < 10; i++) clock.finished('/a', false, 0)
    expect(clock.due('/a', 5, 3_599_999)).toBe(false)
    expect(clock.due('/a', 5, 3_600_000)).toBe(true)
  })
})
