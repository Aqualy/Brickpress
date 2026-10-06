import { catalogMeta } from '../catalog/catalog';
import { bounds } from '../geometry/geometry';
import { seededRandom } from './noise';
import type { InkPass, PaperSettings, PlacedPiece, PrintSettings } from '../types/document';

export const safeId = (value: string) =>
  'i' +
  Array.from(value)
    .map((c) => c.charCodeAt(0).toString(16))
    .join('-');
const n = (value: number) => Number(value.toFixed(5));
export function passTransform(pass: InkPass, settings: PrintSettings) {
  const random = seededRandom(pass.registrationSeed);
  const amount =
    settings.registrationError *
    catalogMeta.rendering.recommendedPrintSimulation.registrationErrorForMultiColorStuds[1] *
    3;
  return `translate(${n((random() - 0.5) * amount * 2)} ${n((random() - 0.5) * amount * 2)})`;
}
export function impression(piece: PlacedPiece, settings: PrintSettings) {
  const random = seededRandom(piece.seed),
    b = bounds(piece);
  const range = catalogMeta.rendering.recommendedPrintSimulation;
  const jitter = settings.misalignment * range.positionJitterStuds[1];
  const dx = (random() - 0.5) * jitter * 2,
    dy = (random() - 0.5) * jitter * 2;
  const rotation = (random() - 0.5) * range.rotationJitterDegrees[1] * 2 * settings.misalignment;
  const opacity = Math.min(
    1,
    0.68 +
      settings.inkAmount * 0.32 +
      settings.pressure * 0.08 -
      random() * (1 - settings.rollerConsistency) * 0.18 -
      random() * settings.pieceWear * 0.1
  );
  return {
    transform: `translate(${n(dx)} ${n(dy)}) rotate(${n(rotation)} ${b.x + b.width / 2} ${b.y + b.height / 2})`,
    opacity: n(opacity),
    filterId: safeId(piece.uid)
  };
}
export function impressionFilter(
  piece: PlacedPiece,
  settings: PrintSettings,
  paper: PaperSettings
) {
  const id = safeId(piece.uid);
  const roughness = settings.printRoughness;
  const erosion = settings.edgeWear * 0.016 * (1 - settings.pressure * 0.35);
  const noiseFrequency = n(27 / settings.textureScale);
  const alphaContrast = 8 + settings.inkAmount * 4;
  const dropout =
    (1 - settings.inkCoverage) * 0.65 +
    settings.pieceWear * 0.03 +
    (settings.paperTooth + paper.tooth) * 0.013;
  const cutoff = 0.47 + dropout - settings.inkAmount * 0.11;
  const noiseOffset = n(1 - alphaContrast * cutoff);
  if (
    settings.inkCoverage === 1 &&
    settings.pieceWear === 0 &&
    settings.edgeWear === 0 &&
    roughness === 0
  )
    return `<filter id="${id}"><feColorMatrix in="SourceGraphic" type="matrix" values="1 0 0 0 0 0 1 0 0 0 0 0 1 0 0 0 0 0 1 0"/></filter>`;
  // Every filter has its own seed. A low-frequency field produces uneven roller
  // coverage; a high-frequency field produces tooth, dropout and edge character.
  return `<filter id="${id}" x="-15%" y="-15%" width="130%" height="130%" color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="${noiseFrequency}" numOctaves="2" seed="${piece.seed % 65536}" result="grain"/>
    <feColorMatrix in="grain" type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  ${n(alphaContrast * 0.333)} ${n(alphaContrast * 0.333)} ${n(alphaContrast * 0.334)} 0 ${noiseOffset}" result="tooth"/>
    <feTurbulence type="fractalNoise" baseFrequency="${n(1.4 / settings.textureScale)}" numOctaves="2" seed="${(piece.seed + 817) % 65536}" result="roller"/>
    <feColorMatrix in="roller" type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  ${n((1 - settings.rollerConsistency) * 0.2)} ${n((1 - settings.rollerConsistency) * 0.2)} ${n((1 - settings.rollerConsistency) * 0.2)} 0 ${n(0.98 - (1 - settings.rollerConsistency) * 0.4)}" result="uneven"/>
    <feComposite in="tooth" in2="uneven" operator="in" result="coverage"/>
    <feMorphology in="SourceGraphic" operator="erode" radius="${n(erosion)}" result="edge"/>
    <feDisplacementMap in="edge" in2="grain" scale="${n(roughness * 0.026)}" xChannelSelector="R" yChannelSelector="G" result="worn"/>
    <feComposite in="worn" in2="coverage" operator="in"/>
  </filter>`;
}
export function paperColor(paper: PaperSettings) {
  const rgb = [1, 3, 5].map((offset) =>
    Math.min(
      255,
      Math.round(parseInt(paper.color.slice(offset, offset + 2), 16) * paper.brightness)
    )
  );
  return `#${rgb.map((v) => v.toString(16).padStart(2, '0')).join('')}`;
}
export function paperMarkup(paper: PaperSettings, width: number, height: number) {
  const random = seededRandom(paper.seed);
  const fibers = Array.from({ length: Math.round(width * height * paper.fibers * 1.2) }, () => {
    const x = random() * width,
      y = random() * height;
    return `<path d="M${n(x)} ${n(y)}l${n((random() - 0.5) * 0.08)} ${n(random() * 0.06)}" stroke="#655c49" stroke-width=".008" opacity=".075"/>`;
  }).join('');
  return `<defs><filter id="paper-grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="32" numOctaves="3" seed="${paper.seed % 65536}"/><feColorMatrix type="saturate" values="0"/></filter></defs><rect width="${width}" height="${height}" fill="${paperColor(paper)}"/><rect width="${width}" height="${height}" filter="url(#paper-grain)" opacity="${n(paper.grain * 0.12)}"/>${fibers}`;
}
