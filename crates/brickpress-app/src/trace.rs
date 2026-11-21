//! Tracing guide images: decoding with the 1.x limits (20 MB, 40 megapixels).

use crate::gpu::TraceUpload;
use crate::storage;
use brickpress_core::persistence::tracing::{mime_for_extension, MAX_PIXELS, MAX_TRACE_BYTES};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Decodes image bytes into an upload for the renderer.
pub fn decode(bytes: &[u8]) -> Result<TraceUpload, String> {
    if bytes.len() as u64 > MAX_TRACE_BYTES {
        return Err("Tracing images must be smaller than 20 MB.".into());
    }
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "This image could not be read.".to_string())?;
    if reader.format() == Some(image::ImageFormat::Avif) {
        return Err("AVIF guides are not supported yet. Convert the image to PNG or JPEG.".into());
    }
    let (w, h) =
        reader.into_dimensions().map_err(|_| "This image could not be decoded. Try PNG or JPEG.".to_string())?;
    if w == 0 || h == 0 || w as u64 * h as u64 > MAX_PIXELS {
        return Err("Use an image up to 40 megapixels.".into());
    }
    let image =
        image::load_from_memory(bytes).map_err(|_| "This image could not be decoded. Try PNG or JPEG.".to_string())?;
    let rgba = image.to_rgba8();
    Ok(TraceUpload {
        id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        size: [rgba.width(), rgba.height()],
        rgba: Arc::new(rgba.into_raw()),
    })
}

/// Reads and decodes a guide chosen by the user. Returns the original bytes
/// (stored on the device), the decoded upload and the MIME type.
pub fn load_file(path: &Path) -> Result<(Vec<u8>, TraceUpload, &'static str), String> {
    let mime = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(mime_for_extension)
        .ok_or("Choose a PNG, JPEG, WebP, GIF, AVIF or BMP image.")?;
    let (bytes, _) = storage::read_limited(path, MAX_TRACE_BYTES).map_err(|e| match e {
        storage::StorageError::TooLarge(_) => "Tracing images must be smaller than 20 MB.".to_string(),
        e => e.to_string(),
    })?;
    let upload = decode(&bytes)?;
    Ok((bytes, upload, mime))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_png_and_rejects_garbage() {
        let mut png = Vec::new();
        image::RgbaImage::from_pixel(3, 2, image::Rgba([10, 20, 30, 255]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let upload = decode(&png).unwrap();
        assert_eq!(upload.size, [3, 2]);
        assert_eq!(&upload.rgba[..4], &[10, 20, 30, 255]);
        assert!(decode(b"not an image").is_err());
    }
}
