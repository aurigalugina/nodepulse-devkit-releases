<script>
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { authStore } from '../stores/authStore.svelte.js';
  import ErrorPanel from './ErrorPanel.svelte';

  /** @type {{ nodeId: string, path: string } | null} */
  let { pending, onDone } = $props();

  let step = $state('idle'); // idle | picking | cloning | launching | done | error
  let error = $state('');
  let failedStep = $state(''); // which step ('clone' | 'launch') produced the error — shown in the copy-able report
  let localPath = $state('');

  async function startOpenFlow() {
    if (!pending) return;
    step = 'picking';
    error = '';
    const chosen = await open({
      directory: true,
      multiple: false,
      title: 'Choose where to save this project locally',
    });
    if (!chosen) {
      step = 'idle';
      return;
    }
    // Nest under a subfolder named after the remote folder's basename so
    // repeatedly opening different NodePulse folders into the same parent
    // directory doesn't collide or require the user to type a name.
    const baseName = pending.path.split('/').filter(Boolean).pop() || 'project';
    localPath = `${chosen}/${baseName}`;

    step = 'cloning';
    try {
      await invoke('git_clone', {
        host: authStore.url,
        nodeId: pending.nodeId,
        folder: pending.path,
        localPath,
        jwtToken: authStore.token,
        displayName: authStore.username,
        email: `${authStore.username}@nodepulse.local`, // best-effort — NodePulse users don't all have a real email on file
      });
    } catch (e) {
      error = typeof e === 'string' ? e : 'Clone failed.';
      failedStep = 'git_clone';
      step = 'error';
      return;
    }

    step = 'launching';
    try {
      await invoke('launch_vscodium', {
        localPath,
        vscodiumPath: authStore.vscodiumPath || null,
      });
    } catch (e) {
      // Clone succeeded even if VSCodium launch failed (e.g. not
      // installed/not on PATH) — surface the error but still let the user
      // proceed to the project view, where "Open in VSCodium" can retry.
      error = typeof e === 'string' ? e : 'Could not launch VSCodium.';
      failedStep = 'launch_vscodium';
    }

    step = 'done';
    onDone({
      localPath,
      folder: pending.path,
      nodeId: pending.nodeId,
      // Non-fatal launch error (if any) rides along so ProjectView can
      // still show it — the clone itself succeeded, so we don't want to
      // block the user on the error screen, but we also don't want to
      // silently swallow a "VSCodium not found" failure.
      launchError: failedStep === 'launch_vscodium' ? error : '',
    });
  }

  $effect(() => {
    if (pending && step === 'idle') {
      startOpenFlow();
    }
  });
</script>

<div class="h-full flex items-center justify-center px-6">
  <div class="text-center {step === 'error' ? 'max-w-lg w-full' : 'max-w-sm'}">
    {#if step === 'picking'}
      <p class="text-sm text-np-muted">Choose a local folder…</p>
    {:else if step === 'cloning'}
      <p class="text-sm text-np-muted">Cloning project from NodePulse…</p>
    {:else if step === 'launching'}
      <p class="text-sm text-np-muted">Opening in VSCodium…</p>
    {:else if step === 'error'}
      <ErrorPanel
        title="Couldn't open this project"
        message={error}
        context={{
          Step: failedStep,
          Node: pending?.nodeId ?? '',
          Folder: pending?.path ?? '',
          'NodePulse host': authStore.url ?? ''
        }}
      />
      <button class="np-btn-ghost mt-3" onclick={() => (step = 'idle')}>Try again</button>
    {/if}
  </div>
</div>
