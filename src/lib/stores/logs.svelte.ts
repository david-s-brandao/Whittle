import { api } from '../api'
import { projects } from './projects.svelte'

export type LogStatus = 'running' | 'passed' | 'failed' | 'none'

export interface LogRun {
  /** What was run, shown in the header of the Logs tab. */
  title: string
  lines: string[]
  status: LogStatus
  startedAt: number
  endedAt: number | null
}

/** Output of the repository checks (the pre-commit hook), live, per repository. */
class Logs {
  runs = $state<Record<string, LogRun>>({})
  #listening = false

  /** Subscribes to backend `hook-log` events once. */
  listen() {
    if (this.#listening) return
    this.#listening = true
    void api.onHookLog((e) => {
      if (e.kind === 'start') {
        this.runs[e.path] = {
          title: 'pre-commit hook',
          lines: [],
          status: 'running',
          startedAt: Date.now(),
          endedAt: null,
        }
        return
      }
      const run = this.runs[e.path]
      if (!run) return
      if (e.kind === 'line') {
        run.lines.push(e.text)
      } else {
        run.endedAt = Date.now()
        run.status = e.ok ? 'passed' : 'failed'
      }
    })
  }

  get current(): LogRun | undefined {
    const path = projects.current?.path
    return path ? this.runs[path] : undefined
  }

  /** Marks a finished run as "no hook found" (the backend reports that on the result). */
  markNone(path: string) {
    const run = this.runs[path]
    if (!run) return
    run.status = 'none'
    run.lines.push('This repository has no pre-commit hook, so there is nothing to run.')
  }

  clear(path: string | undefined) {
    if (path) delete this.runs[path]
  }
}

export const logs = new Logs()
