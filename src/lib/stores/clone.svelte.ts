import { api } from '../api'
import type { ProgressOp, SyncError } from '../types'
import { projects } from './projects.svelte'
import { settings } from './settings.svelte'

const DIR_KEY = 'whittle.cloneDir'
const OLD_DIR_KEY = 'knife.cloneDir'

function savedDir(): string | null {
  try {
    return localStorage.getItem(DIR_KEY) ?? localStorage.getItem(OLD_DIR_KEY)
  } catch {
    return null
  }
}

/** State of the "Clone repository" dialog. */
class Clone {
  open = $state(false)
  url = $state('')
  name = $state('')
  parent = $state('')
  running = $state(false)
  line = $state('')
  percent = $state<number | null>(null)
  error = $state<SyncError | null>(null)
  #nameEdited = false
  #urlGen = 0

  async show() {
    this.url = ''
    this.name = ''
    this.#nameEdited = false
    this.error = null
    this.running = false
    this.line = ''
    this.percent = null
    // an explicit default from Settings wins over the last-used folder
    this.parent = settings.value.clone_dir ?? savedDir() ?? (await api.defaultCloneDir())
    this.open = true
  }

  close() {
    if (!this.running) this.open = false
  }

  /** Update the URL and, unless the user typed a folder name, derive one from it. */
  async setUrl(url: string) {
    this.url = url
    this.error = null
    const gen = ++this.#urlGen
    const derived = await api.cloneName(url)
    if (gen === this.#urlGen && !this.#nameEdited) this.name = derived
  }

  setName(name: string) {
    this.name = name
    this.#nameEdited = name.length > 0
    this.error = null
  }

  async browse() {
    const dir = await api.pickFolder()
    if (dir) this.parent = dir
  }

  get target(): string {
    return this.parent && this.name ? `${this.parent.replace(/\/$/, '')}/${this.name}` : ''
  }

  get kind(): 'ssh' | 'https' | 'other' | null {
    const u = this.url.trim()
    if (!u) return null
    if (/^(ssh:\/\/|[\w.-]+@[\w.-]+:)/.test(u)) return 'ssh'
    if (/^https?:\/\//.test(u)) return 'https'
    return 'other'
  }

  onProgress(op: ProgressOp, line: string) {
    if (!this.running || op !== 'clone') return
    this.line = line
    const m = line.match(/(\d{1,3})%/)
    if (m) this.percent = Math.min(100, Number(m[1]))
  }

  async start() {
    if (this.running || !this.url.trim() || !this.name) return
    this.running = true
    this.error = null
    this.line = 'Starting…'
    this.percent = null
    try {
      const path = await api.cloneRepo(this.url.trim(), this.parent, this.name)
      try {
        localStorage.setItem(DIR_KEY, this.parent)
      } catch {
        // storage unavailable: remembering the folder is a convenience only
      }
      this.running = false
      this.open = false
      await projects.openPath(path)
    } catch (e) {
      this.running = false
      this.error =
        e && typeof e === 'object' && 'message' in e
          ? (e as SyncError)
          : { kind: 'other', message: String(e), details: '' }
    }
  }
}

export const clone = new Clone()
