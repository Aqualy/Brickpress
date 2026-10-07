import { printControls } from '../printing/settings';
import type { PrintSettings } from '../types/document';

export interface SavedPrintPreset {
  id: string;
  name: string;
  settings: PrintSettings;
}
export interface Preferences {
  version: 1;
  gridAppearance: 'flat' | 'embossed';
  printPresets: SavedPrintPreset[];
}
export function defaultPreferences(): Preferences {
  return { version: 1, gridAppearance: 'flat', printPresets: [] };
}
function number(value: unknown, min: number, max: number): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value >= min && value <= max;
}
function record(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}
export function validPrintSettings(value: unknown): value is PrintSettings {
  return record(value) && printControls.every((c) => number(value[c.key], c.min ?? 0, c.max ?? 1));
}
/** Read legacy appearance records and retain only supported preference fields. */
export function parsePreferences(text: string): Preferences {
  const defaults = defaultPreferences();
  if (text === 'flat' || text === 'embossed') return { ...defaults, gridAppearance: text };
  const value: unknown = JSON.parse(text);
  if (!record(value) || !['flat', 'embossed'].includes(String(value.gridAppearance)))
    throw new Error('Invalid editor preferences.');
  const gridAppearance = value.gridAppearance as 'flat' | 'embossed';
  if (!('version' in value)) return { ...defaults, gridAppearance };
  if (value.version !== 1 || !Array.isArray(value.printPresets) || value.printPresets.length > 100)
    throw new Error('Invalid editor preferences.');
  const named = (row: unknown) =>
    record(row) &&
    typeof row.id === 'string' &&
    row.id.length > 0 &&
    row.id.length <= 100 &&
    typeof row.name === 'string' &&
    row.name.trim().length > 0 &&
    row.name.length <= 80;
  if (
    !value.printPresets.every((row) => named(row) && validPrintSettings(row.settings)) ||
    new Set(value.printPresets.map((row) => row.id)).size !== value.printPresets.length
  )
    throw new Error('Invalid saved presets.');
  return { version: 1, gridAppearance, printPresets: value.printPresets as SavedPrintPreset[] };
}
