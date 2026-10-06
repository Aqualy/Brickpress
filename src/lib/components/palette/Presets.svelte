<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import { downloadBlob, filename } from '../../export/export';
  import type { CompositionPreset } from '../../persistence/presets';
  import { Button } from '../ui/button';
  import { Input } from '../ui/input';
  import Popover from '../ui/Popover.svelte';
  import Icon from '../ui/Icon.svelte';
  import PresetThumbnail from './PresetThumbnail.svelte';
  let { editor }: { editor: Editor } = $props();
  let query = $state(''),
    name = $state(''),
    saveOpen = $state(false);
  let source = $derived(editor.selected.length ? 'selection' : 'canvas');
  let fileInput: HTMLInputElement;
  let filtered = $derived(
    editor.presets.filter((preset) => preset.name.toLowerCase().includes(query.toLowerCase()))
  );
  function save(event: SubmitEvent) {
    event.preventDefault();
    if (editor.savePreset(name, source === 'selection')) {
      name = '';
      saveOpen = false;
    }
  }
  function exportPresets(presets: CompositionPreset[]) {
    downloadBlob(
      new Blob([JSON.stringify(presets, null, 2)], { type: 'application/json' }),
      `${presets.length === 1 ? filename(presets[0].name) : 'brickpress-library'}.brickpress-presets.json`
    );
  }
  async function importFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement,
      file = input.files?.[0];
    if (!file) return;
    try {
      if (file.size > 20_000_000) throw new Error('Preset files must be smaller than 20 MB.');
      editor.importPresets(await file.text());
    } catch (e) {
      editor.notify(e instanceof Error ? e.message : 'The preset file could not be opened.');
    }
    input.value = '';
  }
  function dragstart(event: DragEvent, id: string) {
    editor.choosePreset(id, false);
    event.dataTransfer?.setData('application/x-brickpress-preset', id);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
  }
</script>

<div class="preset-library">
  <input
    class="hidden-input"
    bind:this={fileInput}
    type="file"
    accept=".json"
    aria-label="Import composition presets file"
    onchange={importFile}
  />
  <label class="search-field"
    ><Icon name="search" size={16} /><Input
      aria-label="Search presets"
      placeholder="Search presets…"
      bind:value={query}
    /></label
  >
  <div class="preset-actions">
    <Popover
      label="Save composition preset"
      bind:open={saveOpen}
      width={280}
      className="preset-save-button"
    >
      {#snippet trigger()}<Icon name="plus" size={16} /><span>Save preset</span>{/snippet}
      <form onsubmit={save}>
        <h3>Save composition</h3>
        <label class="field-label"
          >Name<Input
            aria-label="Preset name"
            placeholder="e.g. Flower"
            maxlength={80}
            bind:value={name}
            required
          /></label
        >
        <label class="field-label"
          >Include<select aria-label="Preset source" bind:value={source}>
            <option value="canvas">All visible pieces</option><option
              value="selection"
              disabled={!editor.selected.length}>Selected pieces ({editor.selected.length})</option
            >
          </select></label
        >
        <p class="fine-print">
          Keeps ink colors, rotations and spacing. Empty canvas margins are trimmed.
        </p>
        <Button
          type="submit"
          disabled={!name.trim() || !editor.allPieces.some((p) => p.pass.visible)}
          >Save preset</Button
        >
      </form>
    </Popover>
    <Button
      class="icon-button"
      aria-label="Import presets"
      title="Import presets"
      onclick={() => fileInput.click()}><Icon name="open" size={17} /></Button
    >
    <Button
      class="icon-button"
      aria-label="Export preset library"
      title="Export preset library"
      disabled={!editor.presets.length}
      onclick={() => exportPresets(editor.presets)}><Icon name="export" size={17} /></Button
    >
  </div>
  <p class="fine-print">
    Drag onto the canvas, or choose a preset and use arrows + Enter. <kbd>R</kbd> rotates. Canvas size
    stays unchanged.
  </p>
  <div class="preset-grid">
    {#each filtered as preset (preset.id)}
      <article class="preset-entry">
        <button
          class="preset-card"
          class:active={editor.activePresetId === preset.id && editor.tool === 'place'}
          draggable="true"
          ondragstart={(event) => dragstart(event, preset.id)}
          onclick={() => editor.choosePreset(preset.id)}
          aria-label={`Place preset ${preset.name}, ${preset.width} by ${preset.height} studs`}
          aria-pressed={editor.activePresetId === preset.id && editor.tool === 'place'}
        >
          <PresetThumbnail {preset} />
          <strong>{preset.name}</strong><small
            >{preset.width}×{preset.height} · {preset.passes.reduce(
              (n, p) => n + p.pieces.length,
              0
            )} pieces</small
          >
        </button>
        <Popover
          label={`Preset actions for ${preset.name}`}
          className="icon-button preset-more"
          width={270}
        >
          {#snippet trigger()}<Icon name="more" size={15} />{/snippet}
          <h3>Preset actions</h3>
          <label class="field-label"
            >Name<Input
              aria-label={`Rename preset ${preset.name}`}
              maxlength={80}
              value={preset.name}
              onchange={(e) => editor.renamePreset(preset.id, e.currentTarget.value)}
            /></label
          >
          <Button class="menu-action" onclick={() => exportPresets([preset])}
            ><Icon name="export" size={16} /> Export preset</Button
          >
          <Button class="menu-action danger" onclick={() => editor.deletePreset(preset.id)}
            ><Icon name="trash" size={16} /> Delete preset</Button
          >
        </Popover>
      </article>
    {/each}
  </div>
  {#if !filtered.length}<p class="empty-state">
      {editor.presets.length
        ? 'No matching presets.'
        : 'Save a composition or selected pieces to reuse them in any project.'}
    </p>{/if}
  <p class="fine-print">Saved in this browser. Export the library to use it on another device.</p>
</div>
