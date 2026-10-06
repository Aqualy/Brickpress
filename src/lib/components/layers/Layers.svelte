<script lang="ts">
  import { getPiece } from '../../catalog/catalog';
  import type { Editor } from '../../stores/editor.svelte';
  import Icon from '../ui/Icon.svelte';
  import { Input } from '../ui/input';
  import { Button } from '../ui/button';
  import Popover from '../ui/Popover.svelte';
  let { editor, compact = false }: { editor: Editor; compact?: boolean } = $props();
  let expanded = $state.raw<string[]>([]);
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
    <div class="pass-block" class:active={editor.activePassId === pass.id}>
      <div class="pass-row">
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
  {#if !compact}<Button class="text-button add-pass" onclick={() => editor.addPass()}
      ><Icon name="plus" size={16} /> Add ink pass</Button
    >
    <p class="fine-print">
      Expand a pass to select pieces. Shift-click or Shift-Enter to select several.
    </p>{/if}
</section>
