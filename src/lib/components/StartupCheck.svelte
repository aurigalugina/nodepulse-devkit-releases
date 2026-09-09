<script>
  import { onMount } from 'svelte';
  import { RefreshCw, CheckCircle2, GitBranch } from 'lucide-svelte';

  let { onDone = () => {} } = $props();

  /** @type {'checking'|'up-to-date'|'update-available'|'offline'} */
  let status     = $state('checking');
  let updateInfo = $state(null);
  let installing = $state(false);
  let progress   = $state(0);
  let installError = $state('');

  const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  onMount(async () => {
    if (!isTauri()) { onDone(); return; }
    try {
      const { check } = await import(/* @vite-ignore */ '@tauri-apps/plugin-updater');
      const result = await check();
      // Guard: strip 'v' prefix before comparing — mismatch between manifest "v0.x.y"
      // and binary "0.x.y" would cause check() to always return available.
      const norm = (v) => v?.replace(/^v/, '') ?? '';
      const isRealUpdate = result?.available && norm(result.version) !== norm(result.currentVersion);
      if (isRealUpdate) {
        status = 'update-available';
        updateInfo = result;
      } else {
        status = 'up-to-date';
        setTimeout(onDone, 900);
      }
    } catch {
      status = 'offline';
      setTimeout(onDone, 600);
    }
  });

  async function installUpdate() {
    if (!updateInfo) return;
    installing = true;
    installError = '';
    let downloaded = 0;
    let total = 0;
    try {
      const { relaunch } = await import(/* @vite-ignore */ '@tauri-apps/plugin-process');
      await updateInfo.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          total = event.data.contentLength ?? 0;
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          progress = total > 0 ? Math.round((downloaded / total) * 100) : 0;
        } else if (event.event === 'Finished') {
          progress = 100;
        }
      });
      await relaunch();
    } catch (e) {
      installing = false;
      installError = typeof e === 'string' ? e : 'Update install failed — try downloading manually from Settings → NodePulse IDE.';
    }
  }
</script>

<div class="flex flex-col h-full items-center justify-center px-6 gap-6">
  <!-- Branding -->
  <div class="flex flex-col items-center gap-2.5">
    <div class="w-16 h-16 rounded-2xl flex items-center justify-center"
         style="background: var(--color-np-indigo-dim); border: 1px solid var(--color-np-indigo-ring);">
      <GitBranch size={26} class="text-np-indigo-light" />
    </div>
    <div class="text-center">
      <p class="text-np-text font-semibold text-sm leading-tight">NodePulse IDE</p>
      <p class="text-np-muted text-xs leading-tight mt-0.5">Open in local VSCodium</p>
    </div>
  </div>

  <!-- Status indicator -->
  <div class="flex flex-col items-center gap-3 w-full max-w-sm">
    {#if status === 'checking'}
      <div class="flex items-center gap-2 text-xs text-np-muted">
        <RefreshCw size={13} class="animate-spin" />
        Checking for updates…
      </div>

    {:else if status === 'up-to-date'}
      <div class="flex items-center gap-2 text-xs text-np-green">
        <CheckCircle2 size={14} />
        App is up to date
      </div>

    {:else if status === 'offline'}
      <p class="text-xs text-np-subtle">Update check skipped (offline or no release published)</p>

    {:else if status === 'update-available'}
      <div class="flex flex-col gap-3 w-full">
        <div class="rounded-lg px-3 py-3 flex items-start gap-3"
             style="background: var(--color-np-indigo-dim); border: 1px solid var(--color-np-indigo-ring);">
          <div class="mt-1 h-2 w-2 shrink-0 rounded-full bg-np-indigo animate-pulse"></div>
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-np-text">Update v{updateInfo.version} available</p>
            {#if updateInfo.body}
              <p class="text-xs text-np-muted mt-0.5 leading-relaxed">{updateInfo.body}</p>
            {/if}
          </div>
        </div>

        {#if installError}
          <p class="text-xs text-np-red">{installError}</p>
        {/if}

        {#if installing}
          <div class="flex flex-col gap-1.5 px-1">
            <div class="h-1.5 w-full rounded-full overflow-hidden"
                 style="background: var(--color-np-elevated);">
              <div class="h-full rounded-full bg-np-indigo transition-all duration-200"
                   style="width: {progress}%"></div>
            </div>
            <p class="text-xs text-np-muted text-center">
              {progress < 100 ? `Downloading… ${progress}%` : 'Installing…'}
            </p>
          </div>
        {:else}
          <div class="flex gap-2">
            <button onclick={() => onDone()} class="np-btn-ghost flex-1 text-xs">
              Later
            </button>
            <button onclick={installUpdate} class="np-btn-primary flex-1 text-xs">
              Update Now
            </button>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
