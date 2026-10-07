<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import { canPlace } from '../../geometry/geometry';
  import TracingControls from './TracingControls.svelte';
  let { editor }: { editor: Editor } = $props();
  function overlap(value: boolean) {
    const doc = { ...editor.doc, options: { ...editor.doc.options, allowOverlap: value } };
    const items = editor.allPieces.map(({ piece }) => piece);
    if (!value && !canPlace(doc, items, new Set(items.map((p) => p.uid)))) {
      editor.notify('Separate overlapping pieces first.');
      return;
    }
    editor.commit((doc) => (doc.options.allowOverlap = value));
  }
</script>

<h3>Grid & composition</h3>
<label class="field-label"
  >Grid style<select
    aria-label="Grid style"
    value={editor.doc.options.grid}
    onchange={(e) =>
      editor.commit((doc) => (doc.options.grid = e.currentTarget.value as typeof doc.options.grid))}
  >
    <option value="off">Off</option><option value="points">Points</option><option value="lines"
      >Fine lines</option
    ><option value="squares">Stud squares</option></select
  ></label
>
<label class="field-label"
  >Grid appearance<select
    aria-label="Grid appearance"
    value={editor.gridAppearance}
    onchange={(e) => (editor.gridAppearance = e.currentTarget.value as 'flat' | 'embossed')}
    ><option value="flat">Flat</option><option value="embossed">Embossed studs</option></select
  ></label
>
<p class="fine-print">Embossed studs are an editor guide. Exports keep the flat grid.</p>
<label class="field-label"
  >Major lines<select
    aria-label="Major grid interval"
    value={String(editor.doc.options.majorGrid)}
    onchange={(e) =>
      editor.commit((doc) => (doc.options.majorGrid = Number(e.currentTarget.value)))}
    ><option value="4">Every 4 studs</option><option value="8">Every 8 studs</option></select
  ></label
>
<label class="field-label"
  >Symmetry<select
    aria-label="Symmetry"
    value={editor.doc.options.symmetry}
    onchange={(e) =>
      editor.commit(
        (doc) => (doc.options.symmetry = e.currentTarget.value as typeof doc.options.symmetry)
      )}
    ><option value="off">Off</option><option value="x">Mirror across X</option><option value="y"
      >Mirror across Y</option
    ><option value="both">4-way symmetry</option></select
  ></label
>
<label class="toggle-row"
  ><span>Allow overlap</span><input
    aria-label="Allow overlap"
    type="checkbox"
    checked={editor.doc.options.allowOverlap}
    onchange={(e) => {
      overlap(e.currentTarget.checked);
      e.currentTarget.checked = editor.doc.options.allowOverlap;
    }}
  /></label
>
<label class="toggle-row"
  ><span>Compatible printing heights only</span><input
    aria-label="Compatible printing heights only"
    aria-describedby="printing-height-help"
    type="checkbox"
    checked={editor.doc.options.physical}
    onchange={(e) => {
      editor.setPhysical(e.currentTarget.checked);
      e.currentTarget.checked = editor.doc.options.physical;
    }}
  /></label
>
<p id="printing-height-help" class="fine-print">
  Restricts placement to pieces with a standard printing height. This is a construction constraint;
  it does not change the preview.
</p>

<TracingControls {editor} />
