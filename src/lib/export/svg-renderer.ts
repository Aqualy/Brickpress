import { getPiece } from '../catalog/catalog';
import { pieceTransform } from '../geometry/geometry';
import { designMaskId, designMaskMarkup } from '../geometry/design-surface';
import {
  impression,
  impressionFilter,
  paperColor,
  paperMarkup,
  passTransform
} from '../printing/print-engine';
import type { PressDocument } from '../types/document';

export interface ExportOptions {
  mode: 'design' | 'print';
  paper: boolean;
  width: number;
  grid?: boolean;
}
export const escapeXml = (text: string) =>
  text.replace(
    /[<>&"']/g,
    (c) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;', "'": '&apos;' })[c]!
  );
export function renderSvg(doc: PressDocument, options: ExportOptions): string {
  const { width, height } = doc.board;
  const printed = options.mode === 'print';
  const definitions: string[] = [],
    artwork: string[] = [];
  const masked = new Set<string>();
  for (const pass of doc.passes) {
    if (!pass.visible) continue;
    const paths = pass.pieces
      .map((p) => {
        const piece = getPiece(p.pieceId);
        if (!piece) return '';
        const effect = impression(p, doc.printSettings);
        if (printed) definitions.push(impressionFilter(p, doc.printSettings, doc.paper));
        else if (!masked.has(piece.id)) {
          definitions.push(designMaskMarkup(piece));
          masked.add(piece.id);
        }
        const transform = printed ? pieceTransform(piece, p) : pieceTransform(piece, p, 1);
        return `<g${printed ? ` transform="${effect.transform}" opacity="${effect.opacity}"` : ''}><path data-piece-id="${piece.id}" d="${escapeXml(piece.geometry.path)}" transform="${transform}" fill="${pass.color}" fill-rule="${piece.geometry.fillRule}"${printed ? ` filter="url(#${effect.filterId})"` : ` mask="url(#${designMaskId(piece)})"`}/></g>`;
      })
      .join('');
    artwork.push(
      `<g data-ink-pass="${escapeXml(pass.name)}"${printed ? ` transform="${passTransform(pass, doc.printSettings)}"` : ''}>${paths}</g>`
    );
  }
  const grid = options.grid
    ? `<defs><pattern id="export-grid" width="1" height="1" patternUnits="userSpaceOnUse"><path d="M1 0H0V1" fill="none" stroke="#7d8177" stroke-width=".01"/></pattern></defs><rect width="${width}" height="${height}" fill="url(#export-grid)"/>`
    : '';
  const background = options.paper
    ? printed
      ? paperMarkup(doc.paper, width, height)
      : `<rect width="${width}" height="${height}" fill="${paperColor(doc.paper)}"/>`
    : '';
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${options.width}" height="${Math.round((options.width * height) / width)}" viewBox="0 0 ${width} ${height}"><title>${escapeXml(doc.name)}</title><desc>Individual catalog tile impressions. 1 stud = 8 mm. ${printed ? 'Seeded procedural print filters.' : 'Vector design.'}</desc><defs>${definitions.join('')}</defs>${background}${artwork.join('')}${grid}</svg>`;
}
