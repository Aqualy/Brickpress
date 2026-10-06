<script lang="ts">
  import { untrack } from 'svelte';
  import type { PaneGroup } from 'paneforge';
  import * as Resizable from '../ui/resizable';
  import SidePanel from '../ui/SidePanel.svelte';
  import Palette from '../palette/Palette.svelte';
  import Inspector from '../properties/Inspector.svelte';
  import Board from './Board.svelte';
  import type { Editor } from '../../stores/editor.svelte';
  let { editor }: { editor: Editor } = $props();
  let width = $state(1448);
  let group = $state<PaneGroup>();
  let available = $derived(Math.max(1, width - (editor.narrow ? 0 : 2)));
  let left = $derived(editor.narrow ? 0 : ((width < 1200 ? 248 : 306) / available) * 100);
  let right = $derived(editor.narrow ? 0 : ((width < 1200 ? 296 : 340) / available) * 100);
  $effect(() => {
    const layout = [left, 100 - left - right, right];
    const api = group;
    editor.panelResetToken;
    // Synchronize responsive defaults with Paneforge's imperative layout API.
    if (api) untrack(() => api.setLayout(layout));
  });
</script>

<div class="editor-layout" bind:clientWidth={width}>
  <Resizable.PaneGroup
    bind:api={group}
    direction="horizontal"
    keyboardResizeBy={2}
    class="editor-panes"
  >
    <Resizable.Pane
      defaultSize={left}
      minSize={editor.narrow ? 0 : (208 / available) * 100}
      maxSize={editor.narrow ? 0 : (420 / available) * 100}
      collapsible
      collapsedSize={0}
      order={1}
      class="sidebar-pane"
    >
      <SidePanel {editor} side="palette" label="Pieces"><Palette {editor} /></SidePanel>
    </Resizable.Pane>
    <Resizable.Handle
      class={editor.narrow ? 'panel-resizer hidden' : 'panel-resizer'}
      disabled={editor.narrow}
      aria-label="Resize Pieces panel"
    />
    <Resizable.Pane defaultSize={100 - left - right} minSize={editor.narrow ? 100 : 25} order={2}>
      <main class="editor-center"><Board {editor} /></main>
    </Resizable.Pane>
    <Resizable.Handle
      class={editor.narrow ? 'panel-resizer hidden' : 'panel-resizer'}
      disabled={editor.narrow}
      aria-label="Resize Inspector panel"
    />
    <Resizable.Pane
      defaultSize={right}
      minSize={editor.narrow ? 0 : (250 / available) * 100}
      maxSize={editor.narrow ? 0 : (460 / available) * 100}
      collapsible
      collapsedSize={0}
      order={3}
      class="sidebar-pane"
    >
      <SidePanel {editor} side="inspector" label="Inspector"><Inspector {editor} /></SidePanel>
    </Resizable.Pane>
  </Resizable.PaneGroup>
</div>
