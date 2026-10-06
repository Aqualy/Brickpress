import raw from '../../../lego-letterpress-kit/lego-letterpress-kit/letterpress-pieces.json' with { type: 'json' };
import type { LetterpressPiece } from '../../../lego-letterpress-kit/lego-letterpress-kit/letterpress-pieces';
export type { LetterpressPiece };

export function parsePieces(input: unknown[]): LetterpressPiece[] {
  const seen = new Set<string>();
  return input.filter((entry): entry is LetterpressPiece => {
    const p = entry as LetterpressPiece;
    const valid =
      typeof p?.id === 'string' &&
      !seen.has(p.id) &&
      typeof p.name === 'string' &&
      ['rectilinear', 'round', 'curve', 'wedge', 'special'].includes(p.category) &&
      p.footprint?.widthStuds > 0 &&
      p.footprint?.heightStuds > 0 &&
      typeof p.geometry?.path === 'string' &&
      p.geometry.path.length > 0 &&
      p.geometry.viewBox?.length === 4 &&
      p.geometry.viewBox.every(Number.isFinite) &&
      ['nonzero', 'evenodd'].includes(p.geometry.fillRule) &&
      Array.isArray(p.allowedRotations) &&
      p.allowedRotations.length > 0 &&
      p.allowedRotations.every((r) => [0, 90, 180, 270].includes(r)) &&
      Array.isArray(p.gridMask) &&
      p.gridMask.length === p.footprint.heightStuds &&
      p.gridMask.every(
        (row) => row.length === p.footprint.widthStuds && row.every((n) => n === 0 || n === 1)
      );
    if (!valid) console.warn('Omitting malformed letterpress catalog entry:', p?.id ?? 'unknown');
    else seen.add(p.id);
    return valid;
  });
}
export const pieces = parsePieces(raw.pieces);
export const pieceMap = new Map(pieces.map((p) => [p.id, p]));
export const catalogMeta = raw;
export const surfaceScale = raw.coordinateSystem.defaultSurfaceScale;
export const getPiece = (id: string) => pieceMap.get(id);
export const isPhysical = (p: LetterpressPiece) =>
  p.physicalLetterpressReady && p.surfaceHeightPlates === 1;
export const categories = [
  { id: 'rectilinear', name: 'Basic' },
  { id: 'round', name: 'Round' },
  { id: 'curve', name: 'Curves' },
  { id: 'wedge', name: 'Angular' },
  { id: 'special', name: 'Special' }
];
