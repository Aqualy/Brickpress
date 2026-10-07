import { AUTOSAVE_KEY } from '../persistence/document';
import { PRESETS_KEY } from '../persistence/presets';
import { loadTrace, saveTrace } from '../persistence/tracing';
import { downloadBlob } from '../export/export';
import { fileResult, type PlatformAdapter, type StateKey } from './types';

const keys: Record<StateKey, string> = {
  recovery: AUTOSAVE_KEY,
  presets: PRESETS_KEY,
  preferences: 'form-impression-grid-appearance',
  'trace-metadata': 'brickpress:trace-metadata'
};
export function createBrowserPlatform(): PlatformAdapter {
  return {
    desktop: false,
    async chooseFile(kind) {
      return new Promise((resolve) => {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept =
          kind === 'trace'
            ? 'image/png,image/jpeg,image/webp,image/gif,image/avif,image/bmp'
            : '.json';
        input.hidden = true;
        document.body.append(input);
        const finish = () => input.remove();
        input.onchange = () => {
          const file = input.files?.[0];
          finish();
          resolve(
            file ? { status: 'success', value: { file, selected: null } } : { status: 'cancelled' }
          );
        };
        input.addEventListener(
          'cancel',
          () => {
            finish();
            resolve({ status: 'cancelled' });
          },
          { once: true }
        );
        input.click();
      });
    },
    async chooseSaveFile() {
      return { status: 'cancelled' };
    },
    async writeFile() {
      return {
        status: 'error',
        error: { code: 'unsupported', message: 'Use a browser download to save this file.' }
      };
    },
    async releaseFile() {},
    async exportFile(_kind, blob, name) {
      return fileResult(async () => downloadBlob(blob, name));
    },
    async readState(key) {
      return localStorage.getItem(keys[key]);
    },
    async writeState(key, value) {
      if (value === null) localStorage.removeItem(keys[key]);
      else localStorage.setItem(keys[key], value);
    },
    async quarantineState(key) {
      const value = localStorage.getItem(keys[key]);
      if (value !== null) localStorage.setItem(`${keys[key]}:invalid:${Date.now()}`, value);
      localStorage.removeItem(keys[key]);
    },
    loadTrace,
    saveTrace,
    async confirmAction(message) {
      return window.confirm(message);
    },
    async confirmUnsaved(_name, message) {
      return window.confirm(message ?? 'Discard unsaved changes?') ? 'discard' : 'cancel';
    },
    async confirmConflict() {
      return false;
    },
    async setDocumentTitle() {},
    async onCloseRequested() {
      return () => {};
    },
    async destroyWindow() {},
    async flush() {}
  };
}
