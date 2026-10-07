<script lang="ts">
  import { asset } from '$app/paths';
  import { Dialog } from 'bits-ui';
  import type { Editor } from '../../stores/editor.svelte';
  import Icon from '../ui/Icon.svelte';
  import { Input } from '../ui/input';
  import { Button } from '../ui/button';
  import Popover from '../ui/Popover.svelte';
  import GridControls from './GridControls.svelte';
  let {
    editor,
    onnew,
    onopen,
    onsave,
    onsaveas
  }: {
    editor: Editor;
    onnew: () => void;
    onopen: () => void;
    onsave: () => void;
    onsaveas: () => void;
  } = $props();
  let noticesOpen = $state(false),
    notices = $state('');
  async function openNotices() {
    noticesOpen = true;
    notices = 'Loading dependency notices…';
    try {
      const responses = await Promise.all([
        fetch(asset('/THIRD_PARTY_NOTICES.txt')),
        fetch(asset('/RUST_DEPENDENCY_NOTICES.txt'))
      ]);
      if (responses.some((response) => !response.ok))
        throw new Error(
          'Dependency notices could not be loaded. They are also included with the application.'
        );
      notices = (await Promise.all(responses.map((response) => response.text()))).join('\n\n');
    } catch (error) {
      notices = error instanceof Error ? error.message : 'Dependency notices could not be loaded.';
    }
  }
  function tool(value: 'select' | 'place' | 'hand') {
    editor.tool = value;
    if (value !== 'hand') editor.mode = 'design';
    if (value === 'place') editor.selected = [];
    editor.focusCanvas();
  }
</script>

<header class="app-header" aria-label="Editor toolbar">
  <div class="toolbar-group menu-group">
    <Popover label="Main menu" className="menu-trigger">
      {#snippet trigger()}<Icon name="menu" size={22} />{/snippet}
      <h3>Brickpress</h3>
      <label class="field-label"
        >Document name<Input
          aria-label="Document name"
          value={editor.doc.name}
          onchange={(e) =>
            editor.commit(
              (doc) => (doc.name = e.currentTarget.value.slice(0, 160) || 'Untitled impression')
            )}
        /></label
      >
      <div class="menu-actions">
        <Button disabled={editor.documentBusy} onclick={onnew}><Icon name="file" /> New</Button
        ><Button disabled={editor.documentBusy} onclick={onopen}><Icon name="open" /> Open</Button
        ><Button disabled={editor.documentBusy} onclick={onsave}><Icon name="save" /> Save</Button>
      </div>
      <Button class="menu-action" disabled={editor.documentBusy} onclick={onsaveas}
        ><Icon name="save" /> Save As <kbd>⇧⌘/Ctrl S</kbd></Button
      >
      <hr />
      <Button class="menu-action" aria-label="Hand tool" onclick={() => tool('hand')}
        ><Icon name="hand" /> Hand tool <kbd>H</kbd></Button
      >
      <Button class="menu-action" onclick={() => editor.setZoom('fit')}
        ><Icon name="fit" /> Fit artboard</Button
      >
      <Button class="menu-action" onclick={() => editor.setZoom(200)}
        ><Icon name="plus" /> Zoom to 200%</Button
      >
      <Button class="menu-action" onclick={() => editor.panelResetToken++}
        ><Icon name="settings" /> Reset panel widths</Button
      >
      <hr />
      <div class="menu-actions">
        <Button disabled={!editor.selected.length} onclick={() => editor.copy()}>Copy</Button
        ><Button onclick={() => editor.paste()}>Paste</Button><Button
          disabled={!editor.selected.length}
          onclick={() => editor.duplicate()}>Duplicate</Button
        >
      </div>
      <Button class="menu-action danger" onclick={() => editor.clear()}
        ><Icon name="trash" /> Clear canvas</Button
      >
      <div class="menu-actions compact-actions">
        <Button disabled={!editor.canUndo} onclick={() => editor.undo()}>Undo</Button><Button
          disabled={!editor.canRedo}
          onclick={() => editor.redo()}>Redo</Button
        ><Button
          disabled={editor.mode === 'print' || (!editor.selected.length && editor.tool !== 'place')}
          onclick={() => editor.rotate()}>Rotate</Button
        >
      </div>
      <p class="fine-print">
        Artboard: V Select · B Place · H Hand · R Rotate · G Grid. Ctrl/⌘ S saves. Place mode:
        middle-click or arrows + I picks a piece.
      </p>
      <p class="fine-print">
        Independent software. LEGO® is a trademark of the LEGO Group, which does not sponsor,
        authorize or endorse Brickpress. {#if editor.desktop}<button
            class="notices-link menu-action"
            onclick={openNotices}>Third-party notices</button
          >{:else}<a
            class="underline underline-offset-2"
            href={asset('/THIRD_PARTY_NOTICES.txt')}
            target="_blank"
            rel="noreferrer">Third-party notices</a
          >{/if}.
      </p>
    </Popover>
    <Button
      class="icon-button drawer-toggle"
      aria-label="Open Pieces"
      onclick={() => (editor.drawer = 'palette')}><Icon name="cube" size={22} /></Button
    >
  </div>
  <div class="toolbar-group file-actions desktop-actions">
    <Button
      class="toolbar-button"
      disabled={editor.documentBusy}
      onclick={onnew}
      title="New document"><Icon name="file" size={22} /><span>New</span></Button
    >
    <Button
      class="toolbar-button"
      disabled={editor.documentBusy}
      onclick={onopen}
      title="Open · Ctrl/⌘ O"><Icon name="open" size={22} /><span>Open</span></Button
    >
    <Button
      class="toolbar-button"
      disabled={editor.documentBusy}
      onclick={onsave}
      title="Save · Ctrl/⌘ S"><Icon name="save" size={22} /><span>Save</span></Button
    >
  </div>
  <div class="toolbar-group history-actions desktop-actions">
    <Button
      class="toolbar-button"
      disabled={!editor.canUndo}
      onclick={() => editor.undo()}
      title="Undo · Ctrl/⌘ Z"><Icon name="undo" size={22} /><span>Undo</span></Button
    >
    <Button
      class="toolbar-button"
      disabled={!editor.canRedo}
      onclick={() => editor.redo()}
      title="Redo · Ctrl/⌘ Shift Z"><Icon name="redo" size={22} /><span>Redo</span></Button
    >
  </div>
  <div class="toolbar-group tool-options">
    <Button
      class={['toolbar-button', editor.tool === 'select' && 'active']}
      aria-label="Select tool"
      aria-pressed={editor.tool === 'select'}
      title="Select · V"
      onclick={() => tool('select')}><Icon name="arrow" size={22} /><span>Select</span></Button
    >
    <Button
      class={['toolbar-button', editor.tool === 'place' && 'active']}
      aria-label="Place tool"
      aria-pressed={editor.tool === 'place'}
      title="Place · B · Middle-click a piece to pick its shape and orientation"
      onclick={() => tool('place')}><Icon name="cube" size={22} /><span>Place</span></Button
    >
    <Button
      class="toolbar-button desktop-actions"
      aria-label="Rotate pieces"
      disabled={editor.mode === 'print' || (!editor.selected.length && editor.tool !== 'place')}
      onclick={() => editor.rotate()}><Icon name="rotate" size={22} /><span>Rotate</span></Button
    >
    <Popover label="Grid settings" className="toolbar-button grid-trigger">
      {#snippet trigger()}<Icon name="grid" size={22} /><span>Grid</span>{/snippet}
      <GridControls {editor} />
    </Popover>
  </div>
  <div class="mode-switch" role="group" aria-label="Editor mode">
    <Button
      class={editor.mode === 'design' ? 'active' : ''}
      aria-pressed={editor.mode === 'design'}
      onclick={() => (editor.mode = 'design')}>Design</Button
    >
    <Button
      class={editor.mode === 'print' ? 'active' : ''}
      aria-pressed={editor.mode === 'print'}
      onclick={() => {
        editor.mode = 'print';
        editor.selected = [];
      }}>Print preview</Button
    >
  </div>
  <div class="toolbar-group inspector-drawer-group">
    <Button
      class="icon-button drawer-toggle"
      aria-label="Open Inspector"
      onclick={() => (editor.drawer = 'inspector')}><Icon name="settings" size={22} /></Button
    >
  </div>
</header>

<Dialog.Root bind:open={noticesOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="notices-overlay" />
    <Dialog.Content
      class="notices-dialog"
      onCloseAutoFocus={(event) => {
        event.preventDefault();
        document.querySelector<HTMLButtonElement>('button[aria-label="Main menu"]')?.focus();
      }}
    >
      <div class="notices-heading">
        <Dialog.Title>Dependency notices</Dialog.Title><Dialog.Close
          aria-label="Close dependency notices"><Icon name="close" size={16} /></Dialog.Close
        >
      </div>
      <Dialog.Description
        >Licenses and attribution for dependencies bundled with Brickpress.</Dialog.Description
      >
      <textarea readonly aria-label="Dependency license text" value={notices}></textarea>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  .notices-link {
    display: inline;
    padding: 0;
    min-height: 24px;
    color: inherit;
    text-decoration: underline;
    background: none;
    border: 0;
    font: inherit;
  }
  :global(.notices-overlay) {
    position: fixed;
    inset: 0;
    z-index: 80;
    background: #17203333;
  }
  :global(.notices-dialog) {
    position: fixed;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    z-index: 81;
    width: min(760px, calc(100vw - 24px));
    max-height: calc(100dvh - 24px);
    display: flex;
    flex-direction: column;
    padding: 16px;
    background: white;
    border: 1px solid #cbd0d8;
    border-radius: 6px;
    color: #172033;
    gap: 12px;
    font-size: 12px;
  }
  .notices-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .notices-heading :global(h2) {
    font-size: 14px;
    font-weight: 600;
  }
  .notices-heading :global(button) {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: 1px solid #dce0e6;
    border-radius: 4px;
  }
  textarea {
    overflow: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: 11px;
    min-height: min(480px, 55dvh);
    max-height: 70dvh;
    border: 1px solid #dce0e6;
    border-radius: 4px;
    padding: 12px;
    resize: none;
    width: 100%;
    font-family: monospace;
  }
</style>
