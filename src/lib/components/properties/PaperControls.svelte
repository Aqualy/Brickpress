<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import { paperPresets } from '../../printing/settings';
  import BoardSize from '../ui/BoardSize.svelte';
  import Popover from '../ui/Popover.svelte';
  import Icon from '../ui/Icon.svelte';
  let { editor }: { editor: Editor } = $props();
  const textureId = $props.id();
</script>

<section class="property-section paper-controls">
  <h3>Paper</h3>
  <label class="paper-stock"
    ><svg class="paper-thumbnail" viewBox="0 0 42 42" aria-hidden="true"
      ><defs
        ><filter id={textureId}
          ><feTurbulence
            type="fractalNoise"
            baseFrequency=".8"
            numOctaves="3"
            seed={editor.doc.paper.seed % 65536}
          /><feColorMatrix type="saturate" values="0" /></filter
        ></defs
      ><rect width="42" height="42" fill={editor.doc.paper.color} /><rect
        width="42"
        height="42"
        filter={`url(#${textureId})`}
        opacity={editor.doc.paper.grain * 0.4}
      /></svg
    ><select
      aria-label="Paper stock"
      value={editor.doc.paper.preset}
      onchange={(e) =>
        editor.commit(
          (doc) => (doc.paper = { ...paperPresets[e.currentTarget.value], seed: doc.paper.seed })
        )}
      >{#each Object.keys(paperPresets) as name (name)}<option>{name}</option>{/each}</select
    ></label
  >
  <div class="paper-size-row"><span>Size</span><BoardSize {editor} /></div>
  <Popover label="Paper texture and color" className="text-button paper-details-button">
    {#snippet trigger()}<Icon name="settings" size={15} /><span>Paper texture & color</span
      >{/snippet}
    <h3>Paper appearance</h3>
    <label class="field-label"
      >Paper color<input
        type="color"
        aria-label="Paper color"
        value={editor.doc.paper.color}
        onchange={(e) => editor.commit((doc) => (doc.paper.color = e.currentTarget.value))}
      /></label
    >
    {#each [{ key: 'grain', name: 'Paper grain' }, { key: 'tooth', name: 'Paper tooth texture' }, { key: 'fibers', name: 'Fiber amount' }, { key: 'brightness', name: 'Brightness' }] as control (control.key)}
      {@const key = control.key as 'grain' | 'tooth' | 'fibers' | 'brightness'}
      <label class="slider-field"
        ><span>{control.name}<output>{Math.round(editor.doc.paper[key] * 100)}%</output></span
        ><input
          aria-label={control.name}
          type="range"
          min={key === 'brightness' ? 0.7 : 0}
          max={key === 'brightness' ? 1.2 : 1}
          step=".01"
          value={editor.doc.paper[key]}
          style:--progress={`${((editor.doc.paper[key] - (key === 'brightness' ? 0.7 : 0)) / (key === 'brightness' ? 0.5 : 1)) * 100}%`}
          onchange={(e) => editor.commit((doc) => (doc.paper[key] = Number(e.currentTarget.value)))}
        /></label
      >
    {/each}
  </Popover>
</section>
