/** Remembered choices (merge options, pull mode). Stored in this browser profile only. */

const KEY = 'whittle.prefs'

export type MergeFf = 'auto' | 'never' | 'only'
export type PullMode = 'default' | 'merge' | 'rebase' | 'ff-only'

interface Saved {
  mergeFf: MergeFf
  mergeSquash: boolean
  mergeNoCommit: boolean
  pullMode: PullMode
  pullAutostash: boolean
}

const defaults: Saved = {
  mergeFf: 'auto',
  mergeSquash: false,
  mergeNoCommit: false,
  pullMode: 'default',
  pullAutostash: false,
}

function load(): Saved {
  try {
    const raw = localStorage.getItem(KEY)
    return raw ? { ...defaults, ...(JSON.parse(raw) as Partial<Saved>) } : { ...defaults }
  } catch {
    return { ...defaults }
  }
}

class Prefs {
  mergeFf = $state<MergeFf>(defaults.mergeFf)
  mergeSquash = $state(defaults.mergeSquash)
  mergeNoCommit = $state(defaults.mergeNoCommit)
  pullMode = $state<PullMode>(defaults.pullMode)
  pullAutostash = $state(defaults.pullAutostash)

  constructor() {
    const s = load()
    this.mergeFf = s.mergeFf
    this.mergeSquash = s.mergeSquash
    this.mergeNoCommit = s.mergeNoCommit
    this.pullMode = s.pullMode
    this.pullAutostash = s.pullAutostash
  }

  save() {
    const s: Saved = {
      mergeFf: this.mergeFf,
      mergeSquash: this.mergeSquash,
      mergeNoCommit: this.mergeNoCommit,
      pullMode: this.pullMode,
      pullAutostash: this.pullAutostash,
    }
    try {
      localStorage.setItem(KEY, JSON.stringify(s))
    } catch {
      // not remembered, still works for this session
    }
  }
}

export const prefs = new Prefs()
