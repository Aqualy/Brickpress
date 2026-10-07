<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import { pngBlob, svgBlob, filename, exportDimensions } from '../../export/export';
  import Icon from '../ui/Icon.svelte';
  import { Button } from '../ui/button';
  let { editor }: { editor: Editor } = $props();
  let busy = $state(false),
    error = $state(''),
    previewUrl = $state(''),
    previewError = $state('');
  let options = $derived(editor.exportOptions);
  let format = $derived(options.format),
    paper = $derived(options.paper),
    grid = $derived(options.grid);
  let mode = $derived(options.mode ?? editor.mode);
  let width = $derived(
    options.scale === 'custom'
      ? Number(options.custom)
      : editor.doc.board.width * 64 * Number(options.scale)
  );
  let height = $derived(Math.round((width * editor.doc.board.height) / editor.doc.board.width));
  let validation = $derived.by(() => {
    if (format === 'json') return '';
    try {
      exportDimensions(editor.doc, width);
      return '';
    } catch (e) {
      return e instanceof Error ? e.message : 'Invalid export size.';
    }
  });
  $effect(() => {
    if (editor.inspectorTab !== 'export' || (editor.narrow && editor.drawer !== 'inspector')) {
      previewUrl = '';
      return;
    }
    const doc = editor.doc;
    const config = {
      mode: format === 'json' ? ('design' as const) : mode,
      paper: format === 'json' ? false : paper,
      grid: format === 'json' ? false : grid,
      width: format === 'json' ? 640 : width
    };
    const invalid = validation;
    // Object URLs need explicit lifetime management; release them when options change.
    previewUrl = '';
    let url = '';
    const timer = setTimeout(() => {
      previewError = invalid;
      if (invalid) return;
      try {
        url = URL.createObjectURL(svgBlob(doc, config));
        previewUrl = url;
      } catch (e) {
        previewError = e instanceof Error ? e.message : 'Preview unavailable.';
      }
    }, 120);
    return () => {
      clearTimeout(timer);
      if (url) URL.revokeObjectURL(url);
    };
  });
  async function run() {
    busy = true;
    error = '';
    try {
      let completed = false;
      const snapshot = editor.projectText();
      if (format === 'json') {
        completed = await editor.exportBlob(
          'project',
          new Blob([editor.projectText()], { type: 'application/json' }),
          `${filename(editor.doc.name)}.brickpress.json`
        );
        if (completed && !editor.desktop && editor.projectText() === snapshot) editor.saved();
      } else {
        const doc = editor.doc,
          name = `${filename(doc.name)}-${mode}.${format}`;
        const blob =
          format === 'svg'
            ? svgBlob(doc, { mode, paper, width, grid })
            : await pngBlob(doc, { mode, paper, width, grid });
        completed = await editor.exportBlob(format === 'svg' ? 'svg' : 'png', blob, name);
      }
      if (completed) editor.notify('Export ready.');
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
        onclick={() => (options.format = item.id)}
        ><strong>{item.name}</strong><span>{item.caption}</span></Button
      >{/each}
  </div>
  <figure
    class="export-preview"
    aria-label={format === 'json' ? 'Project composition preview' : 'Export preview'}
  >
    <div class="export-preview-image" class:transparent={!paper || format === 'json'}>
      {#if previewError}<p role="status">{previewError}</p>
      {:else if previewUrl}<img
          src={previewUrl}
          alt={`${editor.doc.name} — ${format === 'json' ? 'editable composition' : mode === 'print' ? 'letterpress export' : 'design export'} preview`}
          onerror={() => (previewError = 'The export preview could not be rendered.')}
        />
      {:else}<p>Preparing preview…</p>{/if}
    </div>
    <figcaption>
      {format === 'json'
        ? `${editor.allPieces.length} pieces · ${editor.doc.passes.length} ink passes`
        : `${mode === 'print' ? 'Print impression' : 'Vector design'} · ${paper ? 'Paper included' : 'Transparent'}`}
    </figcaption>
  </figure>
  {#if format !== 'json'}
    <label class="field-label"
      >Artwork<select
        aria-label="Export artwork mode"
        value={mode}
        onchange={(e) => (options.mode = e.currentTarget.value as 'design' | 'print')}
        ><option value="print">Letterpress impression</option><option value="design"
          >Clean vector design</option
        ></select
      ></label
    >
    <label class="field-label"
      >Resolution<select aria-label="Export resolution" bind:value={options.scale}
        ><option value="1">1× · 64 pixels / stud</option><option value="2"
          >2× · 128 pixels / stud</option
        ><option value="4">4× · 256 pixels / stud</option><option value="custom"
          >Custom pixel width</option
        ></select
      ></label
    >
    {#if options.scale === 'custom'}<label class="field-label"
        >Pixel width<input
          aria-label="Export pixel width"
          type="number"
          min="64"
          max="12000"
          bind:value={options.custom}
        /></label
      >{/if}
    <div class="export-dimensions mono">
      {Number.isFinite(width) ? width.toLocaleString() : '—'} × {Number.isFinite(height)
        ? height.toLocaleString()
        : '—'} PX
      <span>{Number.isFinite(width * height) ? ((width * height) / 1e6).toFixed(1) : '—'} MP</span>
    </div>
    <label class="toggle-row"
      ><span>Include paper background</span><input
        aria-label="Include paper background"
        type="checkbox"
        bind:checked={options.paper}
      /></label
    >
    <label class="toggle-row"
      ><span>Include grid</span><input
        aria-label="Include grid in export"
        type="checkbox"
        bind:checked={options.grid}
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
  <Button class="export-button dialog-export" disabled={busy || !!validation} onclick={run}
    ><Icon name="export" size={16} />{busy
      ? 'Making your impression…'
      : `Export ${format === 'json' ? 'project' : format.toUpperCase()}`}</Button
  >
</section>

<style>
  .export-preview {
    margin: 12px 0;
  }
  .export-preview-image {
    display: grid;
    place-items: center;
    min-height: 120px;
    height: 210px;
    padding: 8px;
    background: #e9ebef;
    border: 1px solid var(--line);
    border-radius: 4px;
    overflow: hidden;
  }
  .export-preview-image.transparent {
    background-color: white;
    background-image: conic-gradient(#e4e7eb 25%, white 0 50%, #e4e7eb 0 75%, white 0);
    background-size: 16px 16px;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    min-height: 0;
  }
  .export-preview-image p {
    font-size: 12px;
    background: white;
    padding: 8px;
  }
  figcaption {
    font-size: 11px;
    color: var(--muted);
    margin-top: 6px;
  }
</style>
