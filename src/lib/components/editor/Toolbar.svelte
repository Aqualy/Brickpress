<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import Icon from '../ui/Icon.svelte';
  import { Input } from '../ui/input';
  import { Button } from '../ui/button';
  import Popover from '../ui/Popover.svelte';
  import ZoomControls from '../ui/ZoomControls.svelte';
  import GridControls from './GridControls.svelte';
  let {
    editor,
    onnew,
    onopen,
    onsave,
    onexport
  }: {
    editor: Editor;
    onnew: () => void;
    onopen: () => void;
    onsave: () => void;
    onexport: () => void;
  } = $props();
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
      <h3>Form & Impression</h3>
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
        <Button onclick={onnew}><Icon name="file" /> New</Button><Button onclick={onopen}
          ><Icon name="open" /> Open</Button
        ><Button onclick={onsave}><Icon name="save" /> Save</Button>
      </div>
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
        Keyboard tools work when the artboard has focus. V Select · B Place · H Hand · R Rotate · G
        Grid. Ctrl/⌘ S saves.
      </p>
    </Popover>
    <Button
      class="icon-button drawer-toggle"
      aria-label="Open Pieces"
      onclick={() => (editor.drawer = 'palette')}><Icon name="cube" size={22} /></Button
    >
  </div>
  <div class="toolbar-group file-actions desktop-actions">
    <Button class="toolbar-button" onclick={onnew} title="New document"
      ><Icon name="file" size={22} /><span>New</span></Button
    >
    <Button class="toolbar-button" onclick={onopen} title="Open · Ctrl/⌘ O"
      ><Icon name="open" size={22} /><span>Open</span></Button
    >
    <Button class="toolbar-button" onclick={onsave} title="Save · Ctrl/⌘ S"
      ><Icon name="save" size={22} /><span>Save</span></Button
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
      title="Place · B"
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
  <div class="toolbar-zoom"><ZoomControls {editor} location="Toolbar" /></div>
  <div class="toolbar-group export-group">
    <Button class="toolbar-button export-trigger" aria-label="Export" onclick={onexport}
      ><Icon name="export" size={22} /><span>Export</span></Button
    >
    <Button
      class="icon-button drawer-toggle"
      aria-label="Open Properties"
      onclick={() => (editor.drawer = 'inspector')}><Icon name="settings" size={22} /></Button
    >
    <span class="save-indicator" title={editor.autosaveStatus}
      ><span class="status-dot"></span><span class="sr-only">{editor.autosaveStatus}</span></span
    >
  </div>
</header>
