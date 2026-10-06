<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import Icon from './Icon.svelte';
  import { Input } from './input';
  import { Button } from './button';
  import Popover from './Popover.svelte';
  let { editor }: { editor: Editor } = $props();
  let color = $derived(editor.activePass.color);
  let rgb = $derived([1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16)));
  let recent = $derived([...new Set(editor.doc.passes.map((p) => p.color))].slice(-6));
  function setRgb(index: number, value: string) {
    const channels = [...rgb];
    channels[index] = Math.max(0, Math.min(255, Math.round(Number(value) || 0)));
    editor.setColor('#' + channels.map((n) => n.toString(16).padStart(2, '0')).join(''));
  }
</script>

<section class="property-section ink-panel" aria-label="Colors and inks">
  <h3>Colors / Inks</h3>
  <div class="swatches">
    {#each editor.doc.swatches as swatch (swatch)}<Button
        class={['ink-swatch', swatch.toLowerCase() === color.toLowerCase() && 'chosen']}
        style={`background: ${swatch}`}
        aria-pressed={swatch.toLowerCase() === color.toLowerCase()}
        aria-label={`Use ink ${swatch}`}
        title={swatch.toUpperCase()}
        onclick={() => editor.setColor(swatch)}
      ></Button>{/each}
    <Popover label="Custom ink color" className="icon-button swatch-add">
      {#snippet trigger()}<Icon name="plus" size={18} />{/snippet}
      <h3>Ink color</h3>
      <label class="field-label"
        >Color<input
          class="custom-color-well"
          type="color"
          aria-label="Choose custom ink color"
          value={color}
          onchange={(e) => editor.setColor(e.currentTarget.value)}
        /></label
      >
      <label class="field-label"
        >HEX<Input
          aria-label="Ink HEX color"
          value={color.toUpperCase()}
          maxlength={7}
          onchange={(e) => editor.setColor('#' + e.currentTarget.value.replace('#', ''))}
        /></label
      >
      <div class="dimensions-row rgb-fields">
        {#each ['R', 'G', 'B'] as channel, i (channel)}<label
            >{channel}<Input
              type="number"
              aria-label={`Ink ${channel} channel`}
              min="0"
              max="255"
              value={rgb[i]}
              onchange={(e) => setRgb(i, e.currentTarget.value)}
            /></label
          >{/each}
      </div>
      <Button
        class="outline-button full-width"
        aria-label="Save ink swatch"
        onclick={() =>
          editor.commit((doc) => {
            if (!doc.swatches.includes(color)) doc.swatches.push(color);
          })}><Icon name="plus" size={16} /> Save swatch</Button
      >
      <h4>Recent inks</h4>
      <div class="swatches">
        {#each recent as swatch (swatch)}<Button
            class="ink-swatch"
            style={`background: ${swatch}`}
            aria-label={`Recent ink ${swatch}`}
            onclick={() => editor.setColor(swatch)}
          ></Button>{/each}
      </div>
      <p class="fine-print">
        {editor.selected.length
          ? 'Color changes apply to your selection.'
          : 'Choose the ink for the next pieces you place.'}
      </p>
    </Popover>
  </div>
</section>
