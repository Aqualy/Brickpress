import { afterEach, describe, expect, it } from 'vitest';
import { Vector3 } from 'three';
import { geometryPose, pieceHeight, pieceMatrix } from './geometry';
import { getPiece } from '../catalog/catalog';
import { footprint } from '../geometry/geometry';
import { Editor } from '../stores/editor.svelte';
import type { PlacedPiece } from '../types/document';

describe('3D document mapping', () => {
  it('maps every rotation and global reflection to the same footprint as SVG', () => {
    const catalog = getPiece('3069')!;
    for (const rotation of [0, 90, 180, 270])
      for (const mirrorX of [false, true])
        for (const mirrorY of [false, true]) {
          const p: PlacedPiece = {
            uid: 'p',
            pieceId: catalog.id,
            x: 3,
            y: 4,
            rotation,
            mirrorX,
            mirrorY,
            seed: 1
          };
          const pose = geometryPose(p),
            size = footprint(catalog, rotation),
            angle = (rotation * Math.PI) / 180;
          const local = new Vector3(-0.8, 0.4, -0.3);
          const actual = local
            .clone()
            .multiply(new Vector3(pose.mx ? -1 : 1, 1, pose.my ? -1 : 1))
            .applyMatrix4(pieceMatrix(p, { width: 16, height: 16 }));
          expect(actual.x).toBeCloseTo(
            3 +
              size.width / 2 -
              8 +
              (mirrorX ? -1 : 1) * (local.x * Math.cos(angle) - local.z * Math.sin(angle)),
            10
          );
          expect(actual.z).toBeCloseTo(
            4 +
              size.height / 2 -
              8 +
              (mirrorY ? -1 : 1) * (local.x * Math.sin(angle) + local.z * Math.cos(angle)),
            10
          );
          expect(actual.y).toBeCloseTo(0.4);
          expect(pieceMatrix(p, { width: 16, height: 16 }).determinant()).toBeCloseTo(1);
        }
  });
  it('uses the catalog heights, including non-coplanar special pieces', () => {
    expect(pieceHeight('3070')).toBe(0.4);
    expect(pieceHeight('68869')).toBe(getPiece('68869')!.surfaceHeightPlates * 0.4);
    expect(pieceHeight('68869')).toBeGreaterThan(0.4);
  });
});

const editors: Editor[] = [];
afterEach(() => editors.splice(0).forEach((e) => e.destroy()));
describe('3D shared editing state', () => {
  it('keeps view and camera out of documents, preferences and history', () => {
    const e = new Editor();
    editors.push(e);
    e.newDocument();
    const original = e.projectText(),
      preferences = e.preferencesText();
    e.setView('3d');
    e.camera3D = { azimuth: 1, elevation: 1, target: [2, 0, 3], projection: 'orthographic' };
    e.setZoom(150);
    e.setView('2d');
    expect(e.zoomCommand.value).toBe('fit');
    e.setView('3d');
    expect(e.zoomCommand.value).toBe(150);
    expect(e.projectText()).toBe(original);
    expect(e.preferencesText()).toBe(preferences);
    expect(e.dirty).toBe(false);
    expect(e.canUndo).toBe(false);
  });
  it('shares placements, validation and undo across views, and cancels unfinished strokes', () => {
    const e = new Editor();
    editors.push(e);
    e.newDocument();
    e.setView('3d');
    e.choosePiece('3070');
    e.place(2, 2);
    const uid = e.allPieces[0].piece.uid;
    e.select(uid);
    e.tool = 'select';
    e.move(2, 0);
    expect(e.allPieces[0].piece.x).toBe(4);
    e.move(-10, 0);
    expect(e.allPieces[0].piece.x).toBe(4);
    e.setView('2d');
    e.undo();
    expect(e.allPieces[0].piece.x).toBe(2);
    e.setView('3d');
    e.redo();
    expect(e.allPieces[0].piece.x).toBe(4);
    e.choosePiece('3070');
    e.beginPlacementStroke();
    e.paintPlacement(6, 6);
    e.setView('2d');
    expect(e.allPieces).toHaveLength(1);
    expect(e.placementStrokeActive).toBe(false);
  });
});
