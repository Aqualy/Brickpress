<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import Icon from './Icon.svelte';
  let {
    editor,
    location = 'canvas',
    fit = false
  }: { editor: Editor; location?: string; fit?: boolean } = $props();
</script>

<div class="zoom-controls" role="group" aria-label={`${location} zoom controls`}>
  <button
    class="icon-button"
    aria-label={`${location} zoom out`}
    onclick={() => editor.setZoom(editor.zoomPercent / 1.2)}><Icon name="minus" size={19} /></button
  >
  <button
    class="zoom-value"
    aria-label={`${location} zoom ${editor.zoomPercent}%, reset to 100%`}
    onclick={() => editor.setZoom(100)}>{editor.zoomPercent}%</button
  >
  <button
    class="icon-button"
    aria-label={`${location} zoom in`}
    onclick={() => editor.setZoom(editor.zoomPercent * 1.2)}><Icon name="plus" size={19} /></button
  >
</div>
{#if fit}<button
    class="fit-button"
    title="Fit artboard"
    aria-label="Fit artboard"
    onclick={() => editor.setZoom('fit')}><Icon name="fit" size={20} /></button
  >{/if}
