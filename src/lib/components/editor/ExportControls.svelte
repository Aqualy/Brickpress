<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import { exportPng, exportSvg, downloadBlob, filename } from '../../export/export';
  import Icon from '../ui/Icon.svelte';
  import { Button } from '../ui/button';
  let { editor }: { editor: Editor } = $props();
  let format = $state('png'),
    paper = $state(true),
    grid = $state(false),
    scale = $state('2'),
    custom = $state(2048),
    busy = $state(false),
    error = $state('');
  let mode = $derived(editor.mode);
  let width = $derived(scale === 'custom' ? custom : editor.doc.board.width * 64 * Number(scale));
  let height = $derived(Math.round((width * editor.doc.board.height) / editor.doc.board.width));
  async function run() {
    busy = true;
    error = '';
    try {
      if (format === 'json') {
        downloadBlob(
          new Blob([editor.projectText()], { type: 'application/json' }),
          `${filename(editor.doc.name)}.brickpress.json`
        );
        editor.saved();
      } else if (format === 'svg') exportSvg(editor.doc, { mode, paper, width, grid });
      else await exportPng(editor.doc, { mode, paper, width, grid });
      editor.notify('Export ready.');
    } catch (e) {
      error = e instanceof Error ? e.message : 'Export failed.';
    } finally {
      busy = false;
    }
  }
</script>

<section class="export-panel property-section" aria-label="Export settings">
  <div class="dialog-title">
    <div>
      <h3>Export artwork</h3>
    </div>
  </div>
  <p class="subtle-text">Download your print, vector artwork, or editable project.</p>
  <div class="export-formats">
    {#each [{ id: 'png', name: 'PNG', caption: 'High-resolution print' }, { id: 'svg', name: 'SVG', caption: 'Scalable artwork' }, { id: 'json', name: 'Project', caption: 'Editable composition' }] as item (item.id)}<Button
        class={format === item.id ? 'active' : ''}
        aria-pressed={format === item.id}
        onclick={() => (format = item.id)}
        ><strong>{item.name}</strong><span>{item.caption}</span></Button
      >{/each}
  </div>
  {#if format !== 'json'}
    <label class="field-label"
      >Artwork<select aria-label="Export artwork mode" bind:value={mode}
        ><option value="print">Letterpress impression</option><option value="design"
          >Clean vector design</option
        ></select
      ></label
    >
    <label class="field-label"
      >Resolution<select aria-label="Export resolution" bind:value={scale}
        ><option value="1">1× · 64 pixels / stud</option><option value="2"
          >2× · 128 pixels / stud</option
        ><option value="4">4× · 256 pixels / stud</option><option value="custom"
          >Custom pixel width</option
        ></select
      ></label
    >
    {#if scale === 'custom'}<label class="field-label"
        >Pixel width<input
          aria-label="Export pixel width"
          type="number"
          min="64"
          max="12000"
          bind:value={custom}
        /></label
      >{/if}
    <div class="export-dimensions mono">
      {width.toLocaleString()} × {height.toLocaleString()} PX
      <span>{((width * height) / 1e6).toFixed(1)} MP</span>
    </div>
    <label class="toggle-row"
      ><span>Include paper background</span><input
        aria-label="Include paper background"
        type="checkbox"
        bind:checked={paper}
      /></label
    >
    <label class="toggle-row"
      ><span>Include grid</span><input
        aria-label="Include grid in export"
        type="checkbox"
        bind:checked={grid}
      /></label
    >
    <p class="fine-print">
      {paper
        ? 'Paper color and procedural fibers are included.'
        : 'Transparent background. Only individual ink impressions.'}
      {format === 'svg' && mode === 'print'
        ? 'SVG keeps seeded procedural filters; rendering may vary between vector applications.'
        : ''}
    </p>
  {:else}<p class="project-export-note">
      <Icon name="layers" size={24} />All pieces, ink passes, seeds, paper, and press settings are
      saved in an editable .brickpress.json file.
    </p>{/if}
  {#if error}<p class="export-error" role="alert">{error}</p>{/if}
  <Button class="export-button dialog-export" disabled={busy} onclick={run}
    ><Icon name="export" size={16} />{busy
      ? 'Making your impression…'
      : `Export ${format === 'json' ? 'project' : format.toUpperCase()}`}</Button
  >
</section>
