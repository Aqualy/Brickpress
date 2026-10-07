<script lang="ts">
  import { categories, pieces, isPhysical } from '../../catalog/catalog';
  import type { Editor } from '../../stores/editor.svelte';
  import Icon from '../ui/Icon.svelte';
  import { Input } from '../ui/input';
  import Popover from '../ui/Popover.svelte';
  import { Tooltip } from 'bits-ui';
  import * as Tabs from '../ui/tabs';
  import Presets from './Presets.svelte';
  let { editor }: { editor: Editor } = $props();
  let query = $state(''),
    category = $state('all'),
    compatible = $state(false);
  let filtered = $derived(
    pieces.filter((p) => {
      const search = query.toLowerCase().replaceAll('×', 'x').replaceAll(' ', '');
      const dims = `${p.footprint.widthStuds}x${p.footprint.heightStuds} ${p.footprint.heightStuds}x${p.footprint.widthStuds}`;
      const aliases =
        (categories.find((c) => c.id === p.category)?.name ?? '') +
        (p.category === 'rectilinear'
          ? ' rectangular rectangle'
          : p.category === 'wedge'
            ? ' wedges angular'
            : '');
      return (
        (category === 'all' || p.category === category) &&
        (!compatible || isPhysical(p)) &&
        `${p.name} ${p.id} ${aliases} ${dims}`
          .toLowerCase()
          .replaceAll('×', 'x')
          .replaceAll(' ', '')
          .includes(search)
      );
    })
  );
  function dragstart(event: DragEvent, id: string) {
    editor.choosePiece(id);
    event.dataTransfer?.setData('application/x-legopress-piece', id);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
  }
</script>

<Tooltip.Provider delayDuration={400} skipDelayDuration={100}>
  <aside class="palette-panel" aria-label="Piece palette">
    <div class="panel-title">
      <h2>{editor.paletteTab === 'pieces' ? 'Pieces' : 'Presets'}</h2>
      {#if editor.paletteTab === 'pieces'}
        <span
          class="active-ink-dot"
          style:background={editor.activePass.color}
          title={`Placement ink ${editor.activePass.color}`}
        ></span>
        <Popover label="Piece filters" className="icon-button" width={270}>
          {#snippet trigger()}<Icon name="settings" size={18} />{/snippet}
          <h3>Piece filters</h3>
          <label class="check-row"
            ><input type="checkbox" bind:checked={compatible} /> Standard printing heights only</label
          >
          <p class="fine-print">
            {pieces.length} catalog pieces. Thumbnails preview the active ink.
          </p>
        </Popover>
      {/if}
    </div>
    <Tabs.Root
      class="palette-tabs-root"
      value={editor.paletteTab}
      onValueChange={(value) => (editor.paletteTab = value as typeof editor.paletteTab)}
      loop
    >
      <Tabs.List class="palette-library-tabs" aria-label="Asset library">
        <Tabs.Trigger value="pieces">Pieces</Tabs.Trigger><Tabs.Trigger value="presets"
          >Presets</Tabs.Trigger
        >
      </Tabs.List>
      <Tabs.Content value="pieces" class="piece-tab" tabindex={-1}>
        <div class="palette-top">
          <label class="search-field"
            ><Icon name="search" size={16} /><Input
              placeholder="Search pieces…"
              aria-label="Search pieces"
              bind:value={query}
            /></label
          >
          <nav class="category-filters" aria-label="Piece categories">
            <button
              class:active={category === 'all'}
              aria-current={category === 'all' ? 'true' : undefined}
              onclick={() => (category = 'all')}>All pieces<small>{pieces.length}</small></button
            >
            {#each categories as item (item.id)}<button
                class:active={category === item.id}
                aria-current={category === item.id ? 'true' : undefined}
                onclick={() => {
                  category = item.id;
                }}>{item.name}</button
              >{/each}
          </nav>
        </div>
        <div class="piece-list">
          <div class="piece-grid">
            {#each filtered as piece (piece.id)}{@const disabled =
                editor.doc.options.physical && !isPhysical(piece)}
              <Tooltip.Root
                ><div class="piece-card-wrap">
                  <Tooltip.Trigger
                    class={[
                      'piece-card',
                      !editor.activePresetId &&
                        editor.activePieceId === piece.id &&
                        editor.tool === 'place' &&
                        'active'
                    ]}
                    aria-pressed={!editor.activePresetId &&
                      editor.activePieceId === piece.id &&
                      editor.tool === 'place'}
                    aria-disabled={disabled}
                    draggable={!disabled}
                    ondragstart={(e) => dragstart(e, piece.id)}
                    onclick={() => {
                      if (disabled) return;
                      editor.choosePiece(piece.id);
                      editor.focusCanvas();
                    }}
                    aria-label={`Place ${piece.name}, ${piece.designId}`}
                  >
                    <svg
                      class="piece-thumbnail"
                      viewBox={`${piece.geometry.viewBox[0] - 0.15} ${piece.geometry.viewBox[1] - 0.15} ${piece.geometry.viewBox[2] + 0.3} ${piece.geometry.viewBox[3] + 0.3}`}
                      aria-hidden="true"
                      ><path
                        d={piece.geometry.path}
                        fill={editor.activePass.color}
                        fill-rule={piece.geometry.fillRule}
                        stroke="#111827"
                        stroke-opacity=".12"
                        stroke-width=".015"
                      /></svg
                    >
                    <span class="piece-size"
                      >{piece.footprint.widthStuds}×{piece.footprint.heightStuds}{piece.geometry
                        .fidelity === 'approximate'
                        ? ' ≈'
                        : ''}</span
                    >
                  </Tooltip.Trigger>
                  <Tooltip.Portal
                    ><Tooltip.Content
                      class="piece-tooltip"
                      role="tooltip"
                      side="right"
                      align="start"
                      sideOffset={8}
                      collisionPadding={12}
                    >
                      <strong>{piece.name.replace(/^Tile /, '')}</strong><span
                        >{piece.designId} · {piece.footprint.widthStuds} × {piece.footprint
                          .heightStuds} studs</span
                      >
                      <span
                        >{piece.geometry.fidelity === 'approximate'
                          ? 'Approximate catalog silhouette'
                          : 'Catalog silhouette'} · {piece.surfaceHeightPlates} plate height</span
                      >
                      {#if disabled}<span>Non-standard printing height</span>{/if}
                    </Tooltip.Content></Tooltip.Portal
                  >
                </div></Tooltip.Root
              >
            {/each}
          </div>
          {#if !filtered.length}<p class="empty-state">
              No matching pieces.<br />Try a design ID or stud size.
            </p>{/if}
        </div>
      </Tabs.Content>
      <Tabs.Content value="presets" class="preset-tab" tabindex={-1}
        ><Presets {editor} /></Tabs.Content
      >
    </Tabs.Root>
  </aside>
</Tooltip.Provider>
