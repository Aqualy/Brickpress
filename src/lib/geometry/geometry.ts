import { getPiece, surfaceScale, type LetterpressPiece } from '../catalog/catalog';
import type { PlacedPiece, PressDocument } from '../types/document';
import { surfacesOverlap } from './silhouette';

const maskCache = new Map<string, number[][]>();
export const normalizeRotation = (r: number) => ((r % 360) + 360) % 360;
export function rotateMask(mask: number[][], rotation: number): number[][] {
  let result = mask.map((row) => [...row]);
  for (let i = 0; i < normalizeRotation(rotation) / 90; i++) {
    result = Array.from({ length: result[0].length }, (_, y) =>
      result.map((row) => row[y]).reverse()
    );
  }
  return result;
}
export function footprint(piece: LetterpressPiece, rotation: number) {
  const swap = normalizeRotation(rotation) % 180 !== 0;
  return {
    width: swap ? piece.footprint.heightStuds : piece.footprint.widthStuds,
    height: swap ? piece.footprint.widthStuds : piece.footprint.heightStuds
  };
}
export function gridMask(p: PlacedPiece): number[][] {
  const key = `${p.pieceId}:${p.rotation}:${!!p.mirrorX}:${!!p.mirrorY}`;
  if (maskCache.has(key)) return maskCache.get(key)!;
  const piece = getPiece(p.pieceId);
  if (!piece) return [];
  let mask = rotateMask(piece.gridMask, p.rotation);
  if (p.mirrorX) mask = mask.map((row) => [...row].reverse());
  if (p.mirrorY) mask = [...mask].reverse();
  maskCache.set(key, mask);
  return mask;
}
export function occupiedCells(p: PlacedPiece): string[] {
  return gridMask(p).flatMap((row, y) =>
    row.flatMap((filled, x) => (filled ? [`${p.x + x},${p.y + y}`] : []))
  );
}
export function bounds(p: PlacedPiece) {
  const piece = getPiece(p.pieceId)!;
  return { x: p.x, y: p.y, ...footprint(piece, p.rotation) };
}
export function createPlacementValidator(doc: PressDocument, ignored = new Set<string>()) {
  const occupied = new Map<string, PlacedPiece[]>();
  const register = (
    ownersByCell: Map<string, PlacedPiece[]>,
    p: PlacedPiece,
    cells = occupiedCells(p)
  ) => {
    for (const cell of cells) {
      const owners = ownersByCell.get(cell);
      if (owners) owners.push(p);
      else ownersByCell.set(cell, [p]);
    }
  };
  if (!doc.options.allowOverlap)
    for (const pass of doc.passes)
      for (const p of pass.pieces) {
        if (!ignored.has(p.uid)) register(occupied, p);
      }
  const validate = (candidates: PlacedPiece[]): boolean => {
    const pending = new Map<string, PlacedPiece[]>();
    for (const p of candidates) {
      if (!getPiece(p.pieceId) || !Number.isInteger(p.x) || !Number.isInteger(p.y)) return false;
      const b = bounds(p);
      if (
        b.x < 0 ||
        b.y < 0 ||
        b.x + b.width > doc.board.width ||
        b.y + b.height > doc.board.height
      )
        return false;
      if (!doc.options.allowOverlap) {
        const cells = occupiedCells(p);
        const checked = new Set<PlacedPiece>();
        for (const cell of cells)
          for (const other of [...(occupied.get(cell) ?? []), ...(pending.get(cell) ?? [])]) {
            if (checked.has(other)) continue;
            checked.add(other);
            if (surfacesOverlap(p, other)) return false;
          }
        register(pending, p, cells);
      }
    }
    return true;
  };
  return {
    canPlace: validate,
    add(candidates: PlacedPiece[]) {
      if (!doc.options.allowOverlap) for (const p of candidates) register(occupied, p);
    }
  };
}
export function canPlace(
  doc: PressDocument,
  candidates: PlacedPiece[],
  ignored = new Set<string>()
): boolean {
  return createPlacementValidator(doc, ignored).canPlace(candidates);
}
export function nextRotation(p: PlacedPiece): PlacedPiece {
  const piece = getPiece(p.pieceId)!;
  const rotations = piece.allowedRotations;
  const rotation = rotations[(rotations.indexOf(p.rotation) + 1) % rotations.length];
  const old = footprint(piece, p.rotation),
    next = footprint(piece, rotation);
  // Center rotation, with half-stud ties resolved toward the original origin.
  // Symmetric rounding prevents cumulative drift after four quarter turns.
  return {
    ...p,
    rotation,
    x: p.x + Math.trunc((old.width - next.width) / 2),
    y: p.y + Math.trunc((old.height - next.height) / 2)
  };
}
export function pieceTransform(piece: LetterpressPiece, p: PlacedPiece, scale = surfaceScale) {
  const size = footprint(piece, p.rotation);
  return `translate(${p.x + size.width / 2} ${p.y + size.height / 2}) scale(${p.mirrorX ? -1 : 1} ${p.mirrorY ? -1 : 1}) rotate(${p.rotation}) scale(${scale}) translate(${-piece.footprint.widthStuds / 2} ${-piece.footprint.heightStuds / 2})`;
}
export function symmetryPlacements(p: PlacedPiece, doc: PressDocument): PlacedPiece[] {
  const b = bounds(p);
  const list = [p];
  if (doc.options.symmetry === 'x' || doc.options.symmetry === 'both')
    list.push({ ...p, x: doc.board.width - p.x - b.width, mirrorX: !p.mirrorX });
  if (doc.options.symmetry === 'y' || doc.options.symmetry === 'both')
    list.push(
      ...list.map((item) => ({
        ...item,
        y: doc.board.height - item.y - b.height,
        mirrorY: !item.mirrorY
      }))
    );
  const seen = new Set<string>();
  return list.filter((item) => {
    const key = `${item.x},${item.y}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
