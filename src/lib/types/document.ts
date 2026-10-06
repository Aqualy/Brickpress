export interface PlacedPiece {
  uid: string;
  pieceId: string;
  x: number;
  y: number;
  rotation: number;
  seed: number;
  mirrorX?: boolean;
  mirrorY?: boolean;
}
export interface InkPass {
  id: string;
  name: string;
  color: string;
  visible: boolean;
  locked: boolean;
  registrationSeed: number;
  pieces: PlacedPiece[];
}
export interface PrintSettings {
  inkAmount: number;
  pressure: number;
  inkCoverage: number;
  rollerConsistency: number;
  pieceWear: number;
  edgeWear: number;
  paperTooth: number;
  textureScale: number;
  misalignment: number;
  registrationError: number;
  printRoughness: number;
}
export interface PaperSettings {
  preset: string;
  color: string;
  grain: number;
  tooth: number;
  fibers: number;
  brightness: number;
  seed: number;
}
export type GridStyle = 'off' | 'points' | 'lines' | 'squares';
export interface PressDocument {
  version: 1;
  name: string;
  board: { width: number; height: number };
  paper: PaperSettings;
  printSettings: PrintSettings;
  passes: InkPass[];
  swatches: string[];
  options: {
    grid: GridStyle;
    majorGrid: number;
    allowOverlap: boolean;
    physical: boolean;
    symmetry: 'off' | 'x' | 'y' | 'both';
  };
}
