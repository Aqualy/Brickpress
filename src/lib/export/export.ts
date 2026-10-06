import type { PressDocument } from '../types/document';
import { renderSvg, type ExportOptions } from './svg-renderer';

export function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob),
    link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
export const filename = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '') || 'impression';
export function exportDimensions(doc: PressDocument, width: number) {
  const height = Math.round((width * doc.board.height) / doc.board.width);
  if (
    !Number.isInteger(width) ||
    width < 64 ||
    width > 12000 ||
    height > 12000 ||
    width * height > 40_000_000
  )
    throw new Error('Use dimensions up to 12,000 px and 40 megapixels.');
  return { width, height };
}
export function exportSvg(doc: PressDocument, options: ExportOptions) {
  exportDimensions(doc, options.width);
  downloadBlob(
    new Blob([renderSvg(doc, options)], { type: 'image/svg+xml' }),
    `${filename(doc.name)}-${options.mode}.svg`
  );
}
export async function exportPng(doc: PressDocument, options: ExportOptions) {
  const { height } = exportDimensions(doc, options.width);
  const svg = renderSvg(doc, options);
  const url = URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' }));
  try {
    const image = new Image();
    await new Promise<void>((resolve, reject) => {
      image.onload = () => resolve();
      image.onerror = () => reject(new Error('The print could not be rasterized. Try SVG export.'));
      image.src = url;
    });
    const canvas = document.createElement('canvas');
    canvas.width = options.width;
    canvas.height = height;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('Canvas export is unavailable in this browser.');
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise<Blob>((resolve, reject) =>
      canvas.toBlob(
        (b) => (b ? resolve(b) : reject(new Error('PNG encoding failed.'))),
        'image/png'
      )
    );
    downloadBlob(blob, `${filename(doc.name)}-${options.mode}.png`);
  } finally {
    URL.revokeObjectURL(url);
  }
}
