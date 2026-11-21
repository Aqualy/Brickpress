//! Offscreen rendering to RGBA pixels and PNG, tiled for large exports.

use crate::board2d::{FrameOptions, Renderer2D, View2D};
use crate::gpu::GpuError;
use crate::scene::{board_params, build_pieces, Overlay, RenderMode};
use crate::wgpu;
use brickpress_core::persistence::preferences::GridAppearance;
use brickpress_core::PressDocument;

pub const MAX_SIDE: u32 = 12_000;
pub const MAX_PIXELS: u64 = 40_000_000;
const TILE: u32 = 2048;
pub const EXPORT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const GRID_COLOR: [f32; 4] = [0.455, 0.494, 0.553, 1.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportMode {
    Design,
    Print,
}

impl ExportMode {
    pub fn render_mode(self) -> RenderMode {
        match self {
            ExportMode::Design => RenderMode::Design,
            ExportMode::Print => RenderMode::Print,
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            ExportMode::Design => "design",
            ExportMode::Print => "print",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ExportOptions {
    pub mode: ExportMode,
    pub paper: bool,
    pub grid: bool,
    /// Output width in pixels; the height follows the board's aspect ratio.
    pub width: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Use dimensions up to 12,000 px and 40 megapixels.")]
    Dimensions,
    #[error(transparent)]
    Gpu(#[from] GpuError),
    #[error("PNG encoding failed: {0}")]
    Encode(String),
}

/// Validated output size, preserving the board's aspect ratio.
pub fn export_dimensions(doc: &PressDocument, width: u32) -> Result<(u32, u32), ExportError> {
    let height = ((width as f64 * doc.board.height as f64) / doc.board.width as f64).round() as u32;
    if !(64..=MAX_SIDE).contains(&width)
        || height > MAX_SIDE
        || height == 0
        || width as u64 * height as u64 > MAX_PIXELS
    {
        return Err(ExportError::Dimensions);
    }
    Ok((width, height))
}

pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    /// Straight (not premultiplied) sRGB RGBA.
    pub pixels: Vec<u8>,
}

/// Renders the document offscreen with the given renderer, which must target
/// [`EXPORT_FORMAT`].
pub fn render_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &Renderer2D,
    doc: &PressDocument,
    appearance: GridAppearance,
    options: &ExportOptions,
) -> Result<RgbaImage, ExportError> {
    let (width, height) = export_dimensions(doc, options.width)?;
    let mode = options.mode.render_mode();
    let params = board_params(doc, mode, options.paper, options.grid, appearance);
    let instances = build_pieces(&renderer.atlas, doc, mode, &Overlay::default());
    let stud_px = width as f32 / doc.board.width as f32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let mut frame = renderer.new_frame(device);
    let frame_options = FrameOptions { grid_color: GRID_COLOR, grid: options.grid, paper: options.paper };

    for ty in (0..height).step_by(TILE as usize) {
        for tx in (0..width).step_by(TILE as usize) {
            let (tw, th) = (TILE.min(width - tx), TILE.min(height - ty));
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("brickpress export tile"),
                size: wgpu::Extent3d { width: tw, height: th, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: EXPORT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = View2D { origin: [-(tx as f32), -(ty as f32)], stud_px, target: [tw as f32, th as f32] };
            renderer.prepare(device, queue, &mut frame, &view, &params, &frame_options, 0, &instances, &[], None);
            let target = texture.create_view(&Default::default());
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("brickpress export"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
                renderer.paint(&mut pass, &frame, options.paper || options.grid);
            }
            let tile = read_texture(device, queue, encoder, &texture, tw, th)?;
            for row in 0..th {
                let src = &tile[(row * tw * 4) as usize..((row + 1) * tw * 4) as usize];
                let start = (((ty + row) * width + tx) * 4) as usize;
                pixels[start..start + src.len()].copy_from_slice(src);
            }
        }
    }
    // Premultiplied → straight alpha.
    if !options.paper {
        for px in pixels.as_chunks_mut::<4>().0 {
            let a = px[3] as u32;
            if a > 0 && a < 255 {
                for c in &mut px[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
    }
    Ok(RgbaImage { width, height, pixels })
}

/// Copies a texture to the CPU, removing row padding.
pub fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, GpuError> {
    let unpadded = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("brickpress readback"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| GpuError::Render(e.to_string()))?;
    rx.recv().map_err(|e| GpuError::Render(e.to_string()))?.map_err(|e| GpuError::Render(e.to_string()))?;
    let mut out = Vec::with_capacity((unpadded * height) as usize);
    {
        let data = slice.get_mapped_range().map_err(|e| GpuError::Render(e.to_string()))?;
        for row in data.chunks(padded as usize) {
            out.extend_from_slice(&row[..unpadded as usize]);
        }
    }
    buffer.unmap();
    Ok(out)
}

pub fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, ExportError> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        let mut writer = encoder.write_header().map_err(|e| ExportError::Encode(e.to_string()))?;
        writer.write_image_data(&image.pixels).map_err(|e| ExportError::Encode(e.to_string()))?;
    }
    Ok(out)
}

/// `<name>-<mode>.<ext>`, with the name slugged like 1.x.
pub fn export_filename(name: &str, mode: ExportMode, extension: &str) -> String {
    format!("{}-{}.{extension}", slug(name), mode.id())
}

/// A file-name-safe version of a document name (1.x rules).
pub fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "impression".into()
    } else {
        slug.into()
    }
}
