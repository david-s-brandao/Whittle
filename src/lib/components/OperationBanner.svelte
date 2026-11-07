<script lang="ts">
  import { plural } from '../format'
  import { changes } from '../stores/changes.svelte'
  import { operations } from '../stores/operations.svelte'
  import { ui } from '../stores/ui.svelte'
  import Icon from './Icon.svelte'

  const op = $derived(changes.status.operation)
  const left = $derived(changes.status.conflicted.length)
  const label = $derived(
    op === 'merge'
      ? 'Merge'
      : op === 'rebase'
        ? 'Rebase'
        : op === 'cherry-pick'
          ? 'Cherry-pick'
          : 'Revert',
  )
</script>

{#if op === 'merge' || op === 'rebase' || op === 'cherry-pick' || op === 'revert'}
  <div class="flex shrink-0 items-center gap-3 border-b border-accent/30 bg-accent/10 px-4 py-2">
    <Icon name="alert" class="shrink-0 text-accent" />
    <div class="min-w-0 flex-1 text-[12.5px]">
      <b>{label} in progress.</b>
      {#if left}
        <span class="text-muted">
          {plural(left, 'file')} still in conflict. Resolve {left === 1 ? 'it' : 'them'} in the Changes
          tab, then continue.
        </span>
      {:else}
        <span class="text-muted">All conflicts resolved. Continue to finish.</span>
      {/if}
    </div>
    {#if left}
      <button class="btn btn-outline h-7" onclick={() => (ui.view = 'changes')}
        >Show conflicts</button
      >
    {/if}
    {#if op === 'rebase' || op === 'cherry-pick'}
      <button
        class="btn btn-outline h-7"
        title="Drop the commit being applied and move on"
        disabled={operations.busy}
        onclick={() => operations.skip()}>Skip commit</button
      >
    {/if}
    <button
      class="btn btn-danger btn-outline h-7"
      disabled={operations.busy}
      onclick={() => operations.abort()}>Abort</button
    >
    <button
      class="btn btn-primary h-7 font-semibold"
      disabled={operations.busy || left > 0}
      title={left ? 'Resolve every conflicted file first' : `Finish the ${op}`}
      onclick={() => operations.continue()}>Continue {op}</button
    >
  </div>
{/if}
