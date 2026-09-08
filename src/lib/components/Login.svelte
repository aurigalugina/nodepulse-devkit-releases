<script>
  import { invoke } from '@tauri-apps/api/core';
  import { authStore } from '../stores/authStore.svelte.js';

  let { onLoggedIn } = $props();

  let url = $state(authStore.url || '');
  let username = $state(authStore.username || '');
  let password = $state('');
  let loading = $state(false);
  let error = $state('');

  async function handleSubmit(e) {
    e.preventDefault();
    if (!url || !username || !password) {
      error = 'Host, username, and password are required.';
      return;
    }
    loading = true;
    error = '';
    try {
      const res = await invoke('login', { host: url, username, password });
      await authStore.setAuth(url, username, res.token, res.expires_at);
      onLoggedIn();
    } catch (e) {
      error = typeof e === 'string' ? e : 'Login failed.';
    } finally {
      loading = false;
    }
  }
</script>

<div class="h-full flex items-center justify-center">
  <form onsubmit={handleSubmit} class="w-full max-w-sm space-y-3 p-6">
    <div class="text-center mb-4">
      <h1 class="text-lg font-medium text-np-text">NodePulse IDE</h1>
      <p class="text-xs text-np-muted mt-1">Sign in once — session lasts 90 days.</p>
    </div>

    <input class="np-input" type="text" placeholder="NodePulse URL (https://...)" bind:value={url} disabled={loading} />
    <input class="np-input" type="text" placeholder="Username" bind:value={username} disabled={loading} />
    <input class="np-input" type="password" placeholder="Password" bind:value={password} disabled={loading} />

    {#if error}
      <p class="text-xs text-np-red">{error}</p>
    {/if}

    <button type="submit" class="np-btn-primary w-full" disabled={loading}>
      {loading ? 'Signing in…' : 'Sign In'}
    </button>
  </form>
</div>
