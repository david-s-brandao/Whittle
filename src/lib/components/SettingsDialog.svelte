<script lang="ts">
  import { api } from '../api'
  import { contrast } from '../color'
  import { applyAppearance, settings } from '../stores/settings.svelte'
  import { theme, type ThemeChoice } from '../stores/theme.svelte'
  import { ui } from '../stores/ui.svelte'
  import type { Settings } from '../types'
  import Icon from './Icon.svelte'

  let draft = $state<Settings>({ ...settings.value })
  let error = $state<string | null>(null)
  let gitCheck = $state<{ ok: boolean; text: string } | null>(null)
  let saving = $state(false)

  const swatches = [
    ['#f66008', 'Ember (default)'],
    ['#ea580c', 'Burnt orange'],
    ['#f97316', 'Orange'],
    ['#fb923c', 'Soft orange'],
    ['#f59e0b', 'Amber'],
    ['#dc2626', 'Red'],
    ['#2563eb', 'Blue'],
    ['#16a34a', 'Green'],
  ] as const
  const themes: ThemeChoice[] = ['system', 'light', 'dark']

  // opening: start from what is saved
  $effect(() => {
    if (ui.settingsOpen) {
      draft = { ...settings.value }
      error = null
      gitCheck = null
    }
  })
  // live preview of the appearance while editing
  $effect(() => {
    if (ui.settingsOpen) applyAppearance({ font_size: draft.font_size, accent: draft.accent })
  })

  const lowContrast = $derived(contrast(draft.accent, '#ffffff') < 2.5)

  function cancel() {
    applyAppearance(settings.value) // undo the preview
    ui.settingsOpen = false
  }
  async function save() {
    saving = true
    error = null
    try {
      await settings.save(draft)
      ui.settingsOpen = false
    } catch (e) {
      error = String(e)
    } finally {
      saving = false
    }
  }
  async function testGit() {
    try {
      gitCheck = { ok: true, text: await api.gitVersion(draft.git_path) }
    } catch (e) {
      gitCheck = { ok: false, text: String(e) }
    }
  }
  async function browseClone() {
    const d = await api.pickFolder()
    if (d) draft.clone_dir = d
  }
  const text = (v: string | null) => v ?? ''
  const orNull = (v: string) => (v.trim() ? v : null)
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') cancel()
  }
</script>

{#if ui.settingsOpen}
  <div
    class="backdrop pt-[6vh]"
    role="presentation"
    onmousedown={(e) => e.target === e.currentTarget && cancel()}
  >
    <div
      class="modal flex max-h-[86vh] w-[620px] flex-col"
      role="dialog"
      aria-modal="true"
      aria-label="Settings"
      tabindex="-1"
      {onkeydown}
    >
      <div class="flex items-center gap-2.5 border-b border-border px-5 py-3.5">
        <Icon name="settings" class="text-accent" />
        <div class="text-[15px] font-semibold">Settings</div>
      </div>

      <div class="space-y-6 overflow-y-auto px-5 py-5">
        <section class="space-y-4">
          <div class="section-label">Appearance</div>
          <div class="flex items-center gap-4">
            <span class="w-28 text-[12.5px] text-muted">Theme</span>
            <div class="flex rounded-md border border-border p-0.5 text-[12px] font-medium">
              {#each themes as t (t)}
                <button
                  class="rounded px-3 py-1 {theme.choice === t
                    ? 'bg-accent text-white'
                    : 'text-muted hover:text-fg'}"
                  onclick={() => theme.set(t)}
                  >{t === 'system' ? 'Follow system' : t[0].toUpperCase() + t.slice(1)}</button
                >
              {/each}
            </div>
          </div>
          <div class="flex items-center gap-4">
            <label class="w-28 text-[12.5px] text-muted" for="font-size">Font size</label>
            <input
              id="font-size"
              type="range"
              min="11"
              max="17"
              step="1"
              class="w-48 accent-[var(--accent)]"
              bind:value={draft.font_size}
            />
            <span class="w-12 text-[12.5px] tabular-nums">{draft.font_size} px</span>
          </div>
          <div class="flex items-start gap-4">
            <span class="w-28 pt-1 text-[12.5px] text-muted">Accent colour</span>
            <div class="space-y-2">
              <div class="flex flex-wrap items-center gap-2">
                {#each swatches as [hex, name] (hex)}
                  <button
                    class="size-6 rounded-full border-2 {draft.accent === hex
                      ? 'border-fg'
                      : 'border-transparent'}"
                    style="background: {hex}"
                    title={name}
                    aria-label={name}
                    onclick={() => (draft.accent = hex)}
                  ></button>
                {/each}
                <input
                  type="color"
                  class="size-6 cursor-pointer rounded border border-border bg-transparent p-0"
                  value={draft.accent}
                  oninput={(e) => (draft.accent = e.currentTarget.value)}
                  title="Pick any colour"
                />
                <span class="font-mono text-[11.5px] text-muted">{draft.accent}</span>
              </div>
              {#if lowContrast}
                <p class="text-[11.5px] text-del">
                  White text on this colour is hard to read. A darker shade works better for
                  buttons.
                </p>
              {/if}
            </div>
          </div>
        </section>

        <section class="space-y-4">
          <div class="section-label">Git</div>
          <div>
            <label class="label" for="git-path">Git executable</label>
            <div class="flex gap-2">
              <input
                id="git-path"
                class="input"
                placeholder="Leave empty to use git from your PATH"
                value={text(draft.git_path)}
                oninput={(e) => (draft.git_path = orNull(e.currentTarget.value))}
                spellcheck="false"
              />
              <button class="btn btn-outline h-9" onclick={testGit}>Test</button>
            </div>
            {#if gitCheck}
              <p class="mt-1.5 text-[12px] {gitCheck.ok ? 'text-add' : 'text-del'}">
                {gitCheck.ok ? '✓ ' : ''}{gitCheck.text}
              </p>
            {/if}
          </div>
          <div>
            <label class="label" for="clone-dir">Default folder for clones</label>
            <div class="flex gap-2">
              <input
                id="clone-dir"
                class="input"
                placeholder="~/projects (or your home folder)"
                value={text(draft.clone_dir)}
                oninput={(e) => (draft.clone_dir = orNull(e.currentTarget.value))}
                spellcheck="false"
              />
              <button class="btn btn-outline h-9" onclick={browseClone}>Browse…</button>
            </div>
          </div>
        </section>

        <section class="space-y-4">
          <div class="section-label">Tools</div>
          <div>
            <label class="label" for="editor-cmd">Editor command</label>
            <input
              id="editor-cmd"
              class="input"
              placeholder="Auto-detect (code, codium, zed, subl, gedit…)"
              value={text(draft.editor_cmd)}
              oninput={(e) => (draft.editor_cmd = orNull(e.currentTarget.value))}
              spellcheck="false"
            />
            <p class="mt-1 text-[11.5px] text-faint">
              The repository folder is added as the last argument, e.g. <span class="font-mono"
                >code --new-window</span
              >.
            </p>
          </div>
          <div>
            <label class="label" for="term-cmd">Terminal command</label>
            <input
              id="term-cmd"
              class="input"
              placeholder="Auto-detect (gnome-terminal, konsole, xfce4-terminal…)"
              value={text(draft.terminal_cmd)}
              oninput={(e) => (draft.terminal_cmd = orNull(e.currentTarget.value))}
              spellcheck="false"
            />
          </div>
        </section>

        {#if error}
          <div class="rounded-lg border border-del/40 bg-del/10 p-3 text-[12.5px] text-del">
            {error}
          </div>
        {/if}
      </div>

      <div class="flex justify-end gap-2 border-t border-border bg-surface-2 px-5 py-3">
        <button class="btn btn-outline h-8" onclick={cancel}>Cancel</button>
        <button class="btn btn-primary h-8 px-4 font-semibold" disabled={saving} onclick={save}
          >{saving ? 'Saving…' : 'Save'}</button
        >
      </div>
    </div>
  </div>
{/if}
