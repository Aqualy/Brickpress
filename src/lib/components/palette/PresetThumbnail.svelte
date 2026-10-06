<script lang="ts">
  import type { CompositionPreset } from '../../persistence/presets';
  import { getPiece } from '../../catalog/catalog';
  import { pieceTransform } from '../../geometry/geometry';
  import { designMaskId, designMaskMarkup } from '../../geometry/design-surface';
  let { preset }: { preset: CompositionPreset } = $props();
  let definitions = $derived(
    [...new Set(preset.passes.flatMap((pass) => pass.pieces.map((p) => p.pieceId)))]
      .map((id) => designMaskMarkup(getPiece(id)!, `preset-${preset.id}`))
      .join('')
  );
</script>

<svg
  class="preset-thumbnail"
  viewBox={`-.15 -.15 ${preset.width + 0.3} ${preset.height + 0.3}`}
  aria-hidden="true"
>
  <defs>{@html definitions}</defs>
  {#each preset.passes as pass, i (i)}
    {#each pass.pieces as placed (placed.uid)}
      {@const piece = getPiece(placed.pieceId)!}
      <path
        d={piece.geometry.path}
        transform={pieceTransform(piece, placed, 1)}
        fill={pass.color}
        fill-rule={piece.geometry.fillRule}
        mask={`url(#${designMaskId(piece, `preset-${preset.id}`)})`}
      />
    {/each}
  {/each}
</svg>
