import { api } from '../api'
import { shade } from '../color'
import type { Settings } from '../types'

export const defaultSettings: Settings = {
  clone_dir: null,
  git_path: null,
  editor_cmd: null,
  terminal_cmd: null,
  font_size: 13,
  accent: '#f66008',
}

/** Applies appearance settings to the page: font size and the accent colour tokens. */
export function applyAppearance(s: Pick<Settings, 'font_size' | 'accent'>) {
  const root = document.documentElement
  root.style.fontSize = `${s.font_size}px`
  root.style.setProperty('--accent', s.accent)
  root.style.setProperty('--accent-hover', shade(s.accent, 0.12))
}

class SettingsStore {
  value = $state<Settings>({ ...defaultSettings })
  loaded = $state(false)

  async load() {
    try {
      this.value = await api.getSettings()
    } catch {
      // keep defaults: the app is still fully usable
    }
    this.loaded = true
    applyAppearance(this.value)
  }

  /** Validates and stores on the backend; throws the backend's message if invalid. */
  async save(next: Settings) {
    this.value = await api.saveSettings(next)
    applyAppearance(this.value)
  }
}

export const settings = new SettingsStore()
