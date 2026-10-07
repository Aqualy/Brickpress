<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import { bounds } from '../../geometry/geometry';
  import { getPiece } from '../../catalog/catalog';
  import Layers from '../layers/Layers.svelte';
  import ColorPanel from '../ui/ColorPanel.svelte';
  import PrintControls from '../print/PrintControls.svelte';
  import PaperControls from './PaperControls.svelte';
  import ExportControls from '../editor/ExportControls.svelte';
  import Icon from '../ui/Icon.svelte';
  import { Input } from '../ui/input';
  import { Button } from '../ui/button';
  import * as Tabs from '../ui/tabs';
  let { editor }: { editor: Editor } = $props();
  let selected = $derived(editor.selectedPieces);
  let first = $derived(selected[0]?.piece);

  const tabs = ['properties', 'layers', 'export'] as const;
</script>

<aside class="inspector-panel" aria-label="Properties, layers and export">
  <Tabs.Root
    class="inspector-tabs-root"
    value={editor.inspectorTab}
    onValueChange={(value) => {
      editor.inspectorTab = value as typeof editor.inspectorTab;
    }}
    loop
  >
    <Tabs.List class="inspector-tabs" aria-label="Inspector">
      {#each tabs as tab (tab)}<Tabs.Trigger value={tab}
          >{tab === 'properties'
            ? 'Properties'
            : tab === 'layers'
              ? 'Layers'
              : 'Export'}</Tabs.Trigger
        >{/each}
    </Tabs.List>
    <div class="inspector-content">
      <Tabs.Content value="properties" tabindex={-1}>
        <ColorPanel {editor} /><Layers {editor} compact />
        {#if first && editor.mode === 'design'}<section
            class="property-section selection-properties"
          >
            <h3>
              {selected.length === 1 ? 'Selected piece' : `${selected.length} pieces selected`}
            </h3>
            <div class="selected-summary">
              <strong
                >{selected.length === 1 ? getPiece(first.pieceId)?.name : 'Multiple pieces'}</strong
              ><small
                >{selected.length === 1 ? `Design ID ${first.pieceId}` : 'Move as a group'}</small
              >
            </div>
            <div class="dimensions-row">
              <label
                >X<Input
                  aria-label="Selection X"
                  type="number"
                  value={first.x}
                  onchange={(e) => editor.move(Number(e.currentTarget.value) - first.x, 0)}
                /></label
              ><label
                >Y<Input
                  aria-label="Selection Y"
                  type="number"
                  value={first.y}
                  onchange={(e) => editor.move(0, Number(e.currentTarget.value) - first.y)}
                /></label
              >
            </div>
            <p class="selection-dimensions">
              {bounds(first).width} × {bounds(first).height} studs · {first.rotation}°
            </p>
            <div class="transform-actions">
              <Button class="outline-button" onclick={() => editor.rotate()}
                ><Icon name="rotate" size={17} /> Rotate</Button
              ><Button class="outline-button" onclick={() => editor.duplicate()}
                ><Icon name="duplicate" size={17} /> Duplicate</Button
              >
            </div>
            <label class="field-label"
              >Ink pass<select
                aria-label="Selection ink pass"
                value={selected[0].pass.id}
                onchange={(e) => editor.moveToPass(e.currentTarget.value)}
                >{#each editor.doc.passes as pass (pass.id)}<option
                    value={pass.id}
                    disabled={pass.locked || !pass.visible}>{pass.name}</option
                  >{/each}</select
              ></label
            >
            <div class="selection-stack">
              <Button class="text-button" onclick={() => editor.reorderSelected(1)}
                ><Icon name="forward" size={15} /> Bring forward</Button
              ><Button class="text-button" onclick={() => editor.reorderSelected(-1)}
                ><Icon name="backward" size={15} /> Send backward</Button
              >
            </div>
            <Button class="text-button danger" onclick={() => editor.remove()}
              ><Icon name="trash" size={15} /> Delete selection</Button
            >
            {#if selected.length === 1}<p class="fine-print">
                {getPiece(first.pieceId)?.geometry.fidelity === 'approximate'
                  ? 'Approximate catalog silhouette.'
                  : 'Independent catalog print surface.'}
                {getPiece(first.pieceId)?.surfaceHeightPlates} plate printing height.
              </p>{/if}
          </section>{/if}
        <PaperControls {editor} /><PrintControls {editor} />
      </Tabs.Content>
      <Tabs.Content value="layers" tabindex={-1}>
        <Layers {editor} />
      </Tabs.Content>
      <Tabs.Content value="export" tabindex={-1}>
        <ExportControls {editor} />
      </Tabs.Content>
    </div>
  </Tabs.Root>
  <div class="inspector-footer">
    <span>{editor.doc.options.physical ? 'Physical print mode' : 'Digital print mode'}</span>
  </div>
</aside>
