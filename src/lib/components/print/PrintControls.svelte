<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import { printControls, printPresets } from '../../printing/settings';
  import { catalogMeta } from '../../catalog/catalog';
  import Icon from '../ui/Icon.svelte';
  let { editor }: { editor: Editor } = $props();
  let presetName = $state('');
  const sameSettings = (a: typeof editor.doc.printSettings, b: typeof editor.doc.printSettings) =>
    printControls.every((c) => a[c.key] === b[c.key]);
  let preset = $derived(
    editor.customPrintPresets.find((row) => sameSettings(row.settings, editor.doc.printSettings))
      ?.id ??
      Object.entries(printPresets).find(([, settings]) =>
        sameSettings(settings, editor.doc.printSettings)
      )?.[0] ??
      'Custom'
  );
  const primary = ['inkAmount', 'pressure', 'paperTooth', 'registrationError'];
  const ordered = primary.map((key) => printControls.find((c) => c.key === key)!);
  const mmAmplitude =
    catalogMeta.rendering.recommendedPrintSimulation.registrationErrorForMultiColorStuds[1] *
    3 *
    catalogMeta.coordinateSystem.studPitchMmNominal;
  function output(key: (typeof printControls)[number]['key']) {
    const value = editor.doc.printSettings[key];
    return key === 'registrationError'
      ? (value * mmAmplitude).toFixed(2) + ' mm'
      : key === 'textureScale'
        ? value.toFixed(1) + '×'
        : Math.round(value * 100) + '%';
  }
</script>

{#snippet slider(control: (typeof printControls)[number])}
  <label class="slider-field"
    ><span>{control.label}<output>{output(control.key)}</output></span><input
      aria-label={control.label}
      aria-valuetext={control.key === 'registrationError'
        ? `Maximum offset per axis ${output(control.key)}`
        : output(control.key)}
      type="range"
      min={control.min ?? 0}
      max={control.max ?? 1}
      step=".01"
      value={editor.doc.printSettings[control.key]}
      style:--progress={`${((editor.doc.printSettings[control.key] - (control.min ?? 0)) / ((control.max ?? 1) - (control.min ?? 0))) * 100}%`}
      onchange={(e) =>
        editor.commit((doc) => (doc.printSettings[control.key] = Number(e.currentTarget.value)))}
    /></label
  >
{/snippet}
<section class="property-section print-controls">
  <h3>Print Settings</h3>
  <label class="field-label"
    >Press preset<select
      aria-label="Print preset"
      value={preset}
      onchange={(e) => {
        const settings =
          editor.customPrintPresets.find((row) => row.id === e.currentTarget.value)?.settings ??
          printPresets[e.currentTarget.value];
        if (settings) editor.commit((doc) => (doc.printSettings = { ...settings }));
      }}
    >
      <option disabled value="Custom">Custom</option>
      <optgroup label="Built-in presets"
        >{#each Object.keys(printPresets) as name (name)}<option>{name}</option>{/each}</optgroup
      >
      {#if editor.customPrintPresets.length}<optgroup label="Your presets"
          >{#each editor.customPrintPresets as row (row.id)}<option value={row.id}
              >{row.name}</option
            >{/each}</optgroup
        >{/if}
    </select></label
  >
  <form
    class="preset-save-row"
    onsubmit={(e) => {
      e.preventDefault();
      editor.savePrintPreset(presetName);
      presetName = '';
    }}
  >
    <input
      aria-label="Print preset name"
      placeholder="Name this print preset…"
      maxlength="80"
      bind:value={presetName}
    />
    <button class="outline-button" type="submit" disabled={!presetName.trim()}
      ><Icon name="save" size={16} />Save</button
    >
  </form>
  {#if editor.customPrintPresets.some((row) => row.id === preset)}<button
      class="text-button danger"
      onclick={() => {
        editor.customPrintPresets = editor.customPrintPresets.filter((row) => row.id !== preset);
      }}>Delete saved print preset</button
    >{/if}
  <p class="fine-print">
    Saves all eleven press controls on this device. Use the same name to replace a preset.
  </p>
  {#each ordered as control (control.key)}{@render slider(control)}{/each}
  <details class="advanced-print">
    <summary>Advanced print settings</summary>
    {#each printControls.filter((c) => !primary.includes(c.key)) as control (control.key)}{@render slider(
        control
      )}{/each}
    <button class="outline-button full-width" onclick={() => editor.reseed()}
      ><Icon name="seed" size={18} /> Reseed print</button
    >
    <p class="fine-print">
      Registration is the maximum offset per axis at the nominal stud pitch. Seeds stay fixed until
      you reseed; reseeding can be undone.
    </p>
  </details>
</section>

<style>
  .preset-save-row {
    display: flex;
    gap: 6px;
    margin-bottom: 8px;
  }
  .preset-save-row input {
    min-width: 0;
    flex: 1;
  }
  .preset-save-row button {
    flex-shrink: 0;
    display: flex;
    gap: 4px;
    align-items: center;
  }
</style>
