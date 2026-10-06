import { bounds } from '../geometry/geometry';
import { newId } from '../printing/noise';
import type { PlacedPiece, PressDocument } from '../types/document';
import { createDocument, makePass, parseDocument } from './document';

export const PRESETS_KEY = 'brickpress:compositions:v1';
export interface CompositionPreset {
  version: 1;
  id: string;
  name: string;
  width: number;
  height: number;
  passes: { name: string; color: string; pieces: PlacedPiece[] }[];
}

export function capturePreset(
  doc: PressDocument,
  name: string,
  selected?: Set<string>
): CompositionPreset {
  name = name.trim();
  if (!name || name.length > 80) throw new Error('Give the preset a name of 1–80 characters.');
  const passes = doc.passes
    .filter((p) => p.visible)
    .map((pass) => ({
      name: pass.name,
      color: pass.color,
      pieces: pass.pieces.filter((p) => !selected || selected.has(p.uid))
    }))
    .filter((p) => p.pieces.length);
  const pieces = passes.flatMap((p) => p.pieces);
  if (!pieces.length) throw new Error('Place or select some visible pieces first.');
  const boxes = pieces.map(bounds);
  const x = Math.min(...boxes.map((b) => b.x)),
    y = Math.min(...boxes.map((b) => b.y));
  return {
    version: 1,
    id: newId(),
    name,
    width: Math.max(...boxes.map((b) => b.x + b.width)) - x,
    height: Math.max(...boxes.map((b) => b.y + b.height)) - y,
    passes: passes.map((pass) => ({
      ...pass,
      pieces: pass.pieces.map((p) => ({ ...p, x: p.x - x, y: p.y - y }))
    }))
  };
}

export function parsePresets(text: string): CompositionPreset[] {
  if (text.length > 20_000_000) throw new Error('The preset library is too large.');
  const input = JSON.parse(text) as CompositionPreset[];
  if (!Array.isArray(input) || input.length > 100)
    throw new Error('Use a library of up to 100 presets.');
  const ids = new Set<string>();
  return input.map((preset) => {
    if (
      preset?.version !== 1 ||
      typeof preset.id !== 'string' ||
      !/^[a-z0-9-]{1,100}$/i.test(preset.id) ||
      ids.has(preset.id) ||
      typeof preset.name !== 'string' ||
      !preset.name.trim() ||
      preset.name.length > 80 ||
      !Array.isArray(preset.passes)
    )
      throw new Error('Invalid composition preset.');
    ids.add(preset.id);
    const doc = createDocument();
    doc.board = { width: preset.width, height: preset.height };
    doc.options.allowOverlap = true;
    doc.passes = preset.passes.map((p) => ({ ...makePass(p.color, p.name), pieces: p.pieces }));
    const checked = parseDocument(JSON.stringify(doc));
    if (!checked.passes.some((p) => p.pieces.length))
      throw new Error('A preset must contain pieces.');
    return {
      version: 1,
      id: preset.id,
      name: preset.name.trim(),
      width: preset.width,
      height: preset.height,
      passes: checked.passes
        .filter((p) => p.pieces.length)
        .map(({ name, color, pieces }) => ({ name, color, pieces }))
    };
  });
}

/** Rotate the whole composition, preserving catalog geometry and world-axis mirrors. */
export function presetRows(preset: CompositionPreset, x: number, y: number, rotation = 0) {
  return preset.passes.flatMap((pass, passIndex) =>
    pass.pieces.map((piece, i) => {
      const b = bounds(piece),
        swapped = rotation % 180 !== 0;
      let px = piece.x,
        py = piece.y;
      if (rotation === 90) [px, py] = [preset.height - b.y - b.height, b.x];
      else if (rotation === 180)
        [px, py] = [preset.width - b.x - b.width, preset.height - b.y - b.height];
      else if (rotation === 270) [px, py] = [b.y, preset.width - b.x - b.width];
      return {
        color: pass.color,
        passIndex,
        piece: {
          ...piece,
          uid: `preset-ghost-${passIndex}-${i}`,
          x: px + x,
          y: py + y,
          rotation: (piece.rotation + rotation) % 360,
          mirrorX: swapped ? piece.mirrorY : piece.mirrorX,
          mirrorY: swapped ? piece.mirrorX : piece.mirrorY
        }
      };
    })
  );
}
