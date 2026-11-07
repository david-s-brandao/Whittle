<script lang="ts">
  import { tick } from 'svelte'
  import { fuzzyFilter } from '../fuzzy'
  import { branches } from '../stores/branches.svelte'
  import { repo } from '../stores/repo.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  type Row =
    { kind: 'local' | 'remote'; name: string; current: boolean } | { kind: 'create'; name: string }

  let query = $state('')
  let active = $state(0)
  let switchTo = $state(true)
  let input: HTMLInputElement | undefined = $state()

  const q = $derived(query.trim())
  const local = $derived(
    fuzzyFilter(
      repo.refs.local.map((b) => ({ name: b.name, current: b.is_head })),
      q,
      (b) => b.name,
    ),
  )
  const remote = $derived(
    fuzzyFilter(
      repo.refs.remote.map((b) => ({ name: b.name })),
      q,
      (b) => b.name,
    ),
  )
  const canCreate = $derived(q.length > 0 && !repo.refs.local.some((b) => b.name === q))
  const rows = $derived<Row[]>([
    ...(canCreate ? [{ kind: 'create', name: q } as Row] : []),
    ...local.map((b) => ({ kind: 'local', ...b }) as Row),
    ...remote.map((b) => ({ kind: 'remote', name: b.name, current: false }) as Row),
  ])

  $effect(() => {
    if (ui.branchSwitcherOpen) {
      query = ''
      active = 0
      tick().then(() => input?.focus())
    }
  })

  async function choose(r: Row | undefined) {
    if (!r) return
    ui.branchSwitcherOpen = false
    if (r.kind === 'create') await branches.create(r.name, switchTo)
    else if (!(r.kind === 'local' && r.current))
      await branches.checkout(r.name, r.kind === 'remote')
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') ui.branchSwitcherOpen = false
    else if (e.key === 'ArrowDown') {
      e.preventDefault()
      active = Math.min(active + 1, rows.length - 1)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      active = Math.max(active - 1, 0)
    } else if (e.key === 'Enter') choose(rows[active])
  }
</script>

{#if ui.branchSwitcherOpen}
  <div
    class="backdrop"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && (ui.branchSwitcherOpen = false)}
  >
    <div
      class="modal w-[500px]"
      role="dialog"
      aria-modal="true"
      aria-label="Switch or create branch"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-3 border-b border-border px-4">
        <Icon name="branch" class="text-faint" />
        <input
          bind:this={input}
          bind:value={query}
          oninput={() => (active = 0)}
          placeholder="Switch branch, or type a new name…"
          class="h-12 w-full bg-transparent text-[14px] outline-none placeholder:text-faint"
        />
        <span class="kbd">Esc</span>
      </div>
      <div class="max-h-[50vh] overflow-y-auto p-2" role="listbox">
        {#each rows as r, i (r.kind + r.name)}
          <div
            role="option"
            aria-selected={i === active}
            tabindex="-1"
            class="flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-2 {i === active
              ? 'bg-accent/10'
              : 'hover:bg-hover'}"
            onclick={() => choose(r)}
            onkeydown={null}
            onmousemove={() => (active = i)}
          >
            {#if r.kind === 'create'}
              <span class="grid size-5 place-items-center rounded bg-accent text-white"
                ><Icon name="plus" size={13} /></span
              >
              <span>Create <b>{r.name}</b> from the current commit</span>
            {:else}
              <Icon
                name={r.kind === 'remote' ? 'cloud' : 'branch'}
                size={15}
                class={r.current ? 'text-accent' : 'text-faint'}
              />
              <span class="truncate {r.current ? 'font-semibold text-accent' : ''}">{r.name}</span>
              {#if r.current}
                <span class="ml-auto text-[11.5px] text-muted">current</span>
              {:else if r.kind === 'remote'}
                <span class="ml-auto text-[11.5px] text-faint">creates a tracking branch</span>
              {/if}
            {/if}
          </div>
        {/each}
        {#if !rows.length}
          <div class="px-2 py-10 text-center text-muted">No branches.</div>
        {/if}
      </div>
      <div
        class="flex items-center gap-3 border-t border-border bg-surface-2 px-4 py-2.5 text-[12px]"
      >
        <label class="flex cursor-pointer items-center gap-2 select-none">
          <input type="checkbox" class="size-3.5 accent-[var(--accent)]" bind:checked={switchTo} />
          Switch to new branches
        </label>
        <span class="ml-auto text-[11.5px] text-faint"
          ><span class="kbd">Enter</span> select · <span class="kbd">Esc</span> close</span
        >
      </div>
    </div>
  </div>
{/if}
