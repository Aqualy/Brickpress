import { ExtrudeGeometry, Matrix4 } from 'three';
import { SVGLoader } from 'three/addons/loaders/SVGLoader.js';
import { getPiece } from '../catalog/catalog';
import { footprint } from '../geometry/geometry';
import { designGapStuds } from '../geometry/design-surface';
import type { PlacedPiece } from '../types/document';

export const plateHeight = 0.4;
export function pieceHeight(id: string) {
  return (getPiece(id)?.surfaceHeightPlates ?? 1) * plateHeight;
}

// Document reflections happen after rotation. Bake equivalent local reflections
// into geometry so instance matrices never have unsupported negative scales.
export function geometryPose(p: PlacedPiece) {
  const swapped = p.rotation % 180 !== 0;
  const mx = !!(swapped ? p.mirrorY : p.mirrorX);
  const my = !!(swapped ? p.mirrorX : p.mirrorY);
  return { mx, my, key: `${p.pieceId}:${mx}:${my}` };
}
export function pieceMatrix(p: PlacedPiece, board: { width: number; height: number }, layer = 0) {
  const size = footprint(getPiece(p.pieceId)!, p.rotation);
  return new Matrix4()
    .makeRotationY((-p.rotation * Math.PI) / 180)
    .setPosition(
      p.x + size.width / 2 - board.width / 2,
      layer * 0.000001,
      p.y + size.height / 2 - board.height / 2
    );
}
export function makePieceGeometry(p: PlacedPiece) {
  const piece = getPiece(p.pieceId)!;
  const { mx, my } = geometryPose(p);
  const parsed = new SVGLoader().parse(
    `<svg xmlns="http://www.w3.org/2000/svg"><path d="${piece.geometry.path}" fill-rule="${piece.geometry.fillRule}"/></svg>`
  );
  const shapes = parsed.paths.flatMap((path) => path.toShapes());
  const geometry = new ExtrudeGeometry(shapes, {
    depth: pieceHeight(p.pieceId) - 0.025,
    steps: 1,
    curveSegments: 32,
    bevelEnabled: true,
    bevelSegments: 2,
    bevelSize: 0.012,
    bevelThickness: 0.012,
    bevelOffset: -designGapStuds / 2 - 0.012
  });
  geometry.translate(-piece.footprint.widthStuds / 2, -piece.footprint.heightStuds / 2, 0.013);
  geometry.scale(mx ? -1 : 1, my ? 1 : -1, 1);
  geometry.rotateX(-Math.PI / 2);
  if (mx === my) {
    // SVG's downward Y axis reverses winding; retain outward-facing triangles.
    const count = geometry.getAttribute('position').count;
    const indices = [];
    for (let i = 0; i < count; i += 3) indices.push(i, i + 2, i + 1);
    geometry.setIndex(indices);
  }
  geometry.computeBoundingBox();
  geometry.computeBoundingSphere();
  return geometry;
}
