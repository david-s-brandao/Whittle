<script lang="ts">
  import { theme, type ThemeChoice } from '../stores/theme.svelte'
  import Icon from './Icon.svelte'

  const order: ThemeChoice[] = ['system', 'light', 'dark']
  const icon = { system: 'monitor', light: 'sun', dark: 'moon' } as const
  const label = { system: 'Follow system', light: 'Light', dark: 'Dark' } as const

  function next() {
    theme.set(order[(order.indexOf(theme.choice) + 1) % order.length])
  }
</script>

<button class="icon-btn" title="Theme: {label[theme.choice]} (click to change)" onclick={next}>
  <Icon name={icon[theme.choice]} />
</button>
