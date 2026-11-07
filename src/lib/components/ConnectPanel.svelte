<script lang="ts">
  import { api } from '../api'
  import { forge } from '../stores/forge.svelte'
  import Icon from './Icon.svelte'

  let token = $state('')
  let busy = $state(false)
  let error = $state<string | null>(null)

  const s = $derived(forge.status)
  const tokenUrl = $derived(
    `https://${s?.host || 'github.com'}/settings/tokens/new?scopes=repo&description=Whittle`,
  )

  async function connect() {
    if (!token.trim() || busy) return
    busy = true
    error = null
    try {
      await forge.connect(token)
      token = '' // the token now lives only in the system keyring
    } catch (e) {
      error =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : String(e)
    } finally {
      busy = false
    }
  }
</script>

<div class="mx-auto max-w-md space-y-4 p-6 text-[13px]">
  <div class="flex items-center gap-2.5">
    <Icon name="shield" class="text-accent" />
    <div class="text-[15px] font-semibold">Sign in to GitHub</div>
  </div>
  <p class="text-muted">
    {#if s?.message}{s.message}{:else}
      Signing in lets you see private repositories, comment, and review. Public repositories can be
      read without it, but GitHub allows only a few requests per hour.{/if}
  </p>

  <div class="rounded-lg border border-border bg-surface-2 p-3">
    <div class="font-medium">Option 1: use the GitHub CLI</div>
    <p class="mt-1 text-muted">
      If you have <span class="font-mono">gh</span> installed, run
      <span class="font-mono">gh auth login</span> in a terminal, then press Retry. Whittle asks it for
      your login when needed and never copies it.
    </p>
    <button class="btn btn-outline mt-2 h-7" onclick={() => forge.load()}>
      <Icon name="refresh" size={13} /> Retry
    </button>
  </div>

  <div class="rounded-lg border border-border bg-surface-2 p-3">
    <div class="font-medium">Option 2: paste a personal access token</div>
    <p class="mt-1 text-muted">
      It needs the <b class="text-fg">repo</b> permission. Whittle checks it, then keeps it in your system
      keyring (GNOME Keyring or KWallet), never in a file.
    </p>
    <button class="mt-1 text-[12px] text-accent underline" onclick={() => api.openUrl(tokenUrl)}>
      Create a token on {s?.host || 'github.com'}
    </button>
    <div class="mt-2 flex gap-2">
      <input
        type="password"
        class="input"
        placeholder="ghp_… or github_pat_…"
        autocomplete="off"
        spellcheck="false"
        bind:value={token}
        onkeydown={(e) => e.key === 'Enter' && connect()}
      />
      <button class="btn btn-primary h-9" disabled={!token.trim() || busy} onclick={connect}>
        {busy ? 'Checking…' : 'Connect'}
      </button>
    </div>
    {#if error}<p class="mt-2 text-[12px] text-del">{error}</p>{/if}
  </div>
</div>
