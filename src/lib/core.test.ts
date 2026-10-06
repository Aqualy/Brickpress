import { describe, it, expect, vi } from 'vitest';
import { catalogMeta, getPiece, parsePieces, pieces } from './catalog/catalog';
import {
  bounds,
  canPlace,
  footprint,
  gridMask,
  nextRotation,
  occupiedCells,
  pieceTransform,
  rotateMask,
  symmetryPlacements
} from './geometry/geometry';
import { History } from './history/history';
import { createDocument, parseDocument, serializeDocument } from './persistence/document';
import { seededRandom } from './printing/noise';
import { impression, impressionFilter, passTransform } from './printing/print-engine';
import { renderSvg } from './export/svg-renderer';
import { exportDimensions } from './export/export';
import type { PlacedPiece } from './types/document';

const placed = (pieceId = '3070', x = 0, y = 0, rotation = 0): PlacedPiece => ({
  uid: 'test',
  pieceId,
  x,
  y,
  rotation,
  seed: 171
});
describe('catalog', () => {
  it('loads the supplied catalog and metadata without rebuilding it', () => {
    expect(pieces).toHaveLength(41);
    expect(catalogMeta.coordinateSystem.defaultSurfaceScale).toBe(0.975);
    expect(getPiece('25269')?.geometry.path).toContain('A');
  });
  it('provides lookup, default toolbar and physical-height flags', () => {
    expect(getPiece('missing')).toBeUndefined();
    expect(pieces.filter((p) => p.defaultToolbar)).toHaveLength(10);
    expect(getPiece('68869')?.surfaceHeightPlates).toBe(2);
  });
  it('omits malformed and duplicate entries with useful warnings', () => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    expect(parsePieces([{}, getPiece('3070'), getPiece('3070')])).toHaveLength(1);
    expect(spy).toHaveBeenCalledTimes(2);
    spy.mockRestore();
  });
});
describe('grid geometry', () => {
  it('rotates non-square footprints, then returns to the original', () => {
    const p = getPiece('3069')!;
    expect(footprint(p, 90)).toEqual({ width: 1, height: 2 });
    expect(footprint(p, 360)).toEqual({ width: 2, height: 1 });
  });
  it('rotates asymmetric collision masks clockwise', () => {
    const mask = [
      [1, 1],
      [1, 0]
    ];
    expect(rotateMask(mask, 90)).toEqual([
      [1, 1],
      [0, 1]
    ]);
    expect(rotateMask(mask, 180)).toEqual([
      [0, 1],
      [1, 1]
    ]);
    expect(rotateMask(mask, 360)).toEqual(mask);
    expect(mask).toEqual([
      [1, 1],
      [1, 0]
    ]);
  });
  it('preserves the logical center exactly when footprint parity matches', () => {
    const p = placed('35787', 4, 4);
    const rotated = nextRotation(p);
    expect(bounds(rotated)).toEqual(bounds(p));
    expect(rotated.rotation).toBe(90);
  });
  it('snaps half-stud parity after rotation and respects allowed rotations', () => {
    const p = nextRotation(placed('3069', 4, 4));
    expect(p.x).toBe(4);
    expect(p.y).toBe(4);
    expect(getPiece(p.pieceId)!.allowedRotations).toContain(p.rotation);
    expect(pieceTransform(getPiece(p.pieceId)!, p)).toContain('translate(4.5 5)');
  });
  it('returns non-square pieces to their original origin after four rotations', () => {
    for (const id of ['3069', '2431', '6636', '8165']) {
      const original = placed(id, 8, 8);
      let p = original;
      for (let i = 0; i < 4; i++) p = nextRotation(p);
      expect(p).toEqual(original);
    }
  });
  it('uses gridMask holes instead of bounding boxes', () => {
    const doc = createDocument();
    doc.passes[0].pieces = [placed('14719')];
    expect(canPlace(doc, [placed('3070', 1, 1)])).toBe(true);
    expect(canPlace(doc, [placed('3070', 0, 1)])).toBe(false);
  });
  it('checks rotated masks, candidates, bounds and explicit overlap', () => {
    const doc = createDocument();
    doc.passes[0].pieces = [placed('3069', 0, 0, 90)];
    expect(occupiedCells(doc.passes[0].pieces[0])).toEqual(['0,0', '0,1']);
    expect(canPlace(doc, [placed('3070', 0, 1)])).toBe(false);
    expect(canPlace(doc, [placed('3070', 1, 1)])).toBe(true);
    expect(canPlace(doc, [placed('3069', 15, 0)])).toBe(false);
    expect(canPlace(doc, [placed(), placed()])).toBe(false);
    doc.options.allowOverlap = true;
    expect(canPlace(doc, [placed()])).toBe(true);
  });
  it('ignores moving originals and supports mirrored masks', () => {
    const doc = createDocument();
    const p = placed('14719');
    doc.passes[0].pieces = [p];
    expect(canPlace(doc, [{ ...p, rotation: 90 }], new Set(['test']))).toBe(true);
    expect(gridMask({ ...p, mirrorX: true })).toEqual([
      [1, 1],
      [0, 1]
    ]);
  });
  it('produces deduplicated four-way symmetry ghosts', () => {
    const doc = createDocument();
    doc.options.symmetry = 'both';
    const ghosts = symmetryPlacements(placed('25269', 3, 4), doc);
    expect(ghosts).toHaveLength(4);
    expect(ghosts[3]).toMatchObject({ x: 12, y: 11, mirrorX: true, mirrorY: true });
  });
});
describe('document persistence', () => {
  it('round-trips every piece, pass, seed and setting', () => {
    const doc = createDocument(true);
    expect(parseDocument(serializeDocument(doc))).toEqual(doc);
    expect(doc.passes.flatMap((p) => p.pieces).length).toBeGreaterThan(30);
  });
  it('creates an identical starter document across server and client', () => {
    expect(createDocument(true)).toEqual(createDocument(true));
  });
  it('rejects unsupported, corrupt, out-of-bounds and missing catalog data', () => {
    const doc = createDocument();
    expect(() => parseDocument('{}')).toThrow();
    doc.passes[0].pieces.push(placed('missing'));
    expect(() => parseDocument(serializeDocument(doc))).toThrow(/unavailable/);
    doc.passes[0].pieces = [placed('3070', -1)];
    expect(() => parseDocument(serializeDocument(doc))).toThrow(/outside/);
  });
  it('rejects duplicate UIDs and non-finite settings', () => {
    const doc = createDocument();
    doc.passes[0].pieces = [placed(), placed('3070', 2, 2)];
    expect(() => parseDocument(serializeDocument(doc))).toThrow(/placement/);
    doc.passes[0].pieces = [];
    doc.printSettings.pressure = NaN;
    expect(() => parseDocument(serializeDocument(doc))).toThrow(/pressure/);
  });
});
describe('immutable history', () => {
  it('undoes and redoes complete transactions', () => {
    const history = new History<string>();
    history.push('before');
    expect(history.undo('after')).toBe('before');
    expect(history.canRedo).toBe(true);
    expect(history.redo('before')).toBe('after');
  });
  it('clears redo on a new branch and caps history', () => {
    const h = new History<number>(2);
    h.push(0);
    h.push(1);
    h.push(2);
    expect(h.undo(3)).toBe(2);
    expect(h.undo(2)).toBe(1);
    expect(h.undo(1)).toBeUndefined();
    h.push(9);
    expect(h.canRedo).toBe(false);
  });
});
describe('individual impressions and exports', () => {
  it('validates custom dimensions and preserves document aspect ratio', () => {
    const doc = createDocument();
    doc.board = { width: 24, height: 12 };
    expect(exportDimensions(doc, 2400)).toEqual({ width: 2400, height: 1200 });
    expect(() => exportDimensions(doc, NaN)).toThrow();
    expect(() => exportDimensions(doc, 16000)).toThrow();
  });
  it('produces repeatable seeded noise and distinct piece seeds', () => {
    const a = seededRandom(1),
      b = seededRandom(1),
      c = seededRandom(2);
    const first = Array.from({ length: 20 }, a);
    expect(first).toEqual(Array.from({ length: 20 }, b));
    expect(first).not.toEqual(Array.from({ length: 20 }, c));
  });
  it('keeps per-piece impression and whole-pass registration deterministic', () => {
    const doc = createDocument(true);
    const p = doc.passes[0].pieces[0];
    expect(impression(p, doc.printSettings)).toEqual(impression(p, doc.printSettings));
    expect(passTransform(doc.passes[0], doc.printSettings)).toEqual(
      passTransform(doc.passes[0], doc.printSettings)
    );
    expect(impressionFilter(p, doc.printSettings, doc.paper)).toContain(`seed="${p.seed % 65536}"`);
  });
  it('exports individual surfaces, keeps UI/grid out, and honors transparent background', () => {
    const doc = createDocument();
    doc.passes[0].pieces = [placed(), { ...placed('2431', 2), uid: 'bar' }];
    const svg = renderSvg(doc, { mode: 'design', paper: false, width: 2048 });
    expect(svg.match(/<path /g)).toHaveLength(2);
    expect(svg).not.toContain('filter=');
    expect(svg).not.toContain('stud-grid');
    expect(svg).not.toContain('paper-grain');
    expect(svg).toContain('width="2048"');
  });
  it('applies one filter to every individual piece and excludes hidden passes', () => {
    const doc = createDocument(true);
    doc.passes[1].visible = false;
    const svg = renderSvg(doc, { mode: 'print', paper: false, width: 1024 });
    const count = doc.passes.filter((p) => p.visible).flatMap((p) => p.pieces).length;
    expect(svg.match(/<filter /g)).toHaveLength(count);
    expect(svg.match(/data-ink-pass=/g)).toHaveLength(2);
  });
});
