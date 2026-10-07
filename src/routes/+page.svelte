<script lang="ts">
  import { onMount } from 'svelte';
  import { Editor } from '$lib/stores/editor.svelte';
  import { AUTOSAVE_KEY } from '$lib/persistence/document';
  import { PRESETS_KEY } from '$lib/persistence/presets';
  import { createPlatform } from '$lib/platform';
  import { DocumentController } from '$lib/platform/document-controller';
  import { platformError, type PlatformAdapter, type StateKey } from '$lib/platform/types';
  import Toolbar from '$lib/components/editor/Toolbar.svelte';
  import EditorLayout from '$lib/components/editor/EditorLayout.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import '../app.css';
  const editor = new Editor();
  let ready = $state(false);
  let startupError = $state('');
  let platform: PlatformAdapter | undefined;
  let controller: DocumentController | undefined;
  let disposed = false;
  let unlistenClose: (() => void) | undefined;
  let fileInput: HTMLInputElement;
  function measureFooter(element: HTMLElement) {
    const update = () =>
      document.documentElement.style.setProperty(
        '--footer-height',
        `${element.getBoundingClientRect().height}px`
      );
    const observer = new ResizeObserver(update);
    observer.observe(element);
    update();
    return () => observer.disconnect();
  }
  async function recoverRecord(key: StateKey, consume: (text: string) => void) {
    if (!platform) return;
    let text: string | null;
    try {
      text = await platform.readState(key);
    } catch (error) {
      if (platform.desktop) throw error;
      editor.notify(platformError(error).message);
      return;
    }
    if (text === null) return;
    try {
      consume(text);
    } catch (error) {
      if (platform.desktop) await platform.quarantineState(key);
      editor.notify(`Saved ${key} could not be recovered: ${platformError(error).message}`);
    }
  }
  async function initialize() {
    startupError = '';
    try {
      platform ??= await createPlatform();
      editor.setPlatform(platform);
      controller ??= new DocumentController(editor, platform, flushStorage);
      await recoverRecord('recovery', (text) => controller!.recover(text));
      await recoverRecord('presets', (text) => editor.restorePresets(text));
      await recoverRecord('preferences', (text) => {
        if (platform!.desktop) {
          const preferences = JSON.parse(text);
          if (!preferences || !['flat', 'embossed'].includes(preferences.gridAppearance))
            throw new Error('The appearance preference is invalid.');
          editor.gridAppearance = preferences.gridAppearance;
        } else editor.gridAppearance = text === 'embossed' ? 'embossed' : 'flat';
      });
      try {
        const trace = await platform.loadTrace();
        if (!disposed) editor.restoreTrace(trace);
      } catch (error) {
        if (platform.desktop) {
          await platform.quarantineState('trace-metadata');
          editor.notify(`Tracing guide could not be recovered: ${platformError(error).message}`);
        }
      }
      if (platform.desktop && import.meta.env.VITE_DESKTOP_E2E === '1')
        await import('@wdio/tauri-plugin');
      if (!disposed) {
        if (platform.desktop && !unlistenClose) {
          const stop = await platform.onCloseRequested(async () => {
            await controller?.close();
          });
          if (disposed) {
            stop();
            return;
          }
          unlistenClose = stop;
        }
        await platform.setDocumentTitle(editor.doc.name, editor.dirty);
        ready = true;
      }
    } catch (error) {
      startupError = `Storage could not be loaded: ${platformError(error).message}`;
    }
  }
  onMount(() => {
    const media = window.matchMedia('(max-width: 999px)');
    const updateLayout = () => {
      editor.narrow = media.matches;
      if (!media.matches) editor.drawer = null;
    };
    updateLayout();
    media.addEventListener('change', updateLayout);
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
    void initialize();
    return () => {
      disposed = true;
      window.removeEventListener('storage', synchronizePresets);
      media.removeEventListener('change', updateLayout);
      unlistenClose?.();
      editor.destroy();
    };
  });
  $effect(() => {
    if (!ready) return;
    const trace = editor.trace;
    const timer = setTimeout(() => {
      if (controller?.closing) return;
      void platform?.saveTrace(trace).catch((e) => {
        if (trace || platform?.desktop) editor.notify(platformError(e).message);
      });
    }, 300);
    return () => clearTimeout(timer);
  });
  $effect(() => {
    if (!ready) return;
    const preference = platform?.desktop
      ? JSON.stringify({ gridAppearance: editor.gridAppearance })
      : editor.gridAppearance;
    void platform
      ?.writeState('preferences', preference)
      .catch((error) => editor.notify(platformError(error).message));
  });
  $effect(() => {
    if (!ready) return;
    const doc = editor.doc;
    controller?.documentChanged();
    void platform
      ?.setDocumentTitle(doc.name, editor.dirty)
      .catch((error) => editor.notify(platformError(error).message));
    editor.autosaveStatus = 'Saving…';
    const timer = setTimeout(() => {
      if (controller?.closing) return;
      void platform
        ?.writeState('recovery', JSON.stringify(doc))
        .then(() => {
          editor.autosaveStatus = 'Saved on this device';
        })
        .catch(() => {
          editor.autosaveStatus = 'Autosave unavailable';
          editor.notify(
            'Device storage is full or disabled. Save a project file to keep your work.'
          );
        });
    }, 350);
    return () => clearTimeout(timer);
  });
  function save(as = false) {
    void controller?.save(as);
  }
  async function flushStorage(includeRecovery: boolean) {
    if (!platform) return;
    await editor.flushAssets();
    await editor.flushPresets();
    await platform.writeState('presets', JSON.stringify(editor.presets));
    await platform.saveTrace(editor.trace);
    await platform.writeState(
      'preferences',
      platform.desktop
        ? JSON.stringify({ gridAppearance: editor.gridAppearance })
        : editor.gridAppearance
    );
    if (includeRecovery) await platform.writeState('recovery', editor.projectText());
  }
  function flushAutosave() {
    if (!ready || platform?.desktop) return;
    try {
      localStorage.setItem(AUTOSAVE_KEY, JSON.stringify(editor.doc));
    } catch {
      /* The visible autosave status already reports storage failures. */
    }
  }
  function open() {
    if (platform?.desktop) void controller?.open();
    else fileInput.click();
  }
  async function load(event: Event) {
    const input = event.target as HTMLInputElement,
      file = input.files?.[0];
    if (!file) return;
    await controller?.openBrowserFile(file);
    input.value = '';
  }
  function keyboard(event: KeyboardEvent) {
    if (
      !ready ||
      editor.transitioning ||
      editor.drawer !== null ||
      document.querySelector(
        '[data-popover-content][data-state="open"], [data-tooltip-content][data-state="open"], [data-dialog-content][data-state="open"]'
      ) ||
      ['INPUT', 'TEXTAREA', 'SELECT'].includes((event.target as HTMLElement)?.tagName) ||
      (event.target as HTMLElement)?.isContentEditable
    )
      return;
    const mod = event.ctrlKey || event.metaKey,
      key = event.key.toLowerCase();
    if (mod && key === 's') {
      event.preventDefault();
      save(event.shiftKey);
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
  ><title>Brickpress — Digital Letterpress</title><meta
    name="description"
    content="A focused studio for modular tile compositions and procedural letterpress impressions. Compose, press, and export."
  /></svelte:head
>
<svelte:window onkeydown={keyboard} onpagehide={flushAutosave} />
<input
  class="hidden-input"
  bind:this={fileInput}
  type="file"
  accept=".json,.brickpress.json,.legopress.json"
  aria-label="Open project file"
  onchange={load}
/>
<div
  class="editor-app"
  inert={!ready || editor.transitioning}
  aria-busy={!ready || editor.transitioning}
  data-ready={ready}
>
  <Toolbar
    {editor}
    onnew={() => {
      void controller?.newDocument();
    }}
    onopen={open}
    onsave={() => save()}
    onsaveas={() => save(true)}
  />
  <EditorLayout {editor} />
  <footer class="app-footer" {@attach measureFooter}>
    <span class="footer-document" title={editor.activeFilePath ?? undefined}
      >{editor.doc.name}{editor.dirty ? ' •' : ''}</span
    ><span class="footer-shortcuts"
      ><kbd>Space</kbd> pan <i>·</i> <kbd>R</kbd> rotate <i>·</i> <kbd>⇧</kbd> multi-select</span
    ><span class="footer-statistics"
      ><span>{editor.allPieces.length} pieces</span><span class="footer-dot">·</span><span
        >{editor.selected.length} selected</span
      ><span class="footer-dot">·</span><span class="local-save">{editor.autosaveStatus}</span
      ></span
    >
  </footer>
</div>
{#if startupError}
  <div class="startup-error" role="alert">
    <p>{startupError}</p>
    <button
      onclick={() => {
        void initialize();
      }}>Retry loading storage</button
    >
  </div>
{/if}
{#if editor.toast}<div class="toast" role="status">
    <Icon name="info" size={16} />{editor.toast}
  </div>{/if}

<style>
  .startup-error {
    position: fixed;
    inset: auto 12px 12px;
    z-index: 90;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 12px;
    background: white;
    color: #172033;
    border: 1px solid #cbd0d8;
    border-radius: 6px;
    font-size: 12px;
  }
  .startup-error button {
    min-height: 32px;
    padding: 4px 12px;
    border: 1px solid #cbd0d8;
    border-radius: 4px;
    background: #eef2f7;
    color: #172033;
  }
</style>
