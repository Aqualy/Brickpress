<script lang="ts">
  import { onMount } from 'svelte';
  import { Editor } from '$lib/stores/editor.svelte';
  import { AUTOSAVE_KEY } from '$lib/persistence/document';
  import { PRESETS_KEY } from '$lib/persistence/presets';
  import { loadTrace, saveTrace } from '$lib/persistence/tracing';
  import { downloadBlob, filename } from '$lib/export/export';
  import Toolbar from '$lib/components/editor/Toolbar.svelte';
  import EditorLayout from '$lib/components/editor/EditorLayout.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import '../app.css';
  const editor = new Editor();
  let ready = $state(false);
  let fileInput: HTMLInputElement;
  onMount(() => {
    const media = window.matchMedia('(max-width: 999px)');
    const updateLayout = () => {
      editor.narrow = media.matches;
      if (!media.matches) editor.drawer = null;
    };
    updateLayout();
    media.addEventListener('change', updateLayout);
    try {
      editor.gridAppearance =
        localStorage.getItem('form-impression-grid-appearance') === 'embossed'
          ? 'embossed'
          : 'flat';
    } catch {
      /* Optional presentation preference. */
    }
    try {
      const saved = localStorage.getItem(AUTOSAVE_KEY);
      if (saved) editor.load(saved, true);
    } catch (e) {
      editor.notify(
        e instanceof Error
          ? `Autosave could not be recovered: ${e.message}`
          : 'Autosave is unavailable.'
      );
    }
    try {
      editor.restorePresets(localStorage.getItem(PRESETS_KEY) ?? '[]');
    } catch (e) {
      editor.notify(
        e instanceof Error
          ? `Preset library could not be recovered: ${e.message}`
          : 'Preset storage is unavailable.'
      );
    }
    const synchronizePresets = (event: StorageEvent) => {
      if (event.key === PRESETS_KEY) {
        try {
          editor.restorePresets(event.newValue ?? '[]');
        } catch {
          editor.notify('The preset library in another tab is invalid.');
        }
      }
    };
    window.addEventListener('storage', synchronizePresets);
    let disposed = false;
    void loadTrace()
      .then((trace) => {
        if (!disposed) editor.restoreTrace(trace);
      })
      .catch(() => {
        /* Guides are optional; uploading an image can retry storage. */
      })
      .finally(() => {
        if (!disposed) ready = true;
      });
    return () => {
      disposed = true;
      window.removeEventListener('storage', synchronizePresets);
      media.removeEventListener('change', updateLayout);
      editor.destroy();
    };
  });
  $effect(() => {
    if (!ready) return;
    const trace = editor.trace;
    const timer = setTimeout(() => {
      void saveTrace(trace).catch((e) => {
        if (trace)
          editor.notify(e instanceof Error ? e.message : 'Tracing storage is unavailable.');
      });
    }, 300);
    return () => clearTimeout(timer);
  });
  $effect(() => {
    if (!ready) return;
    try {
      localStorage.setItem('form-impression-grid-appearance', editor.gridAppearance);
    } catch {
      /* The document can still be saved. */
    }
  });
  $effect(() => {
    if (!ready) return;
    const doc = editor.doc;
    editor.autosaveStatus = 'Saving…';
    const timer = setTimeout(() => {
      try {
        localStorage.setItem(AUTOSAVE_KEY, JSON.stringify(doc));
        editor.autosaveStatus = 'Saved on this device';
      } catch {
        editor.autosaveStatus = 'Autosave unavailable';
        editor.notify('Device storage is full or disabled. Save a project file to keep your work.');
      }
    }, 350);
    return () => clearTimeout(timer);
  });
  function save() {
    downloadBlob(
      new Blob([editor.projectText()], { type: 'application/json' }),
      `${filename(editor.doc.name)}.legopress.json`
    );
    editor.saved();
  }
  function flushAutosave() {
    if (!ready) return;
    try {
      localStorage.setItem(AUTOSAVE_KEY, JSON.stringify(editor.doc));
    } catch {
      /* The visible autosave status already reports storage failures. */
    }
  }
  function open() {
    fileInput.click();
  }
  async function load(event: Event) {
    const input = event.target as HTMLInputElement,
      file = input.files?.[0];
    if (!file) return;
    if (
      editor.dirty &&
      !window.confirm(
        'Open this project and replace the current composition? Save first if you want to keep it.'
      )
    ) {
      input.value = '';
      return;
    }
    try {
      if (file.size > 20_000_000) throw new Error('Project files must be smaller than 20 MB.');
      editor.load(await file.text());
    } catch (e) {
      editor.notify(e instanceof Error ? e.message : 'The project could not be opened.');
    }
    input.value = '';
  }
  function keyboard(event: KeyboardEvent) {
    if (
      editor.drawer !== null ||
      document.querySelector(
        '[data-popover-content][data-state="open"], [data-tooltip-content][data-state="open"]'
      ) ||
      ['INPUT', 'TEXTAREA', 'SELECT'].includes((event.target as HTMLElement)?.tagName) ||
      (event.target as HTMLElement)?.isContentEditable
    )
      return;
    const mod = event.ctrlKey || event.metaKey,
      key = event.key.toLowerCase();
    if (mod && key === 's') {
      event.preventDefault();
      save();
      return;
    }
    if (mod && key === 'o') {
      event.preventDefault();
      open();
      return;
    }
    if (mod && key === 'z') {
      event.preventDefault();
      event.shiftKey ? editor.redo() : editor.undo();
      return;
    }
    if (mod && key === 'y') {
      event.preventDefault();
      editor.redo();
      return;
    }
    // Bare-key commands belong to the artboard; tabs and popovers handle their own keys.
    if (!(event.target instanceof Element) || !event.target.matches('.canvas-workspace')) return;
    if (event.key === 'Escape') {
      editor.tool = 'select';
      editor.selected = [];
      return;
    }
    if (key === 'g' && !mod) {
      editor.commit((doc) => (doc.options.grid = doc.options.grid === 'off' ? 'points' : 'off'));
      return;
    }
    if (key === '/' && !mod) {
      event.preventDefault();
      document.querySelector<HTMLInputElement>('input[aria-label="Search pieces"]')?.focus();
      return;
    }
    if (key === 'h' && !mod) {
      editor.tool = 'hand';
      return;
    }
    if (editor.mode === 'print') return;
    if (mod && ['d', 'c', 'v', 'a'].includes(key)) {
      event.preventDefault();
      if (key === 'd') editor.duplicate();
      else if (key === 'c') editor.copy();
      else if (key === 'v') editor.paste();
      else
        editor.selected = editor.allPieces
          .filter((p) => p.pass.visible && !p.pass.locked)
          .map((p) => p.piece.uid);
      return;
    }
    if (mod) return;
    if (key === 'r') {
      event.preventDefault();
      editor.rotate();
    } else if (key === 'v') editor.tool = 'select';
    else if (key === 'b') {
      editor.tool = 'place';
      editor.selected = [];
    } else if (key === 'h') editor.tool = 'hand';
    else if (key === 'delete' || key === 'backspace') {
      event.preventDefault();
      editor.remove();
    } else if (event.key.startsWith('Arrow') && editor.tool !== 'place' && editor.tool !== 'hand') {
      event.preventDefault();
      const step = event.shiftKey ? 4 : 1;
      editor.move(
        event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0,
        event.key === 'ArrowDown' ? step : event.key === 'ArrowUp' ? -step : 0
      );
    }
  }
</script>

<svelte:head
  ><title>Form & Impression — Digital Letterpress</title><meta
    name="description"
    content="A focused studio for modular tile compositions and procedural letterpress impressions. Compose, press, and export."
  /></svelte:head
>
<svelte:window onkeydown={keyboard} onpagehide={flushAutosave} />
<input
  class="hidden-input"
  bind:this={fileInput}
  type="file"
  accept=".json,.legopress.json"
  aria-label="Open project file"
  onchange={load}
/>
<div class="editor-app" inert={!ready} aria-busy={!ready} data-ready={ready}>
  <Toolbar
    {editor}
    onnew={() => editor.newDocument()}
    onopen={open}
    onsave={save}
    onexport={() => editor.openExport()}
  />
  <EditorLayout {editor} />
  <footer class="app-footer">
    <span class="footer-document">{editor.doc.name}{editor.dirty ? ' •' : ''}</span><span
      class="footer-shortcuts"
      ><kbd>Space</kbd> pan <i>·</i> <kbd>R</kbd> rotate <i>·</i> <kbd>⇧</kbd> multi-select</span
    ><span
      >{editor.allPieces.length} pieces<span class="footer-dot">·</span>{editor.selected.length} selected<span
        class="footer-dot">·</span
      ><span class="local-save">{editor.autosaveStatus}</span></span
    >
  </footer>
</div>
{#if editor.toast}<div class="toast" role="status">
    <Icon name="info" size={16} />{editor.toast}
  </div>{/if}
