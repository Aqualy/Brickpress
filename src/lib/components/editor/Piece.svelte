<script lang="ts">
  import { getPiece } from '../../catalog/catalog';
  import { pieceTransform } from '../../geometry/geometry';
  import { impression, impressionFilter } from '../../printing/print-engine';
  import type { PlacedPiece, PrintSettings, PaperSettings } from '../../types/document';
  let {
    placed,
    color,
    printed = false,
    settings,
    paper
  }: {
    placed: PlacedPiece;
    color: string;
    printed?: boolean;
    settings: PrintSettings;
    paper: PaperSettings;
  } = $props();
  let piece = $derived(getPiece(placed.pieceId));
  let transform = $derived(piece ? pieceTransform(piece, placed) : '');
  let effect = $derived(printed ? impression(placed, settings) : undefined);
  let filter = $derived(printed ? impressionFilter(placed, settings, paper) : '');
</script>

{#if piece}
  <g data-uid={placed.uid} transform={effect?.transform} opacity={effect?.opacity}>
    {#if printed}<defs>{@html filter}</defs>{/if}
    <path
      d={piece.geometry.path}
      {transform}
      fill={color}
      fill-rule={piece.geometry.fillRule}
      filter={effect ? `url(#${effect.filterId})` : undefined}
    />
  </g>
{/if}
