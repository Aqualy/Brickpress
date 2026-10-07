import { describe, it, expect } from 'vitest';
import { defaultPreferences, parsePreferences } from './persistence/preferences';
import { Editor } from './stores/editor.svelte';
import { printDefaults } from './printing/settings';

describe('editor preferences and saved print presets', () => {
  it('migrates browser appearance strings and original desktop records', () => {
    expect(parsePreferences('embossed')).toEqual({
      ...defaultPreferences(),
      gridAppearance: 'embossed'
    });
    expect(parsePreferences('{"gridAppearance":"flat"}')).toEqual(defaultPreferences());
  });
  it('drops unsupported preference fields without losing saved print settings', () => {
    const preferences = {
      ...defaultPreferences(),
      printPresets: [{ id: 'p', name: 'Cotton', settings: { ...printDefaults } }]
    };
    expect(parsePreferences(JSON.stringify({ ...preferences, unused: { custom: true } }))).toEqual(
      preferences
    );
  });
  it('rejects invalid saved print controls, names, duplicate ids and preference versions', () => {
    const valid = { id: 'p', name: 'Cotton', settings: { ...printDefaults } };
    for (const printPresets of [
      [{ ...valid, settings: { ...printDefaults, inkAmount: 4 } }],
      [{ ...valid, name: ' ' }],
      [valid, { ...valid, name: 'Other' }]
    ]) {
      expect(() =>
        parsePreferences(JSON.stringify({ ...defaultPreferences(), printPresets }))
      ).toThrow();
    }
    expect(() =>
      parsePreferences(JSON.stringify({ ...defaultPreferences(), version: 2 }))
    ).toThrow();
  });
  it('captures complete print settings and updates a named preset without modifying the project', () => {
    const editor = new Editor();
    const before = editor.projectText();
    editor.savePrintPreset('My press');
    expect(editor.customPrintPresets[0].settings).toEqual(editor.doc.printSettings);
    editor.doc = { ...editor.doc, printSettings: { ...editor.doc.printSettings, pressure: 0.21 } };
    editor.savePrintPreset('MY PRESS');
    expect(editor.customPrintPresets).toHaveLength(1);
    expect(editor.customPrintPresets[0].settings.pressure).toBe(0.21);
    editor.doc = JSON.parse(before);
    expect(editor.projectText()).toBe(before);
    expect(editor.dirty).toBe(false);
    editor.destroy();
  });
  it('recovers custom presets and appearance without changing project serialization', () => {
    const editor = new Editor();
    editor.gridAppearance = 'embossed';
    editor.savePrintPreset('Cotton');
    const next = new Editor();
    const before = next.projectText();
    next.restorePreferences(editor.preferencesText());
    expect(next.customPrintPresets).toEqual(editor.customPrintPresets);
    expect(next.gridAppearance).toBe('embossed');
    expect(next.projectText()).toBe(before);
    expect(Object.keys(JSON.parse(next.preferencesText())).sort()).toEqual([
      'gridAppearance',
      'printPresets',
      'version'
    ]);
    editor.destroy();
    next.destroy();
  });
  it('moving an ink pass to any index keeps its identity and undoes as one operation', () => {
    const editor = new Editor();
    const original = editor.doc.passes.map((pass) => pass.id);
    editor.activePassId = original[0];
    editor.movePass(original[0], original.length - 1);
    expect(editor.doc.passes.at(-1)?.id).toBe(original[0]);
    expect(editor.activePassId).toBe(original[0]);
    editor.undo();
    expect(editor.doc.passes.map((pass) => pass.id)).toEqual(original);
    const before = editor.projectText();
    editor.movePass('missing', 0);
    editor.movePass(original[0], -1);
    expect(editor.projectText()).toBe(before);
    editor.destroy();
  });
});
