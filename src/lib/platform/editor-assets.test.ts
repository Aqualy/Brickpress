import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Editor } from '../stores/editor.svelte';
import { readTraceFile } from '../persistence/tracing';
import { createDocument, serializeDocument } from '../persistence/document';
import { capturePreset } from '../persistence/presets';
import type { PlatformAdapter } from './types';

vi.mock('../persistence/tracing', async (original) => ({
  ...(await original<typeof import('../persistence/tracing')>()),
  readTraceFile: vi.fn()
}));
const editors: Editor[] = [];
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
function setup() {
  const editor = new Editor();
  editors.push(editor);
  const platform = {
    desktop: true,
    chooseFile: vi.fn(),
    releaseFile: vi.fn(async () => {}),
    writeState: vi.fn(async () => {})
  } as unknown as PlatformAdapter;
  editor.setPlatform(platform);
  return { editor, platform };
}
const image = () => ({
  asset: new Blob(['image'], { type: 'image/png' }),
  name: 'guide.png',
  imageWidth: 32,
  imageHeight: 24
});
beforeEach(() => vi.mocked(readTraceFile).mockReset());
afterEach(() => {
  for (const editor of editors.splice(0)) editor.destroy();
});

describe('desktop asset-operation barrier', () => {
  it('waits for a pending image decode before close-time persistence reads the tracing state', async () => {
    const { editor } = setup(),
      decode = deferred<Awaited<ReturnType<typeof readTraceFile>>>();
    vi.mocked(readTraceFile).mockReturnValueOnce(decode.promise);
    const upload = editor.uploadTrace(new File(['png'], 'guide.png', { type: 'image/png' }));
    let flushed = false;
    const barrier = editor.flushAssets().then(() => {
      flushed = true;
    });
    await Promise.resolve();
    expect(flushed).toBe(false);
    expect(editor.trace).toBeNull();
    decode.resolve(image());
    await upload;
    await barrier;
    expect(flushed).toBe(true);
    expect(editor.trace?.name).toBe('guide.png');
  });
  it.each(['new', 'open'] as const)(
    'never applies an old decode after %s replaces the document',
    async (action) => {
      const { editor } = setup(),
        decode = deferred<Awaited<ReturnType<typeof readTraceFile>>>();
      vi.mocked(readTraceFile).mockReturnValueOnce(decode.promise);
      const upload = editor.uploadTrace(new File(['png'], 'guide.png', { type: 'image/png' }));
      if (action === 'new') editor.newDocument();
      else editor.load(serializeDocument(createDocument()));
      decode.resolve(image());
      await upload;
      await editor.flushAssets();
      expect(editor.trace).toBeNull();
      expect(editor.traceLoading).toBe(false);
    }
  );
  it('cancels a stale native selection that finishes reading after the document is replaced', async () => {
    const { editor, platform } = setup();
    const selection = deferred<Awaited<ReturnType<PlatformAdapter['chooseFile']>>>();
    vi.mocked(platform.chooseFile).mockReturnValueOnce(selection.promise);
    const upload = editor.chooseTracingImage();
    editor.load(serializeDocument(createDocument()));
    const selected = { token: 'stale', path: '/guide.png', name: 'guide.png', mime: 'image/png' };
    selection.resolve({
      status: 'success',
      value: { file: new File(['png'], 'guide.png', { type: 'image/png' }), selected }
    });
    await upload;
    expect(readTraceFile).not.toHaveBeenCalled();
    expect(platform.releaseFile).toHaveBeenCalledWith(selected);
    expect(editor.trace).toBeNull();
  });
  it('waits for both preset reading and its durable library write', async () => {
    const { editor, platform } = setup(),
      text = deferred<string>(),
      write = deferred<void>();
    const preset = capturePreset(editor.doc, 'Pending composition');
    vi.mocked(platform.chooseFile).mockResolvedValueOnce({
      status: 'success',
      value: { file: { text: () => text.promise } as File, selected: null }
    });
    vi.mocked(platform.writeState).mockReturnValueOnce(write.promise);
    const importing = editor.choosePresetFile();
    let flushed = false;
    const barrier = editor.flushAssets().then(() => {
      flushed = true;
    });
    await Promise.resolve();
    expect(flushed).toBe(false);
    text.resolve(JSON.stringify([preset]));
    await vi.waitFor(() => expect(platform.writeState).toHaveBeenCalled());
    expect(editor.presets).toHaveLength(0);
    expect(flushed).toBe(false);
    write.resolve();
    await importing;
    await barrier;
    expect(editor.presets[0].name).toBe('Pending composition');
    expect(flushed).toBe(true);
  });
});
