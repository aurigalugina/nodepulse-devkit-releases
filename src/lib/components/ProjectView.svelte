<script>
  import { invoke } from '@tauri-apps/api/core';
  import { Upload, Download, FolderCode, Loader, Settings2 } from 'lucide-svelte';
  import ErrorPanel from './ErrorPanel.svelte';

  /** @type {{ localPath: string, folder: string, nodeId: string, launchError?: string }} */
  let { project, vscodiumPath, onOpenSettings } = $props();

  let statusLines = $state([]); // parsed porcelain lines, [{status, file}]
  let loadingStatus = $state(false);
  let showConfirm = $state(false);
  let pushing = $state(false);
  let pulling = $state(false);
  let resultMessage = $state('');
  let resultIsError = $state(false);
  let resultStep = $state(''); // which action ('push' | 'pull' | 'launch') produced resultMessage — feeds ErrorPanel's context

  // Surface a non-fatal VSCodium launch failure from OpenFolder immediately —
  // the clone still succeeded, so the user lands here rather than an error
  // screen, but the failure shouldn't be silently dropped either.
  $effect(() => {
    if (project.launchError) {
      resultMessage = project.launchError;
      resultIsError = true;
      resultStep = 'launch_vscodium';
    }
  });

  function parsePorcelain(raw) {
    return raw
      .split('\n')
      .map((l) => l.trimEnd())
      .filter(Boolean)
      .map((l) => ({ status: l.slice(0, 2).trim() || '?', file: l.slice(3) }));
  }

  async function refreshStatus() {
    loadingStatus = true;
    try {
      const raw = await invoke('git_status_porcelain', { localPath: project.localPath });
      statusLines = parsePorcelain(raw);
    } catch (e) {
      statusLines = [];
    } finally {
      loadingStatus = false;
    }
  }

  async function openPushPreview() {
    await refreshStatus();
    showConfirm = true;
  }

  async function confirmPush() {
    pushing = true;
    resultMessage = '';
    try {
      const out = await invoke('git_commit_and_push', { localPath: project.localPath, commitMessage: null });
      resultMessage = out || 'Pushed successfully.';
      resultIsError = false;
      showConfirm = false;
      statusLines = [];
    } catch (e) {
      // Non-fast-forward rejection (or any other push error) lands here —
      // surface git's own message and offer Pull as the recovery action,
      // per the design decision to lean on git's native conflict guard
      // rather than any custom conflict handling.
      resultMessage = typeof e === 'string' ? e : 'Push failed.';
      resultIsError = true;
      resultStep = 'git_commit_and_push';
    } finally {
      pushing = false;
    }
  }

  async function pullLatest() {
    pulling = true;
    try {
      const out = await invoke('git_pull', { localPath: project.localPath });
      resultMessage = out || 'Pulled successfully — try pushing again.';
      resultIsError = false;
    } catch (e) {
      resultMessage = typeof e === 'string' ? e : 'Pull failed.';
      resultIsError = true;
      resultStep = 'git_pull';
    } finally {
      pulling = false;
    }
  }

  async function openInVSCodium() {
    try {
      await invoke('launch_vscodium', { localPath: project.localPath, vscodiumPath: vscodiumPath || null });
    } catch (e) {
      resultMessage = typeof e === 'string' ? e : 'Could not launch VSCodium.';
      resultIsError = true;
      resultStep = 'launch_vscodium';
    }
  }
</script>

<div class="h-full flex flex-col p-6 gap-4">
  <div>
    <h1 class="text-sm font-medium text-np-text">Project ready</h1>
    <p class="text-xs text-np-muted mt-1 font-mono break-all">{project.localPath}</p>
  </div>

  <!-- "Open in VSCodium" is the primary, visually dominant action — the
       beginner-friendly Push button below stays secondary, since the goal
       is nudging users toward VSCodium's own Source Control panel over
       time, not replacing it long-term (per the design discussion). -->
  <button class="np-btn-primary" onclick={openInVSCodium}>
    <span class="inline-flex items-center gap-2"><FolderCode size={14} /> Open in VSCodium</span>
  </button>

  <div class="border-t border-np-border pt-4">
    <p class="text-xs text-np-muted mb-2">
      Not comfortable with git yet? Use this instead of VSCodium's Source Control panel:
    </p>
    <div class="flex gap-2">
      <button class="np-btn-ghost flex-1" onclick={openPushPreview} disabled={loadingStatus}>
        <span class="inline-flex items-center gap-2 justify-center">
          {#if loadingStatus}<Loader size={13} class="animate-spin" />{:else}<Upload size={13} />{/if}
          Push to Server
        </span>
      </button>
      <button class="np-btn-ghost flex-1" onclick={pullLatest} disabled={pulling}>
        <span class="inline-flex items-center gap-2 justify-center">
          {#if pulling}<Loader size={13} class="animate-spin" />{:else}<Download size={13} />{/if}
          Pull Latest
        </span>
      </button>
    </div>
  </div>

  {#if resultMessage}
    {#if resultIsError}
      <ErrorPanel
        title="Action failed"
        message={resultMessage}
        context={{
          Step: resultStep,
          Project: project.localPath,
          Node: project.nodeId
        }}
      />
      {#if resultStep === 'launch_vscodium'}
        <button class="np-btn-ghost mt-2 text-xs self-start flex items-center gap-1.5" onclick={onOpenSettings}>
          <Settings2 size={12} />
          Set VSCodium path in Settings
        </button>
      {:else}
        <button class="np-btn-ghost mt-2 text-xs self-start" onclick={pullLatest} disabled={pulling}>
          Pull latest &amp; retry
        </button>
      {/if}
    {:else}
      <div class="text-xs p-3 rounded-lg bg-np-green-dim text-np-green">
        <p class="whitespace-pre-wrap font-mono">{resultMessage}</p>
      </div>
    {/if}
  {/if}
</div>

{#if showConfirm}
  <div
    class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
    onclick={() => (showConfirm = false)}
    onkeydown={(e) => e.key === 'Escape' && (showConfirm = false)}
    role="presentation"
  >
    <div
      class="bg-np-surface border border-np-border rounded-xl p-5 w-full max-w-sm"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <h2 class="text-sm font-medium text-np-text mb-3">
        {statusLines.length === 0 ? 'Nothing to push' : `Push ${statusLines.length} change${statusLines.length === 1 ? '' : 's'}?`}
      </h2>
      {#if statusLines.length > 0}
        <ul class="text-xs font-mono text-np-muted max-h-48 overflow-y-auto space-y-1 mb-4">
          {#each statusLines as line}
            <li><span class="text-np-indigo-light">{line.status}</span> {line.file}</li>
          {/each}
        </ul>
      {/if}
      <div class="flex gap-2 justify-end">
        <button class="np-btn-ghost" onclick={() => (showConfirm = false)}>Cancel</button>
        {#if statusLines.length > 0}
          <button class="np-btn-primary" onclick={confirmPush} disabled={pushing}>
            {pushing ? 'Pushing…' : 'Confirm Push'}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}
