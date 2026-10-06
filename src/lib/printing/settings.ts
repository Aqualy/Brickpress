import type { PrintSettings, PaperSettings } from '../types/document';
export const printDefaults: PrintSettings = {
  inkAmount: 0.82,
  pressure: 0.6,
  inkCoverage: 0.93,
  rollerConsistency: 0.72,
  pieceWear: 0.28,
  edgeWear: 0.24,
  paperTooth: 0.35,
  textureScale: 1,
  misalignment: 0.22,
  registrationError: 0.22,
  printRoughness: 0.32
};
export const printPresets: Record<string, PrintSettings> = {
  Clean: {
    ...printDefaults,
    inkAmount: 1,
    pressure: 0.8,
    inkCoverage: 1,
    rollerConsistency: 1,
    pieceWear: 0,
    edgeWear: 0,
    paperTooth: 0,
    misalignment: 0,
    registrationError: 0,
    printRoughness: 0
  },
  'Fresh Press': {
    ...printDefaults,
    inkCoverage: 0.97,
    pieceWear: 0.1,
    edgeWear: 0.08,
    printRoughness: 0.15
  },
  Normal: printDefaults,
  Worn: {
    ...printDefaults,
    pieceWear: 0.7,
    edgeWear: 0.65,
    rollerConsistency: 0.5,
    printRoughness: 0.6
  },
  'Dry Ink': {
    ...printDefaults,
    inkAmount: 0.46,
    inkCoverage: 0.84,
    rollerConsistency: 0.38,
    paperTooth: 0.65,
    printRoughness: 0.68
  },
  'Heavy Pressure': {
    ...printDefaults,
    pressure: 0.95,
    inkAmount: 0.95,
    inkCoverage: 0.98,
    edgeWear: 0.12,
    printRoughness: 0.2
  }
};
export const printControls: {
  key: keyof PrintSettings;
  label: string;
  min?: number;
  max?: number;
}[] = [
  { key: 'inkAmount', label: 'Ink amount' },
  { key: 'pressure', label: 'Pressure' },
  { key: 'inkCoverage', label: 'Ink coverage', min: 0.7 },
  { key: 'rollerConsistency', label: 'Roller consistency' },
  { key: 'pieceWear', label: 'Piece wear' },
  { key: 'edgeWear', label: 'Edge wear' },
  { key: 'paperTooth', label: 'Paper tooth' },
  { key: 'textureScale', label: 'Texture scale', min: 0.4, max: 2.5 },
  { key: 'misalignment', label: 'Misalignment' },
  { key: 'registrationError', label: 'Registration error' },
  { key: 'printRoughness', label: 'Print roughness' }
];
export const paperPresets: Record<string, Omit<PaperSettings, 'seed'>> = {
  'Bright White': {
    preset: 'Bright White',
    color: '#ffffff',
    grain: 0.1,
    tooth: 0.15,
    fibers: 0.08,
    brightness: 1
  },
  'Natural White': {
    preset: 'Natural White',
    color: '#f6f4ec',
    grain: 0.22,
    tooth: 0.3,
    fibers: 0.16,
    brightness: 1
  },
  'Warm Ivory': {
    preset: 'Warm Ivory',
    color: '#f1e8d4',
    grain: 0.25,
    tooth: 0.35,
    fibers: 0.2,
    brightness: 1
  },
  Recycled: {
    preset: 'Recycled',
    color: '#d9d2bf',
    grain: 0.5,
    tooth: 0.55,
    fibers: 0.7,
    brightness: 1
  },
  Newsprint: {
    preset: 'Newsprint',
    color: '#e4dec9',
    grain: 0.35,
    tooth: 0.3,
    fibers: 0.45,
    brightness: 1
  },
  'Heavy Cotton': {
    preset: 'Heavy Cotton',
    color: '#f6f1e4',
    grain: 0.4,
    tooth: 0.7,
    fibers: 0.3,
    brightness: 1
  }
};
