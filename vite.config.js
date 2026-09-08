import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath, URL } from 'url';

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
    },
  },
  // Exclude lucide-svelte from dep optimization — it ships Svelte 4 syntax ($$props)
  // which conflicts with the global runes:true compiler setting
  optimizeDeps: {
    exclude: ['lucide-svelte'],
  },
  // Tauri: prevent vite from obscuring Rust errors
  clearScreen: false,
  server: {
    port: 1421, // distinct from nodepulse-connect's 1420 — both may run locally at once
    strictPort: true,
    host: true,
    watch: {
      ignored: ['**/src-tauri/**'],
      usePolling: true,
    },
  },
});
