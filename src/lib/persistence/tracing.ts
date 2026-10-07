export interface TraceImage {
  asset: Blob;
  name: string;
  imageWidth: number;
  imageHeight: number;
  x: number;
  y: number;
  width: number;
  height: number;
  opacity: number;
  visible: boolean;
  locked: boolean;
}
export type TraceOverlay = TraceImage & { url: string };

const formats = /^image\/(png|jpeg|webp|gif|avif|bmp)$/;
export async function readTraceFile(file: File) {
  if (!formats.test(file.type))
    throw new Error('Choose a PNG, JPEG, WebP, GIF, AVIF or BMP image.');
  if (file.size > 20_000_000) throw new Error('Tracing images must be smaller than 20 MB.');
  let width: number, height: number;
  if (typeof createImageBitmap === 'function') {
    const bitmap = await createImageBitmap(file).catch(() => {
      throw new Error('This image could not be decoded. Try PNG or JPEG.');
    });
    width = bitmap.width;
    height = bitmap.height;
    bitmap.close();
  } else {
    const image = new Image(),
      url = URL.createObjectURL(file);
    try {
      await new Promise<void>((resolve, reject) => {
        image.onload = () => resolve();
        image.onerror = () =>
          reject(new Error('This image could not be decoded. Try PNG or JPEG.'));
        image.src = url;
      });
      width = image.naturalWidth;
      height = image.naturalHeight;
    } finally {
      URL.revokeObjectURL(url);
    }
  }
  if (!width || !height || width * height > 40_000_000)
    throw new Error('Use an image up to 40 megapixels.');
  return {
    asset: file as Blob,
    name: file.name.slice(0, 160),
    imageWidth: width,
    imageHeight: height
  };
}

export function validateTrace(value: TraceImage): void {
  if (
    !value ||
    !(value.asset instanceof Blob) ||
    !formats.test(value.asset.type) ||
    value.asset.size > 20_000_000 ||
    typeof value.name !== 'string' ||
    value.name.length > 160 ||
    ![value.imageWidth, value.imageHeight, value.width, value.height].every(
      (n) => Number.isFinite(n) && n > 0
    ) ||
    value.imageWidth * value.imageHeight > 40_000_000 ||
    ![value.x, value.y, value.opacity].every(Number.isFinite) ||
    value.opacity < 0 ||
    value.opacity > 1 ||
    typeof value.visible !== 'boolean' ||
    typeof value.locked !== 'boolean'
  )
    throw new Error('The saved tracing guide is invalid. Upload it again.');
}

/** Browser-local guide storage, kept out of project and artwork export data. */
async function database() {
  return new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open('brickpress-guides', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('tracing');
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(new Error('Tracing storage is unavailable.'));
    request.onblocked = () =>
      reject(new Error('Close other editor tabs to update tracing storage.'));
  });
}

export async function loadTrace(): Promise<TraceImage | null> {
  const db = await database();
  try {
    const value = await new Promise<TraceImage | undefined>((resolve, reject) => {
      const request = db.transaction('tracing').objectStore('tracing').get('current');
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(new Error('The tracing image could not be recovered.'));
    });
    if (!value) return null;
    validateTrace(value);
    return value;
  } finally {
    db.close();
  }
}

export async function saveTrace(trace: TraceOverlay | null) {
  const db = await database();
  try {
    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction('tracing', 'readwrite');
      if (trace) {
        const { url: _url, ...record } = trace;
        transaction.objectStore('tracing').put(record, 'current');
      } else transaction.objectStore('tracing').delete('current');
      transaction.oncomplete = () => resolve();
      transaction.onabort = transaction.onerror = () =>
        reject(new Error('The tracing guide could not be saved on this device.'));
    });
  } finally {
    db.close();
  }
}
