import { catalogMeta, getPiece, isPhysical } from '../catalog/catalog';
import { newId, newSeed } from '../printing/noise';
import { paperPresets, printDefaults, printControls } from '../printing/settings';
import { canPlace } from '../geometry/geometry';
import type { InkPass, PressDocument, PlacedPiece } from '../types/document';

export const starterSwatches = [
  '#252a26',
  '#ce4936',
  '#315eae',
  '#3d634b',
  '#e3ae36',
  '#795186',
  '#e17b39'
];
export const makePass = (color = starterSwatches[0], name = 'Ink Pass 1'): InkPass => ({
  id: newId(),
  name,
  color,
  visible: true,
  locked: false,
  registrationSeed: newSeed(),
  pieces: []
});
export function createDocument(sample = false): PressDocument {
  const size = catalogMeta.rendering.recommendedDefaultBoard;
  const doc: PressDocument = {
    version: 1,
    name: sample ? 'A study in bloom' : 'Untitled impression',
    board: { width: size.widthStuds, height: size.heightStuds },
    paper: { ...paperPresets['Natural White'], seed: 52718 },
    printSettings: { ...printDefaults },
    passes: [
      makePass(starterSwatches[1], '01 · Vermilion'),
      makePass(starterSwatches[3], '02 · Botanical'),
      makePass(starterSwatches[4], '03 · Ochre')
    ],
    swatches: [...starterSwatches],
    options: { grid: 'points', majorGrid: 4, allowOverlap: false, physical: false, symmetry: 'off' }
  };
  if (sample) {
    // Stable initial IDs and seeds keep the static server render and hydration
    // identical, and make the starter composition a repeatable first impression.
    doc.passes.forEach((pass, i) => {
      pass.id = `starter-pass-${i}`;
      pass.registrationSeed = 3719 + i * 8171;
    });
    const add = (pass: number, pieceId: string, x: number, y: number, rotation = 0) => {
      const index = doc.passes[pass].pieces.length;
      const p: PlacedPiece = {
        uid: `starter-piece-${pass}-${index}`,
        pieceId,
        x,
        y,
        rotation,
        seed: (pass + 1) * 100003 + index * 8191 + x * 127 + y * 17 + rotation
      };
      if (canPlace(doc, [p])) doc.passes[pass].pieces.push(p);
    };
    // An original modular garden, assembled entirely from catalog surfaces.
    for (const [x, y] of [
      [4, 3],
      [8, 2],
      [11, 5]
    ]) {
      add(0, '25269', x, y);
      add(0, '25269', x + 1, y, 90);
      add(0, '25269', x, y + 1, 270);
      add(0, '25269', x + 1, y + 1, 180);
      add(2, '98138', x, y + 2);
      add(2, '98138', x + 1, y + 2);
      for (let sy = y + 3; sy < 12; sy++) add(1, '3069', x, sy);
      add(1, '25269', x - 1, y + 4, 180);
      add(1, '25269', x + 2, y + 5, 270);
    }
    for (let x = 3; x <= 12; x++) add(1, '24246', x, 12, x % 2 ? 0 : 180);
    add(2, '1748', 3, 6, 180);
    add(2, '1748', 11, 3);
    add(2, '98138', 6, 2);
    add(2, '98138', 3, 9);
  }
  return doc;
}
export function serializeDocument(doc: PressDocument) {
  return JSON.stringify(doc, null, 2);
}
const validColor = (color: unknown): color is string =>
  typeof color === 'string' && /^#[0-9a-f]{6}$/i.test(color);
export function parseDocument(text: string): PressDocument {
  if (text.length > 20_000_000) throw new Error('This project is too large (maximum 20 MB).');
  const doc = JSON.parse(text) as PressDocument;
  if (
    doc?.version !== 1 ||
    !doc.board ||
    !Array.isArray(doc.passes) ||
    !doc.passes.length ||
    doc.passes.length > 100
  )
    throw new Error('This is not a supported .legopress.json project.');
  if (![doc.board.width, doc.board.height].every((n) => Number.isInteger(n) && n >= 1 && n <= 128))
    throw new Error('Board dimensions must be whole numbers between 1 and 128.');
  if (
    typeof doc.name !== 'string' ||
    doc.name.length > 160 ||
    !doc.paper ||
    !validColor(doc.paper.color) ||
    !doc.printSettings ||
    !doc.options
  )
    throw new Error('Project settings are incomplete.');
  for (const c of printControls) {
    const value = doc.printSettings[c.key];
    if (!Number.isFinite(value) || value < (c.min ?? 0) || value > (c.max ?? 1))
      throw new Error(`Invalid ${c.label.toLowerCase()}.`);
  }
  for (const key of ['grain', 'tooth', 'fibers', 'brightness'] as const) {
    if (!Number.isFinite(doc.paper[key]) || doc.paper[key] < 0 || doc.paper[key] > 1.2)
      throw new Error('Invalid paper settings.');
  }
  if (
    !Number.isInteger(doc.paper.seed) ||
    typeof doc.paper.preset !== 'string' ||
    !['off', 'points', 'lines', 'squares'].includes(doc.options.grid) ||
    ![4, 8].includes(doc.options.majorGrid) ||
    !['off', 'x', 'y', 'both'].includes(doc.options.symmetry) ||
    ['allowOverlap', 'physical'].some((k) => typeof doc.options[k as 'physical'] !== 'boolean')
  )
    throw new Error('Invalid document options.');
  const ids = new Set<string>();
  let count = 0;
  for (const pass of doc.passes) {
    if (
      typeof pass.id !== 'string' ||
      ids.has(pass.id) ||
      !validColor(pass.color) ||
      typeof pass.name !== 'string' ||
      typeof pass.visible !== 'boolean' ||
      typeof pass.locked !== 'boolean' ||
      !Number.isInteger(pass.registrationSeed) ||
      !Array.isArray(pass.pieces)
    )
      throw new Error('Invalid ink pass.');
    ids.add(pass.id);
    for (const p of pass.pieces) {
      const piece = getPiece(p.pieceId);
      if (!piece) throw new Error(`Catalog piece ${p.pieceId} is unavailable.`);
      if (
        typeof p.uid !== 'string' ||
        ids.has(p.uid) ||
        !Number.isInteger(p.x) ||
        !Number.isInteger(p.y) ||
        !Number.isInteger(p.seed) ||
        !piece.allowedRotations.includes(p.rotation) ||
        (p.mirrorX !== undefined && typeof p.mirrorX !== 'boolean') ||
        (p.mirrorY !== undefined && typeof p.mirrorY !== 'boolean')
      )
        throw new Error(`Invalid placement for ${piece.name}.`);
      if (doc.options.physical && !isPhysical(piece))
        throw new Error(`${piece.name} is incompatible with physical print mode.`);
      ids.add(p.uid);
      count++;
    }
  }
  if (count > 10_000) throw new Error('Maximum 10,000 pieces per project.');
  if (
    !canPlace(
      doc,
      doc.passes.flatMap((p) => p.pieces),
      new Set(doc.passes.flatMap((p) => p.pieces.map((item) => item.uid)))
    )
  )
    throw new Error(
      'The project has pieces outside the board or overlaps while overlap is disabled.'
    );
  if (!Array.isArray(doc.swatches) || doc.swatches.length > 200 || !doc.swatches.every(validColor))
    throw new Error('Invalid swatches.');
  return doc;
}
export const AUTOSAVE_KEY = 'form-impression:document:v1';
