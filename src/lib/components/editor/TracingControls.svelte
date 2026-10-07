<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import { Button } from '../ui/button';
  import Icon from '../ui/Icon.svelte';
  let { editor }: { editor: Editor } = $props();
  let fileInput: HTMLInputElement;
  async function upload(event: Event) {
    const input = event.currentTarget as HTMLInputElement,
      file = input.files?.[0];
    if (file) await editor.uploadTrace(file);
    input.value = '';
  }
  function coordinate(event: Event, key: 'x' | 'y' | 'width') {
    const input = event.currentTarget as HTMLInputElement;
    editor.updateTrace({ [key]: input.valueAsNumber });
    if (editor.trace) input.value = String(editor.trace[key]);
  }
</script>

<section class="tracing-controls" aria-label="Tracing image controls">
  <h3>Tracing image</h3>
  <input
    class="hidden-input"
    bind:this={fileInput}
    type="file"
    accept="image/png,image/jpeg,image/webp,image/gif,image/avif,image/bmp"
    aria-label="Upload tracing image file"
    onchange={upload}
  />
  {#if editor.trace}
    <div class="trace-summary">
      <img src={editor.trace.url} alt="" /><span title={editor.trace.name}>{editor.trace.name}</span
      >
    </div>
    <label class="toggle-row"
      ><span>Show tracing image</span><input
        type="checkbox"
        checked={editor.trace.visible}
        onchange={(e) => editor.updateTrace({ visible: e.currentTarget.checked })}
      /></label
    >
    <label class="slider-field"
      ><span>Opacity<output>{Math.round(editor.trace.opacity * 100)}%</output></span><input
        aria-label="Tracing image opacity"
        type="range"
        min="0"
        max="100"
        step="1"
        value={editor.trace.opacity * 100}
        style:--progress={`${editor.trace.opacity * 100}%`}
        oninput={(e) => editor.updateTrace({ opacity: Number(e.currentTarget.value) / 100 })}
      /></label
    >
    <label class="toggle-row"
      ><span>Lock tracing position</span><input
        type="checkbox"
        checked={editor.trace.locked}
        onchange={(e) => editor.updateTrace({ locked: e.currentTarget.checked })}
      /></label
    >
    <div class="trace-coordinates">
      <label class="field-label"
        >X<input
          aria-label="Tracing X in studs"
          type="number"
          min="-1024"
          max="1024"
          step="0.1"
          value={editor.trace.x}
          disabled={editor.trace.locked}
          onchange={(e) => coordinate(e, 'x')}
        /></label
      >
      <label class="field-label"
        >Y<input
          aria-label="Tracing Y in studs"
          type="number"
          min="-1024"
          max="1024"
          step="0.1"
          value={editor.trace.y}
          disabled={editor.trace.locked}
          onchange={(e) => coordinate(e, 'y')}
        /></label
      >
      <label class="field-label"
        >Width<input
          aria-label="Tracing width in studs"
          type="number"
          min="0.1"
          max="1024"
          step="0.1"
          value={editor.trace.width}
          disabled={editor.trace.locked}
          onchange={(e) => coordinate(e, 'width')}
        /></label
      >
    </div>
    <p class="fine-print">Stud units · proportions stay locked.</p>
    <div class="trace-placement-actions">
      <Button disabled={editor.trace.locked} onclick={() => editor.fitTrace()}>Fit to canvas</Button
      ><Button disabled={editor.trace.locked} onclick={() => editor.centerTrace()}>Center</Button>
    </div>
  {/if}
  <div class="trace-placement-actions">
    <Button
      disabled={editor.traceLoading}
      onclick={() => (editor.desktop ? editor.chooseTracingImage() : fileInput.click())}
      ><Icon name="open" size={16} />
      {editor.traceLoading
        ? 'Loading image…'
        : editor.trace
          ? 'Replace image'
          : 'Add tracing image'}</Button
    >
    {#if editor.trace}<Button
        class="danger"
        aria-label="Remove tracing image"
        onclick={() => editor.removeTrace()}><Icon name="trash" size={16} /></Button
      >{/if}
  </div>
  <p class="fine-print">
    A guide beneath your pieces, saved on this device. Hidden in Print Preview and excluded from
    project and artwork exports.
  </p>
</section>
