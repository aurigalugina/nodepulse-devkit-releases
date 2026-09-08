import { invoke } from '@tauri-apps/api/core';

/** @type {{ nodepulse_url: string|null, username: string|null, auth_token: string|null, token_expires_at: string|null, vscodium_path: string|null }} */
let _config = $state({
  nodepulse_url: null,
  username: null,
  auth_token: null,
  token_expires_at: null,
  vscodium_path: null,
});

/** @type {boolean} */
let _loaded = $state(false);

export const authStore = {
  get config() { return _config; },
  get loaded() { return _loaded; },
  get token() { return _config.auth_token; },
  get url() { return _config.nodepulse_url; },
  get username() { return _config.username; },
  get vscodiumPath() { return _config.vscodium_path; },

  /** True if we have a token that hasn't passed its expiry — does not
   * re-validate against the server, just checks the locally-stored expiry
   * timestamp devkit received at login. A 90-day token means this is
   * almost always true once logged in; still checked so an expired token
   * shows the login screen again instead of silently failing every git
   * operation with a 401. */
  get isAuthenticated() {
    if (!_config.auth_token || !_config.token_expires_at) return false;
    return new Date(_config.token_expires_at).getTime() > Date.now();
  },

  /** Load persisted config from disk. Call once at app startup. */
  async load() {
    try {
      const saved = await invoke('read_config');
      _config = { ..._config, ...saved };
    } catch (e) {
      console.warn('Failed to load config:', e);
    }
    _loaded = true;
  },

  /** Persist a partial update to disk. */
  async save(patch) {
    _config = { ..._config, ...patch };
    await invoke('write_config', { config: _config });
  },

  /** Store credentials after successful login. Username/url are kept even
   * after logout so re-login only needs a password (per "user password
   * juga tersimpan, terset (kalau bisa ya sekali login)" requirement — the
   * URL/username persisting is what makes that convenient, not full
   * password storage). */
  async setAuth(url, username, token, expiresAt) {
    await authStore.save({
      nodepulse_url: url,
      username,
      auth_token: token,
      token_expires_at: expiresAt,
    });
  },

  /** Clear token on logout (keep url + username for re-login convenience). */
  async clearAuth() {
    await invoke('clear_auth_token');
    _config = { ..._config, auth_token: null, token_expires_at: null };
  },
};
