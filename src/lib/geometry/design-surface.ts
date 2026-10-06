import { catalogMeta, type LetterpressPiece } from '../catalog/catalog';

// A fixed inset keeps seams independent of footprint size and curve radius.
export const designGapStuds =
  catalogMeta.coordinateSystem.regularPieceGapMmNominal /
  catalogMeta.coordinateSystem.studPitchMmNominal;

export const designMaskId = (piece: LetterpressPiece, prefix = 'design') =>
  `${prefix}-gap-${piece.id}`;

/** Transparent inward offset of the canonical silhouette, shared by SVG previews and exports. */
export function designMaskMarkup(piece: LetterpressPiece, prefix = 'design'): string {
  const path = piece.geometry.path.replace(
    /[&<"]/g,
    (c) => ({ '&': '&amp;', '<': '&lt;', '"': '&quot;' })[c]!
  );
  // Half of the centered black stroke removes ink inside each edge. The other
  // half is already outside the white fill, including along concave cutouts.
  return `<mask id="${designMaskId(piece, prefix)}" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" x="0" y="0" width="${piece.footprint.widthStuds}" height="${piece.footprint.heightStuds}" mask-type="luminance"><path d="${path}" fill="white" fill-rule="${piece.geometry.fillRule}" stroke="black" stroke-width="${designGapStuds}" stroke-linejoin="round"/></mask>`;
}
