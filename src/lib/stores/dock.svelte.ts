import { ui } from './ui.svelte'

export type DockTab = 'files' | 'description' | 'terminal' | 'issues' | 'pulls'

const KEY = 'whittle.dock'
const OLD_HEIGHT_KEY = 'whittle.detailsHeight'
export const DOCK_MIN = 140

interface Saved {
  open: boolean
  tab: DockTab
  height: number
}

const TABS: DockTab[] = ['files', 'description', 'terminal', 'issues', 'pulls']

function load(): Saved {
  const d: Saved = { open: true, tab: 'files', height: 320 }
  try {
    const old = Number(localStorage.getItem(OLD_HEIGHT_KEY))
    if (old >= DOCK_MIN) d.height = old
    const v = JSON.parse(localStorage.getItem(KEY) ?? 'null')
    if (v && typeof v === 'object') {
      if (typeof v.open === 'boolean') d.open = v.open
      if (TABS.includes(v.tab)) d.tab = v.tab
      if (typeof v.height === 'number' && v.height >= DOCK_MIN) d.height = v.height
    }
  } catch {
    // defaults
  }
  return d
}

/** The bottom panel: which tab is chosen, how tall it is, and whether it is open or maximized. */
class Dock {
  private saved = load()
  open = $state(this.saved.open)
  /** The tab the user chose; use `active` for the one actually shown. */
  tab = $state<DockTab>(this.saved.tab)
  height = $state(this.saved.height)
  /** The panel takes the whole area, hiding the main view above it. */
  maximized = $state(false)

  /** Tabs that make sense with the main view as it is now. */
  get available(): DockTab[] {
    return TABS.filter((t) => {
      if (t === 'files' || t === 'description') return ui.view === 'history'
      if (t === 'issues' || t === 'pulls') return ui.view !== t
      return true
    })
  }

  get active(): DockTab {
    return this.available.includes(this.tab) ? this.tab : this.available[0]
  }

  persist() {
    try {
      localStorage.setItem(
        KEY,
        JSON.stringify({ open: this.open, tab: this.tab, height: this.height }),
      )
    } catch {
      // not remembered
    }
  }

  /** Opens the panel on `tab` (or just opens it). Choosing the shown tab again collapses it. */
  show(tab?: DockTab) {
    if (tab && this.open && this.active === tab && !this.maximized) {
      this.open = false
    } else {
      if (tab) this.tab = tab
      this.open = true
    }
    this.persist()
  }

  select(tab: DockTab) {
    this.tab = tab
    this.open = true
    this.persist()
  }

  toggle() {
    this.open = !this.open
    if (!this.open) this.maximized = false
    this.persist()
  }

  toggleMaximize() {
    this.maximized = !this.maximized
    if (this.maximized) this.open = true
    this.persist()
  }

  /** "Pull down" a forge tab: commits stay on top, the board / list goes to the panel. */
  pullDown(kind: 'issues' | 'pulls') {
    ui.view = 'history'
    this.tab = kind
    this.open = true
    this.persist()
  }

  /** The reverse: bring a docked forge tab back to the main area. */
  pullUp(kind: 'issues' | 'pulls') {
    this.maximized = false
    ui.view = kind
  }
}

export const dock = new Dock()
