import type { LetterpressPiece } from "./letterpress-pieces";

export type PlacedPiece = {
  uid: string;
  pieceId: string;
  x: number;
  y: number;
  rotation: 0 | 90 | 180 | 270;
  color: string;
  seed?: number;
};

export function svgTransform(
  piece: LetterpressPiece,
  placed: PlacedPiece,
  surfaceScale = 0.975
): string {
  const { widthStuds: w, heightStuds: h } = piece.footprint;
  const cx = w / 2;
  const cy = h / 2;

  // Scale each piece around its center to create the physical seam between tiles.
  return [
    `translate(${placed.x} ${placed.y})`,
    `translate(${cx} ${cy})`,
    `rotate(${placed.rotation})`,
    `scale(${surfaceScale})`,
    `translate(${-cx} ${-cy})`,
  ].join(" ");
}

export function pieceToSvgPath(piece: LetterpressPiece, placed: PlacedPiece) {
  return {
    d: piece.geometry.path,
    fill: placed.color,
    fillRule: piece.geometry.fillRule,
    transform: svgTransform(piece, placed),
  };
}

/**
 * Important:
 * Apply distress/noise PER PIECE, not once over the whole artwork.
 * A single global grunge texture loses the physical-letterpress effect.
 */
export function seededNoise01(seed = 1) {
  let t = seed + 0x6D2B79F5;
  return () => {
    t += 0x6D2B79F5;
    let x = t;
    x = Math.imul(x ^ (x >>> 15), x | 1);
    x ^= x + Math.imul(x ^ (x >>> 7), x | 61);
    return ((x ^ (x >>> 14)) >>> 0) / 4294967296;
  };
}
