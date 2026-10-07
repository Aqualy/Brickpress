let decoderReady: Promise<typeof import('@jsquash/avif/decode.js')> | undefined;

/** Load only the decoder, with its WASM asset served from this application's bundle. */
async function decoder() {
  decoderReady ??= Promise.all([
    import('@jsquash/avif/decode.js'),
    import('@jsquash/avif/codec/dec/avif_dec.wasm?url')
  ])
    .then(async ([module, wasm]) => {
      await module.init({ locateFile: () => wasm.default });
      return module;
    })
    .catch((error) => {
      decoderReady = undefined;
      throw error;
    });
  return decoderReady;
}

/** Normalize an unsupported AVIF guide to lossless PNG so it renders after recovery too. */
export async function decodeAvifGuide(file: Blob) {
  const module = await decoder();
  const image = await module.default(await file.arrayBuffer());
  if (!image) throw new Error('This AVIF image could not be decoded. Try PNG or JPEG.');
  if (!image.width || !image.height || image.width * image.height > 40_000_000)
    throw new Error('Use an image up to 40 megapixels.');
  const canvas = document.createElement('canvas');
  canvas.width = image.width;
  canvas.height = image.height;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('Tracing image conversion is unavailable. Try PNG or JPEG.');
  context.putImageData(image, 0, 0);
  const asset = await new Promise<Blob>((resolve, reject) =>
    canvas.toBlob((blob) => {
      if (blob) resolve(blob);
      else reject(new Error('The tracing image could not be converted. Try PNG or JPEG.'));
    }, 'image/png')
  );
  if (asset.size > 20_000_000)
    throw new Error('The decoded tracing image exceeds 20 MB. Use a smaller image.');
  return { asset, width: image.width, height: image.height };
}
