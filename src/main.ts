import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { theme } from './lib/stores/theme.svelte'

theme.init()

// Dev only: `?mock` swaps in a fake backend so the UI can be viewed in a normal browser.
if (import.meta.env.DEV && new URLSearchParams(location.search).has('mock')) {
  const { installMock } = await import('./dev/mock')
  void installMock()
}

export default mount(App, { target: document.getElementById('app')! })
