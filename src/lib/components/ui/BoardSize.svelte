<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import Popover from './Popover.svelte';
  import Icon from './Icon.svelte';
  let { editor, compact = false }: { editor: Editor; compact?: boolean } = $props();
  let width = $derived(editor.doc.board.width),
    height = $derived(editor.doc.board.height);
  const sizes = [8, 12, 16, 24, 32, 48];
</script>

<Popover
  label={compact ? 'Canvas size' : 'Artboard size'}
  placement={compact ? 'above' : 'below'}
  className="size-trigger"
>
  {#snippet trigger()}<span
      >{editor.doc.board.width} × {editor.doc.board.height} studs{#if compact}<small>
          · {editor.doc.board.width * 8} × {editor.doc.board.height * 8} mm</small
        >{/if}</span
    ><Icon name="chevron" size={16} />{/snippet}
  <h3>Artboard size</h3>
  <label class="field-label"
    >Preset<select
      aria-label="Artboard preset"
      value={editor.doc.board.width === editor.doc.board.height &&
      sizes.includes(editor.doc.board.width)
        ? String(editor.doc.board.width)
        : 'custom'}
      onchange={(e) => {
        if (e.currentTarget.value !== 'custom')
          editor.resize(Number(e.currentTarget.value), Number(e.currentTarget.value));
        e.currentTarget.value =
          editor.doc.board.width === editor.doc.board.height &&
          sizes.includes(editor.doc.board.width)
            ? String(editor.doc.board.width)
            : 'custom';
      }}
      >{#each sizes as size (size)}<option value={size}>{size} × {size} studs</option>{/each}<option
        value="custom">Custom size</option
      ></select
    ></label
  >
  <div class="dimensions-row">
    <label
      >Width<input
        aria-label="Custom board width"
        type="number"
        min="1"
        max="128"
        bind:value={width}
      /></label
    ><label
      >Height<input
        aria-label="Custom board height"
        type="number"
        min="1"
        max="128"
        bind:value={height}
      /></label
    >
  </div>
  <button class="outline-button full-width" onclick={() => editor.resize(width, height)}
    >Apply size</button
  >
  <p class="fine-print">
    {editor.doc.board.width * 8} × {editor.doc.board.height * 8} mm · nominal 8 mm stud pitch
  </p>
</Popover>
