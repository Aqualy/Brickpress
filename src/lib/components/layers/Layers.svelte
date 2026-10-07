<script lang="ts">
  import { tick } from 'svelte';
  import { getPiece } from '../../catalog/catalog';
  import type { Editor } from '../../stores/editor.svelte';
  import Icon from '../ui/Icon.svelte';
  import { Input } from '../ui/input';
  import { Button } from '../ui/button';
  import Popover from '../ui/Popover.svelte';
  let { editor, compact = false }: { editor: Editor; compact?: boolean } = $props();
  let expanded = $state.raw<string[]>([]);
  let dragged = $state<string | null>(null),
    dropTarget = $state<string | null>(null),
    after = $state(false);
  function dragOver(event: DragEvent, id: string) {
    if (!dragged) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    const box =
      event.currentTarget instanceof HTMLElement
        ? event.currentTarget.getBoundingClientRect()
        : null;
    dropTarget = id;
    after = !!box && event.clientY > box.top + box.height / 2;
  }
  function drop(event: DragEvent, target: string) {
    if (!dragged) return;
    event.preventDefault();
    event.stopPropagation();
    const from = editor.doc.passes.findIndex((p) => p.id === dragged);
    const to = editor.doc.passes.findIndex((p) => p.id === target);
    const index = to + (after ? 1 : 0) - (from < to + (after ? 1 : 0) ? 1 : 0);
    editor.movePass(dragged, index);
    editor.notify('Ink pass order updated.');
    dragged = null;
    dropTarget = null;
  }
  async function keyReorder(event: KeyboardEvent, id: string) {
    if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return;
    event.preventDefault();
    event.stopPropagation();
    const handle = event.currentTarget as HTMLButtonElement;
    editor.reorderPass(id, event.key === 'ArrowUp' ? -1 : 1);
    editor.notify('Ink pass order updated.');
    await tick();
    handle.focus();
  }
  function expand(id: string) {
    expanded = expanded.includes(id) ? expanded.filter((p) => p !== id) : [...expanded, id];
  }
</script>

<section class="layers-panel property-section" class:compact>
  <div class="section-heading">
    <h3>Ink Passes</h3>
    <Button
      class="icon-button"
      aria-label="Add ink pass"
      title="Add ink pass"
      onclick={() => editor.addPass()}><Icon name="plus" size={17} /></Button
    >
  </div>
  {#each editor.doc.passes as pass, index (pass.id)}
    <div
      class="pass-block"
      class:active={editor.activePassId === pass.id}
      class:dragging={dragged === pass.id}
      class:drop-before={dropTarget === pass.id && !after}
      class:drop-after={dropTarget === pass.id && after}
      role="group"
      aria-label={pass.name}
      ondragover={(e) => dragOver(e, pass.id)}
      ondrop={(e) => drop(e, pass.id)}
    >
      <div class="pass-row">
        <Button
          class="icon-button pass-grip"
          aria-label={`Reorder ${pass.name}`}
          title="Drag to reorder; use Up or Down while focused"
          draggable
          ondragstart={(e) => {
            dragged = pass.id;
            e.dataTransfer?.setData('application/x-brickpress-pass', pass.id);
            if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
          }}
          ondragend={() => {
            dragged = null;
            dropTarget = null;
          }}
          onkeydown={(e) => keyReorder(e, pass.id)}><Icon name="grip" size={14} /></Button
        >
        {#if !compact}<Button
            class="icon-button expand-pass"
            aria-expanded={expanded.includes(pass.id)}
            aria-label={`Expand ${pass.name}`}
            onclick={() => expand(pass.id)}
            ><span class:expanded={expanded.includes(pass.id)}
              ><Icon name="chevron" size={14} /></span
            ></Button
          >{/if}
        <Button
          class="icon-button"
          aria-label={`${pass.visible ? 'Hide' : 'Show'} ${pass.name}`}
          title="Toggle visibility"
          onclick={() => editor.updatePass(pass.id, { visible: !pass.visible })}
          ><Icon name={pass.visible ? 'eye' : 'hidden'} size={20} /></Button
        >
        <Button
          class="pass-swatch"
          size="icon-xs"
          style={`background: ${pass.color}`}
          aria-pressed={editor.activePassId === pass.id}
          aria-label={`Use ${pass.name}`}
          title="Use this ink pass"
          onclick={() => (editor.activePassId = pass.id)}
        ></Button>
        <Input
          class="pass-name"
          aria-label={`Rename ink pass ${index + 1}`}
          value={pass.name}
          onfocus={() => (editor.activePassId = pass.id)}
          onchange={(e) =>
            editor.updatePass(pass.id, {
              name: e.currentTarget.value.slice(0, 100) || `Ink Pass ${index + 1}`
            })}
        />
        {#if pass.locked}<Icon name="lock" size={14} />{/if}
        <Popover label={`Actions for ${pass.name}`} className="icon-button pass-menu">
          {#snippet trigger()}<Icon name="more" size={20} />{/snippet}
          <h3>{pass.name}</h3>
          <label class="field-label"
            >Color<input
              type="color"
              aria-label={`Recolor ${pass.name}`}
              value={pass.color}
              onchange={(e) => editor.updatePass(pass.id, { color: e.currentTarget.value })}
            /></label
          >
          <Button
            class="menu-action"
            aria-label={`${pass.locked ? 'Unlock' : 'Lock'} ${pass.name}`}
            onclick={() => editor.updatePass(pass.id, { locked: !pass.locked })}
            ><Icon name={pass.locked ? 'unlock' : 'lock'} />
            {pass.locked ? 'Unlock' : 'Lock'} pass</Button
          >
          <Button
            class="menu-action"
            aria-label={`Lower ${pass.name}`}
            disabled={index === 0}
            onclick={() => editor.reorderPass(pass.id, -1)}
            ><Icon name="down" /> Lower in stack</Button
          >
          <Button
            class="menu-action"
            aria-label={`Raise ${pass.name}`}
            disabled={index === editor.doc.passes.length - 1}
            onclick={() => editor.reorderPass(pass.id, 1)}><Icon name="up" /> Raise in stack</Button
          >
          <Button
            class="menu-action danger"
            aria-label={`Delete ${pass.name}`}
            onclick={() => editor.removePass(pass.id)}><Icon name="trash" /> Delete pass</Button
          >
          <p class="fine-print">
            {pass.pieces.length} individual pieces · {pass.color.toUpperCase()}
          </p>
        </Popover>
      </div>
      {#if !compact && expanded.includes(pass.id)}<div class="layer-pieces">
          {#each pass.pieces as piece (piece.uid)}<Button
              class={editor.selected.includes(piece.uid) ? 'selected' : ''}
              aria-pressed={editor.selected.includes(piece.uid)}
              disabled={pass.locked || !pass.visible}
              onclick={(e) => {
                editor.tool = 'select';
                editor.mode = 'design';
                editor.select(piece.uid, e.shiftKey);
              }}
            >
              <Icon name="grid" size={14} /><span>{getPiece(piece.pieceId)?.name}</span><small
                >{piece.rotation}°</small
              >
            </Button>{/each}
          {#if !pass.pieces.length}<p class="fine-print">No pieces in this pass.</p>{/if}
        </div>{/if}
    </div>
  {/each}
  {#if !compact}
    <p class="fine-print">
      Expand a pass to select pieces. Shift-click or Shift-Enter to select several.
    </p>{/if}
</section>

<style>
  :global(.pass-grip) {
    cursor: grab;
    width: 24px;
    min-width: 24px;
  }
  .dragging {
    opacity: 0.5;
  }
  .drop-before {
    border-top: 2px solid var(--blue);
  }
  .drop-after {
    border-bottom: 2px solid var(--blue);
  }
</style>
