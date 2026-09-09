<script>
  import { Copy, Check, AlertCircle } from 'lucide-svelte';

  /** @type {{ title?: string, message: string, context?: Record<string,string> }} */
  let { title = 'Something went wrong', message, context = {} } = $props();

  let copied = $state(false);

  /** Builds a single copy-pasteable block: title, context (e.g. which step
   * failed, node/folder involved), and the raw error text — this is what
   * gets pasted back when reporting a bug, so it needs to be self-contained
   * (no "see screenshot" needed) and unambiguous about which command/step
   * produced it. */
  function buildReportText() {
    const lines = [`NodePulse IDE — ${title}`];
    for (const [key, value] of Object.entries(context)) {
      if (value) lines.push(`${key}: ${value}`);
    }
    lines.push('', message);
    return lines.join('\n');
  }

  async function copyReport() {
    try {
      const { writeText } = await import(/* @vite-ignore */ '@tauri-apps/plugin-clipboard-manager');
      await writeText(buildReportText());
    } catch {
      // Fallback for non-Tauri/dev contexts — plugin-clipboard-manager
      // should always be available in the built app, but degrade gracefully
      // rather than throwing if it somehow isn't.
      try {
        await navigator.clipboard.writeText(buildReportText());
      } catch {
        return;
      }
    }
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<div class="rounded-lg border border-np-red/30 bg-np-red-dim p-3 text-left w-full">
  <div class="flex items-start gap-2">
    <AlertCircle size={14} class="text-np-red flex-shrink-0 mt-0.5" />
    <div class="flex-1 min-w-0">
      <p class="text-xs font-medium text-np-red">{title}</p>
      {#each Object.entries(context) as [key, value]}
        {#if value}
          <p class="text-[11px] text-np-muted mt-1">
            <span class="text-np-subtle">{key}:</span> <span class="font-mono">{value}</span>
          </p>
        {/if}
      {/each}
      <pre class="text-[11px] font-mono text-np-text mt-2 whitespace-pre-wrap break-all bg-np-bg/40 rounded p-2 max-h-40 overflow-y-auto">{message}</pre>
    </div>
    <button
      onclick={copyReport}
      class="flex-shrink-0 flex items-center gap-1 rounded-md border border-np-border px-2 py-1 text-[11px] text-np-muted hover:text-np-text hover:border-np-subtle transition-colors"
      title="Copy full error report to clipboard"
    >
      {#if copied}
        <Check size={11} class="text-np-green" />
        Copied
      {:else}
        <Copy size={11} />
        Copy
      {/if}
    </button>
  </div>
</div>
