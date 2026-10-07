import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { createNativePlatform } from './native';
import type { TraceOverlay } from '../persistence/tracing';

const windowApi = vi.hoisted(() => ({
  onCloseRequested: vi.fn(),
  destroy: vi.fn(async () => {})
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => windowApi }));

const selected = {
  token: 'opaque',
  path: '/Users/版画/épreuve.brickpress.json',
  name: 'épreuve.brickpress.json',
  mime: 'application/json'
};
const trace = (asset = new Blob(['png'], { type: 'image/png' })): TraceOverlay => ({
  asset,
  url: 'blob:guide',
  name: 'guide.png',
  imageWidth: 100,
  imageHeight: 80,
  x: 0,
  y: 0,
  width: 10,
  height: 8,
  opacity: 0.4,
  locked: true,
  visible: true
});
beforeEach(() => {
  vi.mocked(invoke).mockReset();
  windowApi.onCloseRequested.mockReset();
  windowApi.destroy.mockClear();
});

describe('native platform binary bridge', () => {
  it('distinguishes picker cancellation from a native failure', async () => {
    const adapter = createNativePlatform();
    vi.mocked(invoke).mockResolvedValueOnce(null);
    expect(await adapter.chooseFile('project')).toEqual({ status: 'cancelled' });
    vi.mocked(invoke).mockRejectedValueOnce({ code: 'io', message: 'Picker failed' });
    expect(await adapter.chooseFile('project')).toEqual({
      status: 'error',
      error: { code: 'io', message: 'Picker failed' }
    });
  });
  it('reads selected files as binary and retains Unicode names without permitting arbitrary paths', async () => {
    const adapter = createNativePlatform();
    vi.mocked(invoke)
      .mockResolvedValueOnce(selected)
      .mockResolvedValueOnce(new TextEncoder().encode('{"version":1}').buffer);
    const result = await adapter.chooseFile('project');
    expect(result.status).toBe('success');
    if (result.status === 'success') {
      expect(result.value.file.name).toBe(selected.name);
      expect(await result.value.file.text()).toBe('{"version":1}');
    }
    expect(invoke).toHaveBeenLastCalledWith('read_selected_file', { token: 'opaque' });
  });
  it('releases candidate handles when reading fails', async () => {
    const adapter = createNativePlatform();
    vi.mocked(invoke)
      .mockResolvedValueOnce(selected)
      .mockRejectedValueOnce({ code: 'conflict', message: 'Changed while opening' })
      .mockResolvedValueOnce(undefined);
    expect(await adapter.chooseFile('project')).toMatchObject({
      status: 'error',
      error: { code: 'conflict' }
    });
    expect(invoke).toHaveBeenLastCalledWith('release_file', { token: 'opaque' });
  });
  it('exports raw bytes with an opaque token header and releases the output handle', async () => {
    const adapter = createNativePlatform();
    vi.mocked(invoke)
      .mockResolvedValueOnce(selected)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined);
    expect(
      await adapter.exportFile('png', new Blob([new Uint8Array([0, 255, 129])]), 'image.png')
    ).toEqual({ status: 'success', value: undefined });
    const write = vi.mocked(invoke).mock.calls[1];
    expect(write).toEqual([
      'write_selected_file',
      new Uint8Array([0, 255, 129]),
      { headers: { 'x-brickpress-token': 'opaque' } }
    ]);
    expect(invoke).toHaveBeenLastCalledWith('release_file', { token: 'opaque' });
  });
  it('prevents native close immediately and delegates asynchronous protection', async () => {
    const adapter = createNativePlatform(),
      protect = vi.fn(async () => {}),
      stop = vi.fn();
    windowApi.onCloseRequested.mockImplementationOnce(
      async (listener: (event: { preventDefault: () => void }) => void) => {
        const prevent = vi.fn();
        listener({ preventDefault: prevent });
        expect(prevent).toHaveBeenCalledOnce();
        return stop;
      }
    );
    expect(await adapter.onCloseRequested(protect)).toBe(stop);
    expect(protect).toHaveBeenCalledOnce();
    expect(windowApi.destroy).not.toHaveBeenCalled();
  });
});

describe('native tracing persistence', () => {
  it('stages binary assets, commits metadata, then prunes old assets', async () => {
    const adapter = createNativePlatform(),
      guide = trace();
    vi.mocked(invoke)
      .mockResolvedValueOnce('asset-one')
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined);
    await adapter.saveTrace(guide);
    const calls = vi.mocked(invoke).mock.calls;
    expect(calls.map((call) => call[0])).toEqual([
      'write_trace_asset',
      'write_state',
      'prune_trace_assets'
    ]);
    expect(calls[0][1]).toBeInstanceOf(Uint8Array);
    const metadata = JSON.parse((calls[1][1] as { value: string }).value);
    expect(metadata).toMatchObject({ assetId: 'asset-one', mime: 'image/png', opacity: 0.4 });
    expect(metadata).not.toHaveProperty('asset');
    expect(metadata).not.toHaveProperty('url');
    expect(calls[2][1]).toEqual({ keep: 'asset-one' });
  });
  it('preserves the previous metadata and asset on a failed replacement and can retry', async () => {
    const adapter = createNativePlatform(),
      first = trace(),
      second = trace(new Blob(['new'], { type: 'image/png' }));
    vi.mocked(invoke)
      .mockResolvedValueOnce('asset-one')
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined);
    await adapter.saveTrace(first);
    vi.mocked(invoke)
      .mockResolvedValueOnce('asset-two')
      .mockRejectedValueOnce(new Error('Metadata write failed'));
    await expect(adapter.saveTrace(second)).rejects.toThrow('Metadata write failed');
    expect(
      vi.mocked(invoke).mock.calls.filter((call) => call[0] === 'prune_trace_assets')
    ).toHaveLength(1);
    await expect(adapter.flush()).rejects.toThrow('Metadata write failed');
    vi.mocked(invoke).mockResolvedValueOnce(undefined).mockResolvedValueOnce(undefined);
    await adapter.saveTrace(first);
    expect(
      vi.mocked(invoke).mock.calls.filter((call) => call[0] === 'write_trace_asset')
    ).toHaveLength(2);
    expect(invoke).toHaveBeenLastCalledWith('prune_trace_assets', { keep: 'asset-one' });
    await adapter.flush();
  });
  it('changes tracing controls without rewriting the unchanged binary asset', async () => {
    const adapter = createNativePlatform(),
      guide = trace();
    vi.mocked(invoke).mockResolvedValueOnce('asset-one').mockResolvedValue(undefined);
    await adapter.saveTrace(guide);
    await adapter.saveTrace({ ...guide, opacity: 0.7, x: 3 });
    expect(
      vi.mocked(invoke).mock.calls.filter((call) => call[0] === 'write_trace_asset')
    ).toHaveLength(1);
    const records = vi.mocked(invoke).mock.calls.filter((call) => call[0] === 'write_state');
    expect(JSON.parse((records[1][1] as { value: string }).value)).toMatchObject({
      opacity: 0.7,
      x: 3
    });
  });
});
