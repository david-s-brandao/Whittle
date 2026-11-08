/** Per-repository clock, independent of UI timers. Failed fetches back off up to 1 hour. */
export class FetchSchedule {
  #entries = new Map<string, { at: number; failures: number }>()

  due(path: string, minutes: number, now: number): boolean {
    if (!Number.isFinite(minutes) || minutes <= 0) return false
    const entry = this.#entries.get(path)
    if (!entry) {
      this.#entries.set(path, { at: now, failures: 0 })
      return false
    }
    const delay = Math.min(60, minutes * 2 ** Math.min(entry.failures, 6)) * 60_000
    return now - entry.at >= delay
  }

  finished(path: string, success: boolean, now: number) {
    const failures = success ? 0 : (this.#entries.get(path)?.failures ?? 0) + 1
    this.#entries.set(path, { at: now, failures })
  }
}
