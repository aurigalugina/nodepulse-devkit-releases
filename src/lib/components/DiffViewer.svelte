<script>
  import { onMount, onDestroy } from 'svelte';

  // Side-by-side (VSCode-style) diff via CodeMirror 6's official
  // @codemirror/merge package — identical implementation to web-panel's
  // GitDiffViewer.svelte (same package, same data shape), so behavior
  // stays consistent between the two apps rather than reinventing a
  // second diff renderer.
  let { before = '', after = '', beforeExisted = true, afterExisted = true, filename = '' } = $props();

  let editorEl = $state(null);
  let view;

  function languageFor(name) {
    const ext = name.split('.').pop()?.toLowerCase();
    return ext;
  }

  async function build() {
    if (!editorEl) return;
    view?.destroy();
    view = null;

    const [{ MergeView }, { EditorView, lineNumbers }, { oneDark }, langData] = await Promise.all([
      import('@codemirror/merge'),
      import('@codemirror/view'),
      import('@codemirror/theme-one-dark'),
      import('@codemirror/language-data'),
    ]);

    const ext = languageFor(filename);
    let langExt = [];
    if (ext) {
      const found = langData.languages.find((l) => l.extensions.includes(ext));
      if (found) {
        try {
          langExt = [await found.load()];
        } catch { /* fall back to plain text */ }
      }
    }

    view = new MergeView({
      a: {
        doc: beforeExisted ? before : '',
        extensions: [oneDark, EditorView.editable.of(false), EditorView.lineWrapping, lineNumbers(), ...langExt],
      },
      b: {
        doc: afterExisted ? after : '',
        extensions: [oneDark, EditorView.editable.of(false), EditorView.lineWrapping, lineNumbers(), ...langExt],
      },
      parent: editorEl,
      highlightChanges: true,
      gutter: true,
      collapseUnchanged: { margin: 3, minSize: 6 },
    });
  }

  onMount(build);
  onDestroy(() => view?.destroy());

  $effect(() => {
    before; after; filename; // track
    build();
  });
</script>

<div class="h-full w-full flex flex-col">
  <div class="grid grid-cols-2 text-[11px] text-np-muted px-2 py-1 border-b border-np-border flex-shrink-0">
    <span>{beforeExisted ? 'Before (last commit)' : '(new file)'}</span>
    <span>{afterExisted ? 'After (your changes)' : '(deleted)'}</span>
  </div>
  <div bind:this={editorEl} class="flex-1 min-h-0 overflow-auto"></div>
</div>

<style>
  :global(.cm-merge-view) { height: 100%; }
  :global(.cm-editor) { height: 100%; font-size: 12px; }
  :global(.cm-scroller) { font-family: "Cascadia Code", "Fira Code", "JetBrains Mono", monospace; }
</style>
