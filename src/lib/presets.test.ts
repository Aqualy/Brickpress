import { describe, expect, it } from 'vitest';
import { capturePreset, parsePresets, presetRows } from './persistence/presets';
import { createDocument } from './persistence/document';
import { canPlace } from './geometry/geometry';
import type { PlacedPiece } from './types/document';

const tile = (uid: string, x: number, y: number, pieceId = '3070'): PlacedPiece => ({
  uid,
  x,
  y,
  pieceId,
  rotation: 0,
  seed: 171
});

describe('composition presets', () => {
  it('trims canvas margins while preserving colors, pass order and reflected pieces', () => {
    const doc = createDocument();
    doc.passes[0].pieces = [{ ...tile('red', 4, 3), mirrorX: true }];
    doc.passes[1].pieces = [tile('green', 6, 3)];
    doc.passes[2].pieces = [tile('hidden', 10, 10)];
    doc.passes[2].visible = false;
    const before = JSON.stringify(doc);
    const preset = capturePreset(doc, '  Pair  ');
    expect(preset).toMatchObject({ name: 'Pair', width: 3, height: 1 });
    expect(preset.passes.map((p) => p.color)).toEqual([doc.passes[0].color, doc.passes[1].color]);
    expect(preset.passes[0].pieces[0]).toMatchObject({ x: 0, y: 0, mirrorX: true, seed: 171 });
    expect(preset.passes[1].pieces[0]).toMatchObject({ x: 2, y: 0 });
    expect(JSON.stringify(doc)).toBe(before);
  });
  it('saves only selected visible pieces and rejects empty captures', () => {
    const doc = createDocument();
    doc.passes[0].pieces = [tile('one', 3, 5), tile('two', 7, 8)];
    const preset = capturePreset(doc, 'Selected', new Set(['two']));
    expect(preset).toMatchObject({ width: 1, height: 1 });
    expect(preset.passes[0].pieces).toHaveLength(1);
    expect(preset.passes[0].pieces[0].uid).toBe('two');
    expect(() => capturePreset(doc, 'Empty', new Set())).toThrow(/visible pieces/);
  });
  it('rotates complete curved groups and their mirrors without losing the quarter-ring fit', () => {
    const doc = createDocument();
    doc.passes[0].pieces = [
      { ...tile('ring', 3, 3, '27925'), mirrorX: true },
      { ...tile('quarter', 4, 3, '25269'), mirrorX: true }
    ];
    const preset = capturePreset(doc, 'Curve');
    const target = createDocument();
    for (const rotation of [0, 90, 180, 270]) {
      const rows = presetRows(preset, 4, 4, rotation);
      expect(
        canPlace(
          target,
          rows.map((p) => p.piece)
        )
      ).toBe(true);
      expect(rows[0].piece.rotation).toBe(rotation);
      expect(rows[0].piece.mirrorX).toBe(rotation % 180 === 0 ? true : undefined);
      expect(rows[0].piece.mirrorY).toBe(rotation % 180 === 0 ? undefined : true);
    }
  });
  it('keeps original stud geometry on different boards and rejects small boards or collisions', () => {
    const doc = createDocument();
    doc.passes[0].pieces = [tile('bar', 4, 4, '2431')];
    const preset = capturePreset(doc, 'Bar');
    for (const size of [4, 8, 32]) {
      const target = createDocument();
      target.board = { width: size, height: size };
      expect(
        canPlace(
          target,
          presetRows(preset, 0, 0).map((p) => p.piece)
        )
      ).toBe(true);
      expect(target.board).toEqual({ width: size, height: size });
    }
    const small = createDocument();
    small.board = { width: 3, height: 3 };
    expect(
      canPlace(
        small,
        presetRows(preset, 0, 0).map((p) => p.piece)
      )
    ).toBe(false);
    small.board = { width: 4, height: 4 };
    small.passes[0].pieces = [tile('existing', 0, 0)];
    expect(
      canPlace(
        small,
        presetRows(preset, 0, 0).map((p) => p.piece)
      )
    ).toBe(false);
  });
  it('validates portable libraries and rejects bad geometry, duplicate IDs and oversized boards', () => {
    const doc = createDocument();
    doc.passes[0].pieces = [tile('one', 3, 4)];
    const preset = capturePreset(doc, 'One');
    expect(parsePresets(JSON.stringify([preset]))).toEqual([preset]);
    expect(() => parsePresets(JSON.stringify([preset, preset]))).toThrow(/Invalid/);
    expect(() => parsePresets(JSON.stringify([{ ...preset, width: 129 }]))).toThrow(/dimensions/);
    preset.passes[0].pieces[0].pieceId = 'missing';
    expect(() => parsePresets(JSON.stringify([preset]))).toThrow(/unavailable/);
  });
});
