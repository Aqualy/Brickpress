import { afterEach, describe, expect, it, vi } from 'vitest';
import { readTraceFile } from './tracing';

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
describe('tracing image decoding', () => {
  it('uses an image element when the native webview has no createImageBitmap', async () => {
    vi.stubGlobal('createImageBitmap', undefined);
    const revoke = vi.spyOn(URL, 'revokeObjectURL');
    class Decoder {
      naturalWidth = 120;
      naturalHeight = 80;
      onload?: () => void;
      set src(_value: string) {
        queueMicrotask(() => this.onload?.());
      }
    }
    vi.stubGlobal('Image', Decoder);
    const file = new File(['image'], 'épreuve.png', { type: 'image/png' });
    expect(await readTraceFile(file)).toMatchObject({
      asset: file,
      name: 'épreuve.png',
      imageWidth: 120,
      imageHeight: 80
    });
    expect(revoke).toHaveBeenCalledOnce();
  });
  it('releases fallback Blob URLs when decoding fails', async () => {
    vi.stubGlobal('createImageBitmap', undefined);
    const revoke = vi.spyOn(URL, 'revokeObjectURL');
    class Decoder {
      onerror?: () => void;
      set src(_value: string) {
        queueMicrotask(() => this.onerror?.());
      }
    }
    vi.stubGlobal('Image', Decoder);
    await expect(
      readTraceFile(new File(['broken'], 'broken.png', { type: 'image/png' }))
    ).rejects.toThrow('could not be decoded');
    expect(revoke).toHaveBeenCalledOnce();
  });
  it('enforces image dimensions even when fallback decoding succeeds', async () => {
    vi.stubGlobal('createImageBitmap', undefined);
    class Decoder {
      naturalWidth = 10000;
      naturalHeight = 10000;
      onload?: () => void;
      set src(_value: string) {
        queueMicrotask(() => this.onload?.());
      }
    }
    vi.stubGlobal('Image', Decoder);
    await expect(
      readTraceFile(new File(['image'], 'huge.png', { type: 'image/png' }))
    ).rejects.toThrow('40 megapixels');
  });
  it('closes decoded bitmaps and rejects formats that cannot be used as tracing assets', async () => {
    const close = vi.fn();
    vi.stubGlobal(
      'createImageBitmap',
      vi.fn(async () => ({ width: 32, height: 24, close }))
    );
    expect(
      await readTraceFile(new File(['image'], 'guide.webp', { type: 'image/webp' }))
    ).toMatchObject({ imageWidth: 32, imageHeight: 24 });
    expect(close).toHaveBeenCalledOnce();
    await expect(
      readTraceFile(new File(['svg'], 'guide.svg', { type: 'image/svg+xml' }))
    ).rejects.toThrow('Choose a PNG');
  });
});
