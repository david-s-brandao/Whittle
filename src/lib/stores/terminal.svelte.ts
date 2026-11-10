import { api } from '../api'

export interface TermTab {
  /** Frontend key, stable for the life of the tab. */
  key: number
  /** Repository the shell started in. */
  path: string
  title: string
}

type Sink = { write: (data: string) => void; exit: () => void }

/** Terminal tabs per repository. Shells keep running while you look at other tabs or projects. */
class Terminals {
  tabs = $state<TermTab[]>([])
  active = $state<Record<string, number>>({})
  private nextKey = 1
  private sinks = new Map<number, Sink>() // pty id -> the xterm showing it
  private listening = false

  /** Subscribes to backend output once. */
  listen() {
    if (this.listening) return
    this.listening = true
    void api.onPtyData((id, data) => this.sinks.get(id)?.write(data))
    void api.onPtyExit((id) => this.sinks.get(id)?.exit())
  }

  attach(id: number, sink: Sink) {
    this.sinks.set(id, sink)
  }
  detach(id: number) {
    this.sinks.delete(id)
  }

  forPath(path: string | undefined): TermTab[] {
    return path ? this.tabs.filter((t) => t.path === path) : []
  }

  activeFor(path: string | undefined): number | null {
    if (!path) return null
    const list = this.forPath(path)
    const k = this.active[path]
    return list.some((t) => t.key === k) ? k : (list[0]?.key ?? null)
  }

  add(path: string) {
    const n = this.forPath(path).length + 1
    const tab = { key: this.nextKey++, path, title: `Terminal ${n}` }
    this.tabs.push(tab)
    this.active[path] = tab.key
    return tab
  }

  select(path: string, key: number) {
    this.active[path] = key
  }

  close(key: number) {
    const tab = this.tabs.find((t) => t.key === key)
    if (!tab) return
    this.tabs = this.tabs.filter((t) => t.key !== key)
    if (this.active[tab.path] === key) delete this.active[tab.path]
  }
}

export const terminals = new Terminals()
