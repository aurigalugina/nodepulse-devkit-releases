<script>
  import { onMount } from 'svelte';
  import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';
  import { authStore } from './lib/stores/authStore.svelte.js';
  import Login from './lib/components/Login.svelte';
  import OpenFolder from './lib/components/OpenFolder.svelte';
  import ProjectView from './lib/components/ProjectView.svelte';

  let ready = $state(false);
  let pendingOpen = $state(null); // { nodeId, path } parsed from a deep-link, queued until login completes
  let opened = $state(null); // { localPath, folder, nodeId } once OpenFolder finishes — Task 9 builds the real project view here

  /** Parses "nodepulse-ide://open?node=<id>&path=<folder>&host=<origin>" —
   * see NodeFileManager.svelte's openInNodePulseIDE() on the web-panel side
   * for the URL this is built to match. */
  function parseDeepLink(urlStr) {
    try {
      const url = new URL(urlStr);
      const nodeId = url.searchParams.get('node');
      const path = url.searchParams.get('path');
      const host = url.searchParams.get('host');
      if (!nodeId || !path) return null;
      return { nodeId, path, host };
    } catch {
      return null;
    }
  }

  function handleDeepLink(urlStr) {
    const parsed = parseDeepLink(urlStr);
    if (!parsed) return;
    // If devkit is pointed at a different host than the one that sent this
    // link, prefer the link's host — most likely scenario is the user only
    // ever uses one NodePulse instance, but this keeps multi-instance setups
    // from silently cloning against the wrong server.
    if (parsed.host && parsed.host !== authStore.url) {
      authStore.save({ nodepulse_url: parsed.host });
    }
    // Queue it regardless of auth state — if not logged in yet, this stays
    // queued until Login's onLoggedIn fires and authStore.isAuthenticated
    // flips true, at which point the {#if pendingOpen} branch below takes over.
    pendingOpen = { nodeId: parsed.nodeId, path: parsed.path };
  }

  onMount(async () => {
    await authStore.load();
    ready = true;

    // Cold-start case: the OS launched devkit BECAUSE of this deep link —
    // getCurrent() asks the Rust side (which registered the URL before the
    // frontend even existed) what that launch URL was. This is the fix for
    // the bug where clicking "Open in NodePulse-IDE" while devkit wasn't
    // already running would open the app but land on the plain "Signed in
    // as ..." screen — the previous implementation relied on a Rust-side
    // event emitted from .setup() (before any frontend listener could
    // possibly be registered yet), so the cold-start URL was silently lost.
    try {
      const urls = await getCurrent();
      if (urls && urls.length > 0) handleDeepLink(urls[0]);
    } catch (e) {
      console.warn('getCurrent() deep-link check failed:', e);
    }

    // Warm case: devkit is already running and receives another deep link
    // (e.g. user clicks "Open in NodePulse-IDE" again for a different folder).
    await onOpenUrl((urls) => {
      if (urls && urls.length > 0) handleDeepLink(urls[0]);
    });
  });

  function onLoggedIn() {
    // no-op beyond authStore already being updated — pendingOpen (if any)
    // naturally becomes actionable now that authStore.isAuthenticated is true
  }

  function onOpenDone(result) {
    opened = result;
    pendingOpen = null;
  }
</script>

<main class="h-full">
  {#if !ready}
    <div class="h-full flex items-center justify-center text-np-muted text-sm">Loading…</div>
  {:else if !authStore.isAuthenticated}
    <Login onLoggedIn={onLoggedIn} />
  {:else if pendingOpen}
    <OpenFolder pending={pendingOpen} onDone={onOpenDone} />
  {:else if opened}
    <ProjectView project={opened} vscodiumPath={authStore.vscodiumPath} />
  {:else}
    <div class="h-full flex items-center justify-center text-center px-6">
      <div>
        <h1 class="text-sm font-medium text-np-text">NodePulse IDE</h1>
        <p class="text-xs text-np-muted mt-1">
          Signed in as {authStore.username}. Click "Open in NodePulse-IDE" on a folder
          in NodePulse's file manager to get started.
        </p>
      </div>
    </div>
  {/if}
</main>
