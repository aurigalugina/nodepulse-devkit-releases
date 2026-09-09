<script>
  import { open } from '@tauri-apps/plugin-dialog';
  import { authStore } from '../stores/authStore.svelte.js';
  import { Settings2, FolderSearch, Check } from 'lucide-svelte';

  let { onClose } = $props();

  let path = $state(authStore.vscodiumPath || '');
  let saved = $state(false);

  async function browse() {
    // VSCodium's executable is `codium` (Linux/macOS) or `codium.cmd`/`VSCodium.exe`
    // (Windows) depending on install — no extension filter, since the exact
    // binary name varies by platform and install method (installer vs zip).
    const selected = await open({ multiple: false, directory: false });
    if (selected) path = selected;
  }

  async function save() {
    await authStore.save({ vscodium_path: path || null });
    saved = true;
    setTimeout(() => (saved = false), 1500);
  }
</script>

<div class="h-full flex flex-col p-5 gap-4">
  <div class="flex items-center gap-2">
    <Settings2 size={16} class="text-np-indigo-light" />
    <h1 class="text-sm font-medium text-np-text">Settings</h1>
  </div>

  <div class="flex flex-col gap-2">
    <label class="text-xs text-np-muted" for="vscodium-path">
      VSCodium executable path
    </label>
    <p class="text-[11px] text-np-subtle leading-relaxed">
      Leave empty to use <code class="font-mono">codium</code> from your system PATH
      (default). Only set this if "Open in NodePulse-IDE" fails with
      "program not found" — this usually means VSCodium was installed
      without adding it to PATH.
    </p>
    <div class="flex gap-2">
      <input
        id="vscodium-path"
        type="text"
        bind:value={path}
        placeholder="e.g. C:\Users\you\AppData\Local\Programs\VSCodium\VSCodium.exe"
        class="np-input flex-1 text-xs font-mono"
      />
      <button class="np-btn-ghost text-xs flex items-center gap-1.5" onclick={browse}>
        <FolderSearch size={13} />
        Browse
      </button>
    </div>
  </div>

  <div class="flex items-center gap-2 mt-auto">
    <button class="np-btn-primary text-xs flex items-center gap-1.5" onclick={save}>
      {#if saved}
        <Check size={13} />
        Saved
      {:else}
        Save
      {/if}
    </button>
    <button class="np-btn-ghost text-xs" onclick={onClose}>Close</button>
  </div>
</div>
