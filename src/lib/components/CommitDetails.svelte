<script lang="ts">
  import { formatDate, splitPath, statusBadge } from '../format'
  import { repo } from '../stores/repo.svelte'
  import { ui } from '../stores/ui.svelte'
  import DiffView from './DiffView.svelte'
  import Icon from './Icon.svelte'

  let copied = $state(false)
  async function copyHash() {
    if (!repo.selected) return
    try {
      await navigator.clipboard.writeText(repo.selected.id)
      copied = true
      setTimeout(() => (copied = false), 1500)
    } catch {
      // clipboard unavailable: nothing to do
    }
  }
</script>

<div class="flex h-full min-h-0 flex-col bg-surface">
  {#if !repo.selected}
    <div class="grid flex-1 place-items-center text-muted">
      Select a commit to see what changed.
    </div>
  {:else}
    <div class="flex items-center gap-3 border-b border-border px-4 py-2.5">
      <div class="min-w-0 flex-1">
        <div class="selectable truncate text-[13.5px] font-semibold">{repo.selected.summary}</div>
        <div class="mt-0.5 flex items-center gap-2 text-[12px] text-muted">
          <span>{repo.selected.author} · {formatDate(repo.selected.time)}</span>
          {#if repo.selected.signed}
            {@const sig = repo.signature}
            <span
              class="inline-flex items-center gap-1 rounded-md px-1.5 py-px text-[11px] font-medium {sig?.status ===
              'good'
                ? 'bg-add/15 text-add'
                : sig?.status === 'bad' || sig?.status === 'revoked'
                  ? 'bg-del/15 text-del'
                  : 'bg-hover text-muted'}"
              title={sig
                ? `${sig.status === 'good' ? 'Verified signature' : sig.status === 'good_untrusted' ? 'Valid signature, signer not trusted' : sig.status === 'unverifiable' ? 'Signed, but the key is not available to verify it' : sig.status === 'bad' ? 'The signature does not match the commit' : sig.status === 'expired' ? 'Valid signature from an expired key' : 'Signature problem'}${sig.signer ? `\nSigner: ${sig.signer}` : ''}${sig.key ? `\nKey: ${sig.key}` : ''}`
                : 'Checking the signature…'}
            >
              <Icon name={sig?.status === 'good' ? 'shield-check' : 'shield'} size={12} />
              {#if !sig}Signed{:else if sig.status === 'good'}Verified{sig.signer
                  ? ` · ${sig.signer}`
                  : ''}{:else if sig.status === 'bad'}Bad signature{:else if sig.status === 'revoked'}Revoked
                key{:else if sig.status === 'unverifiable'}Signed · can't verify{:else if sig.status === 'good_untrusted'}Signed
                · untrusted{:else if sig.status === 'expired'}Signed · expired key{:else}Signed{/if}
            </span>
          {/if}
        </div>
      </div>
      <button
        class="btn btn-outline h-7 gap-1.5 font-mono text-[11.5px]"
        title="Copy full commit hash"
        onclick={copyHash}
      >
        <Icon
          name={copied ? 'check' : 'copy'}
          size={13}
          class={copied ? 'text-add' : 'text-muted'}
        />
        {repo.selected.short_id}
      </button>
    </div>
    <div class="flex min-h-0 flex-1">
      <div class="flex w-72 shrink-0 flex-col border-r border-border bg-surface-2">
        <div class="section-label px-4 py-2">
          {repo.files.length} changed file{repo.files.length === 1 ? '' : 's'}
        </div>
        <ul class="min-h-0 flex-1 overflow-y-auto pb-2">
          {#each repo.files as f (f.path)}
            {@const [letter, color, label] = statusBadge(f.status)}
            {@const [dir, name] = splitPath(f.path)}
            <li
              class="group flex h-7 items-center pr-2 {repo.selectedFile === f.path
                ? 'bg-accent/10'
                : 'hover:bg-hover'}"
            >
              <button
                class="flex h-7 min-w-0 flex-1 items-center gap-2 pl-4 text-left text-[12.5px]"
                title={f.old_path ? `${f.old_path} → ${f.path}` : f.path}
                onclick={() => repo.selectFile(f.path)}
              >
                <span class="w-3 shrink-0 font-mono text-[11px] font-bold {color}" title={label}
                  >{letter}</span
                >
                <span class="truncate"
                  ><span class="text-faint">{dir}</span><span class="font-medium">{name}</span
                  ></span
                >
              </button>
              {#if f.status !== 'deleted'}
                <span class="hidden shrink-0 items-center group-hover:flex">
                  <button
                    class="icon-btn icon-btn-sm"
                    title="Who changed each line (blame)"
                    onclick={() =>
                      (ui.fileView = {
                        path: f.path,
                        rev: repo.selected?.id ?? null,
                        tab: 'blame',
                      })}><Icon name="blame" size={13} /></button
                  >
                  <button
                    class="icon-btn icon-btn-sm"
                    title="History of this file"
                    onclick={() =>
                      (ui.fileView = {
                        path: f.path,
                        rev: repo.selected?.id ?? null,
                        tab: 'history',
                      })}><Icon name="history" size={13} /></button
                  >
                </span>
              {/if}
            </li>
          {/each}
          {#if !repo.files.length && !repo.diffError}
            <li class="px-4 py-1 text-[12px] text-faint">No file changes.</li>
          {/if}
        </ul>
      </div>
      <div class="min-w-0 flex-1 overflow-auto">
        {#if repo.diffError}
          <div class="p-4 text-del">{repo.diffError}</div>
        {:else if repo.diff}
          <DiffView diff={repo.diff} />
        {/if}
      </div>
    </div>
  {/if}
</div>
