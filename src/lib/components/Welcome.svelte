<script lang="ts">
  import logo from '../../assets/logo.png'
  import { relativeTime } from '../format'
  import { clone } from '../stores/clone.svelte'
  import { projects } from '../stores/projects.svelte'
  import Icon from './Icon.svelte'

  const recent = $derived(projects.list.slice(0, 5))
</script>

<div class="flex h-full flex-col items-center justify-center px-6">
  <img src={logo} alt="" class="size-20 select-none" draggable="false" />
  <h1 class="mt-5 text-2xl font-semibold tracking-tight">Whittle</h1>
  <p class="mt-1 text-muted">Git, carved down to what matters.</p>

  <div class="mt-7 flex gap-3">
    <button class="btn btn-primary btn-lg" onclick={() => projects.openPicker()}>
      <Icon name="folder" /> Open repository
    </button>
    <button class="btn btn-outline btn-lg" onclick={() => clone.show()}>
      <Icon name="download" /> Clone repository
    </button>
  </div>
  {#if projects.error}
    <p class="mt-4 text-del">{projects.error}</p>
  {/if}

  {#if recent.length}
    <div class="mt-10 w-full max-w-md">
      <div class="section-label mb-2 px-1">Recent</div>
      <ul class="space-y-0.5">
        {#each recent as p (p.path)}
          <li>
            <button
              class="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left hover:bg-hover"
              onclick={() => projects.openPath(p.path)}
            >
              <Icon
                name={p.starred ? 'star' : 'folder'}
                fill={p.starred}
                class={p.starred ? 'text-accent' : 'text-faint'}
              />
              <span class="min-w-0 flex-1">
                <span class="block truncate font-medium">{p.name}</span>
                <span class="block truncate text-[11.5px] text-muted">{p.path}</span>
              </span>
              <span class="shrink-0 text-[11.5px] text-faint">{relativeTime(p.last_used)}</span>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
  <p class="mt-10 text-[11.5px] text-faint">
    <span class="kbd">Ctrl</span> <span class="kbd">P</span> switch project
  </p>
</div>
