export type ThemeChoice = 'light' | 'dark' | 'system'

const KEY = 'whittle.theme'
const OLD_KEY = 'knife.theme' // app was called Knife before
const media = window.matchMedia('(prefers-color-scheme: dark)')

function load(): ThemeChoice {
  const v = localStorage.getItem(KEY) ?? localStorage.getItem(OLD_KEY)
  return v === 'light' || v === 'dark' || v === 'system' ? v : 'system'
}

class Theme {
  choice = $state<ThemeChoice>(load())
  /** Bumped whenever the effective theme may have changed (incl. OS theme switches). */
  rev = $state(0)

  get dark() {
    void this.rev
    return this.choice === 'dark' || (this.choice === 'system' && media.matches)
  }

  set(choice: ThemeChoice) {
    this.choice = choice
    localStorage.setItem(KEY, choice)
    this.apply()
  }

  apply() {
    this.rev++
    document.documentElement.classList.toggle('dark', this.dark)
  }

  init() {
    this.apply()
    media.addEventListener('change', () => this.apply())
  }
}

export const theme = new Theme()
