import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { validateTrace, type TraceImage, type TraceOverlay } from '../persistence/tracing';
import { PersistenceQueue } from './queue';
import {
  fileResult,
  type FileKind,
  type FileResult,
  type PlatformAdapter,
  type SelectedFile,
  type UnsavedChoice
} from './types';

interface TraceRecord extends Omit<TraceImage, 'asset'> {
  assetId: string;
  mime: string;
}
export function createNativePlatform(): PlatformAdapter {
  const queue = new PersistenceQueue();
  let persistedTrace: { asset: Blob; assetId: string } | null = null;
  async function select(kind: FileKind, save: boolean, defaultName?: string) {
    const result = await fileResult(() =>
      invoke<SelectedFile | null>('select_file', { kind, save, defaultName })
    );
    if (result.status !== 'success') return result;
    return result.value === null
      ? ({ status: 'cancelled' } as const)
      : ({ status: 'success', value: result.value } as const);
  }
  const adapter: PlatformAdapter = {
    desktop: true,
    async chooseFile(kind) {
      const selection = await select(kind, false);
      if (selection.status !== 'success') return selection;
      const result = await fileResult(async () => {
        const bytes = await invoke<ArrayBuffer>('read_selected_file', {
          token: selection.value.token
        });
        return {
          file: new File([bytes], selection.value.name, { type: selection.value.mime }),
          selected: selection.value
        };
      });
      if (result.status !== 'success') await adapter.releaseFile(selection.value);
      return result;
    },
    async chooseSaveFile(kind, defaultName) {
      return select(kind, true, defaultName);
    },
    async writeFile(selected, bytes) {
      return fileResult(() =>
        invoke<void>('write_selected_file', bytes, {
          headers: { 'x-brickpress-token': selected.token }
        })
      );
    },
    async releaseFile(selected) {
      await invoke('release_file', { token: selected.token });
    },
    async exportFile(kind, blob, name): Promise<FileResult<void>> {
      const selection = await adapter.chooseSaveFile(kind, name);
      if (selection.status !== 'success') return selection;
      try {
        return await adapter.writeFile(selection.value, new Uint8Array(await blob.arrayBuffer()));
      } finally {
        await adapter.releaseFile(selection.value);
      }
    },
    async readState(key) {
      return invoke<string | null>('read_state', { key });
    },
    async writeState(key, value) {
      await queue.run(key, () => invoke('write_state', { key, value }));
    },
    async quarantineState(key) {
      await queue.run(key, () => invoke('quarantine_state', { key }));
    },
    async loadTrace() {
      const raw = await invoke<string | null>('read_state', { key: 'trace-metadata' });
      if (!raw) return null;
      const record = JSON.parse(raw) as TraceRecord;
      if (!record || !/^[a-f\d-]{36}$/i.test(record.assetId) || typeof record.mime !== 'string')
        throw new Error('The saved tracing guide is invalid. Upload it again.');
      const bytes = await invoke<ArrayBuffer>('read_trace_asset', { assetId: record.assetId });
      const { assetId, mime, ...metadata } = record;
      const asset = new Blob([bytes], { type: mime });
      const trace = { ...metadata, asset };
      validateTrace(trace);
      persistedTrace = { asset, assetId };
      return trace;
    },
    async saveTrace(trace: TraceOverlay | null) {
      await queue.run('trace-metadata', async () => {
        if (!trace) {
          await invoke('write_state', { key: 'trace-metadata', value: null });
          persistedTrace = null;
          await invoke('prune_trace_assets', { keep: null });
          return;
        }
        validateTrace(trace);
        const assetId =
          persistedTrace?.asset === trace.asset
            ? persistedTrace.assetId
            : await invoke<string>(
                'write_trace_asset',
                new Uint8Array(await trace.asset.arrayBuffer())
              );
        const { asset, url: _url, ...metadata } = trace;
        await invoke('write_state', {
          key: 'trace-metadata',
          value: JSON.stringify({ ...metadata, assetId, mime: asset.type })
        });
        persistedTrace = { asset, assetId };
        await invoke('prune_trace_assets', { keep: assetId });
      });
    },
    async confirmAction(message) {
      return invoke<boolean>('confirm_action', { message });
    },
    async confirmUnsaved(name) {
      return invoke<UnsavedChoice>('confirm_unsaved', { name });
    },
    async confirmConflict() {
      return invoke<boolean>('confirm_conflict');
    },
    async setDocumentTitle(name, dirty) {
      await invoke('set_document_title', { name, dirty });
    },
    async onCloseRequested(handler) {
      return getCurrentWindow().onCloseRequested((event) => {
        event.preventDefault();
        void handler();
      });
    },
    async destroyWindow() {
      await getCurrentWindow().destroy();
    },
    async flush() {
      await queue.flush();
    }
  };
  return adapter;
}
