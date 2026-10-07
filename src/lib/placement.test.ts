import { afterEach, describe, expect, it } from 'vitest';
import { Editor } from './stores/editor.svelte';
import { placementPath } from './geometry/placement-path';
import { createPlacementValidator } from './geometry/geometry';
import { createDocument, serializeDocument } from './persistence/document';

const editors: Editor[] = [];
function editor() {
  const result = new Editor();
  result.newDocument();
  result.choosePiece('3070');
  editors.push(result);
  return result;
}
afterEach(() => {
  for (const item of editors.splice(0)) item.destroy();
});
const board = { width: 8, height: 8 };

describe('continuous placement path', () => {
  it('fills skipped horizontal cells in both directions', () => {
    const from = { x: 1.2, y: 3.5 },
      to = { x: 6.8, y: 3.5 };
    const path = [1, 2, 3, 4, 5, 6].map((x) => ({ x, y: 3 }));
    expect(placementPath(from, to, board)).toEqual(path);
    expect(placementPath(to, from, board)).toEqual([...path].reverse());
  });
  it('follows thin diagonal lines and every cell of a shallow line', () => {
    expect(placementPath({ x: 0.5, y: 0.5 }, { x: 4.5, y: 4.5 }, board)).toEqual(
      [0, 1, 2, 3, 4].map((x) => ({ x, y: x }))
    );
    expect(placementPath({ x: 0.5, y: 0.5 }, { x: 4.5, y: 1.5 }, board)).toEqual([
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 2, y: 0 },
      { x: 2, y: 1 },
      { x: 3, y: 1 },
      { x: 4, y: 1 }
    ]);
  });
  it('clips huge outside movements without inventing a path outside the paper', () => {
    expect(placementPath({ x: -1e6, y: 2.5 }, { x: 1e6, y: 2.5 }, board)).toEqual(
      Array.from({ length: 8 }, (_, x) => ({ x, y: 2 }))
    );
    expect(placementPath({ x: 8, y: 0 }, { x: 8, y: 7 }, board)).toEqual([]);
    expect(placementPath({ x: 9, y: 7 }, { x: 7, y: 9 }, board)).toEqual([]);
    expect(placementPath({ x: -2, y: -2 }, { x: -1, y: -1 }, board)).toEqual([]);
  });
  it('accepts a stationary pointer and rejects invalid coordinates', () => {
    expect(placementPath({ x: 2.5, y: 3.5 }, { x: 2.5, y: 3.5 }, board)).toEqual([{ x: 2, y: 3 }]);
    expect(placementPath({ x: NaN, y: 0 }, { x: 0, y: 0 }, board)).toEqual([]);
  });
});

describe('placement strokes', () => {
  it('shows pieces immediately, creates one undo step, and preserves seeds on redo', () => {
    const e = editor(),
      before = e.projectText();
    e.beginPlacementStroke();
    for (let x = 2; x < 7; x++) expect(e.paintPlacement(x, 3)).toBe(true);
    expect(e.allPieces).toHaveLength(5);
    e.finishPlacementStroke();
    const after = e.projectText();
    e.undo();
    expect(e.projectText()).toBe(before);
    expect(e.canUndo).toBe(false);
    e.redo();
    expect(e.projectText()).toBe(after);
  });
  it('never stacks duplicates on a revisited anchor with overlap enabled', () => {
    const e = editor();
    e.commit((doc) => {
      doc.options.allowOverlap = true;
    });
    e.beginPlacementStroke();
    e.paintPlacement(2, 3);
    e.paintPlacement(3, 3);
    e.paintPlacement(2, 3);
    e.finishPlacementStroke();
    expect(e.allPieces).toHaveLength(2);
  });
  it('skips blocked anchors and uses the rotated footprint at the canvas boundary', () => {
    const e = editor();
    e.place(2, 3);
    e.choosePiece('3069');
    e.placementRotation = 90;
    e.beginPlacementStroke();
    expect(e.paintPlacement(2, 3)).toBe(false);
    expect(e.paintPlacement(3, 3)).toBe(true);
    expect(e.paintPlacement(3, 4)).toBe(false);
    expect(e.paintPlacement(3, 5)).toBe(true);
    expect(e.paintPlacement(3, 15)).toBe(false);
    e.finishPlacementStroke();
    expect(e.allPieces).toHaveLength(3);
    expect(e.allPieces[1].piece.rotation).toBe(90);
    e.undo();
    expect(e.allPieces).toHaveLength(1);
  });
  it('cancels cleanly without erasing an existing redo branch', () => {
    const e = editor();
    e.place(0, 0);
    const saved = e.projectText();
    e.undo();
    const before = e.projectText(),
      dirty = e.dirty;
    e.beginPlacementStroke();
    e.paintPlacement(5, 5);
    e.finishPlacementStroke(true);
    expect(e.projectText()).toBe(before);
    expect(e.dirty).toBe(dirty);
    expect(e.canRedo).toBe(true);
    e.redo();
    expect(e.projectText()).toBe(saved);
  });
  it('finishes a live stroke before another operation and cannot affect a replacement document', () => {
    const e = editor();
    e.beginPlacementStroke();
    e.paintPlacement(2, 2);
    e.commit((doc) => {
      doc.name = 'Changed';
    });
    e.finishPlacementStroke(true);
    expect(e.allPieces).toHaveLength(1);
    e.undo();
    expect(e.doc.name).toBe('Untitled impression');
    e.undo();
    expect(e.allPieces).toHaveLength(0);
    e.beginPlacementStroke();
    e.paintPlacement(4, 4);
    e.load(serializeDocument(createDocument()));
    expect(e.paintPlacement(5, 5)).toBe(false);
    expect(e.allPieces).toHaveLength(0);
  });
  it('respects locks, visibility, physical compatibility, symmetry and project limits', () => {
    const e = editor();
    e.doc.passes[0].locked = true;
    e.beginPlacementStroke();
    expect(e.paintPlacement(1, 1)).toBe(false);
    e.finishPlacementStroke();
    e.doc.passes[0].locked = false;
    e.doc.passes[0].visible = false;
    e.beginPlacementStroke();
    expect(e.paintPlacement(1, 1)).toBe(false);
    e.finishPlacementStroke();
    e.doc.passes[0].visible = true;
    e.choosePiece('68869');
    e.doc.options.physical = true;
    e.beginPlacementStroke();
    expect(e.paintPlacement(1, 1)).toBe(false);
    e.finishPlacementStroke();
    e.doc.options.physical = false;
    e.choosePiece('3070');
    e.doc.options.symmetry = 'both';
    e.beginPlacementStroke();
    expect(e.paintPlacement(1, 1)).toBe(true);
    e.finishPlacementStroke();
    expect(e.allPieces).toHaveLength(4);
    e.doc.options.allowOverlap = true;
    e.doc.passes[0].pieces = Array.from({ length: 9999 }, (_, i) => ({
      uid: `limit-${i}`,
      pieceId: '3070',
      x: 0,
      y: 0,
      rotation: 0,
      seed: 1
    }));
    e.beginPlacementStroke();
    expect(e.paintPlacement(3, 3)).toBe(false);
    e.finishPlacementStroke();
    expect(e.allPieces).toHaveLength(9999);
  });
});

it('rejected symmetry groups do not reserve partial placements in the stroke validator', () => {
  const doc = createDocument();
  const tile = (x: number) => ({
    uid: `tile-${x}`,
    pieceId: '3070',
    x,
    y: 1,
    rotation: 0,
    seed: 1
  });
  doc.passes[0].pieces = [tile(1)];
  const validator = createPlacementValidator(doc);
  expect(validator.canPlace([tile(2), tile(1)])).toBe(false);
  expect(validator.canPlace([tile(2)])).toBe(true);
  validator.add([tile(2)]);
  expect(validator.canPlace([tile(2)])).toBe(false);
});
