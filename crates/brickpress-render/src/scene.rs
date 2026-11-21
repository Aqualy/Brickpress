//! CPU-side description of a frame: per-piece instances plus board settings.
//! Built once per document change (not per frame) and uploaded as-is.

use crate::atlas::{ShapeAtlas, PAD};
use crate::print::{erosion, impression, jitter_matrix, paper_color, pass_offset};
use brickpress_core::catalog::catalog;
use brickpress_core::geometry::piece_matrix;
use brickpress_core::persistence::preferences::GridAppearance;
use brickpress_core::{parse_color, GridStyle, PlacedPiece, PressDocument};
use bytemuck::{Pod, Zeroable};
use std::collections::HashSet;

pub const FLAG_OUTLINE: u32 = 1;
pub const FLAG_GHOST: u32 = 2;
pub const FLAG_INVALID: u32 = 4;
pub const FLAG_DIM: u32 = 8;

/// One placed surface, as the vertex shader reads it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Instance {
    /// Canonical piece coordinates → board studs: (a, b, c, d) and (e, f).
    pub m0: [f32; 4],
    pub m1: [f32; 2],
    /// Canonical footprint (w, h).
    pub size: [f32; 2],
    /// Atlas uv transform: uv = xy + (local + PAD) * zw.
    pub atlas: [f32; 4],
    pub color: [f32; 4],
    /// seed (as float bits are not portable, so a u32), flags.
    pub seed: u32,
    pub flags: u32,
    pub opacity: f32,
    pub _pad: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenderMode {
    Design,
    Print,
}

/// Uniform parameters for a frame (besides the view transform).
#[derive(Clone, Copy, Debug)]
pub struct BoardParams {
    pub board: [f32; 2],
    pub mode: RenderMode,
    pub paper_on: bool,
    pub paper_rgb: [f32; 3],
    pub paper_grain: f32,
    pub paper_tooth: f32,
    pub paper_fibers: f32,
    pub paper_seed: u32,
    pub grid: GridStyle,
    pub major_grid: i32,
    pub embossed: bool,
    pub show_grid: bool,
    pub print: [f32; 12],
    pub gap: f32,
}

pub fn srgb(color: &str) -> [f32; 3] {
    parse_color(color).unwrap_or([0, 0, 0]).map(|c| c as f32 / 255.0)
}

/// Everything the 2D renderer draws for one document state.
pub struct Scene2D {
    pub key: u64,
    pub params: BoardParams,
    pub pieces: Vec<Instance>,
}

/// Optional on-canvas state drawn with the document.
#[derive(Default)]
pub struct Overlay<'a> {
    pub selected: Option<&'a HashSet<&'a str>>,
    /// Offset applied to selected pieces while dragging.
    pub drag: (i32, i32),
    pub drag_valid: bool,
}

pub fn instance(
    atlas: &ShapeAtlas,
    p: &PlacedPiece,
    color: [f32; 3],
    alpha: f32,
    flags: u32,
    matrix: Option<[f64; 6]>,
) -> Option<Instance> {
    let index = catalog().piece_index(&p.piece_id)?;
    let piece = &catalog().pieces[index];
    let m = matrix.unwrap_or_else(|| piece_matrix(piece, p, 1.0));
    Some(Instance {
        m0: [m[0] as f32, m[1] as f32, m[2] as f32, m[3] as f32],
        m1: [m[4] as f32, m[5] as f32],
        size: [piece.footprint.width_studs as f32, piece.footprint.height_studs as f32],
        atlas: atlas.rects[index].uv_transform(),
        color: [color[0], color[1], color[2], alpha],
        seed: (p.seed as u64 & 0xffff_ffff) as u32,
        flags,
        opacity: 1.0,
        _pad: 0.0,
    })
}

pub fn board_params(
    doc: &PressDocument,
    mode: RenderMode,
    paper_on: bool,
    show_grid: bool,
    appearance: GridAppearance,
) -> BoardParams {
    let s = &doc.print_settings;
    let paper = paper_color(&doc.paper).map(|c| c as f32 / 255.0);
    BoardParams {
        board: [doc.board.width as f32, doc.board.height as f32],
        mode,
        paper_on,
        paper_rgb: paper,
        paper_grain: doc.paper.grain as f32,
        paper_tooth: doc.paper.tooth as f32,
        paper_fibers: doc.paper.fibers as f32,
        paper_seed: (doc.paper.seed as u64 & 0xffff_ffff) as u32,
        grid: doc.options.grid,
        major_grid: doc.options.major_grid,
        embossed: appearance == GridAppearance::Embossed,
        show_grid,
        print: [
            s.ink_amount as f32,
            s.pressure as f32,
            s.ink_coverage as f32,
            s.roller_consistency as f32,
            s.piece_wear as f32,
            s.edge_wear as f32,
            s.paper_tooth as f32,
            s.texture_scale as f32,
            s.misalignment as f32,
            erosion(s) as f32,
            s.print_roughness as f32,
            0.0,
        ],
        gap: catalog().design_gap_studs() as f32,
    }
}

/// Builds instances for every visible pass in paint order. In print mode each
/// piece gets its seeded jitter and opacity and each pass its registration.
pub fn build_pieces(atlas: &ShapeAtlas, doc: &PressDocument, mode: RenderMode, overlay: &Overlay) -> Vec<Instance> {
    let scale = catalog().surface_scale();
    let mut out = Vec::with_capacity(doc.piece_count());
    for pass in doc.passes.iter().filter(|p| p.visible) {
        let color = srgb(&pass.color);
        let offset = if mode == RenderMode::Print { pass_offset(pass, &doc.print_settings) } else { (0.0, 0.0) };
        let dim = if pass.locked && mode == RenderMode::Design { FLAG_DIM } else { 0 };
        for p in pass.pieces.iter() {
            let selected = overlay.selected.is_some_and(|s| s.contains(&*p.uid));
            let moved;
            let p = if selected && overlay.drag != (0, 0) {
                moved = PlacedPiece { x: p.x + overlay.drag.0, y: p.y + overlay.drag.1, ..p.clone() };
                &moved
            } else {
                p
            };
            let Some(index) = catalog().piece_index(&p.piece_id) else { continue };
            let piece = &catalog().pieces[index];
            let mut instance = match mode {
                RenderMode::Design => instance(atlas, p, color, 1.0, dim, None),
                RenderMode::Print => {
                    let imp = impression(p, &doc.print_settings);
                    let m = jitter_matrix(piece_matrix(piece, p, scale), &imp, p, offset);
                    instance(atlas, p, color, 1.0, 0, Some(m)).map(|mut i| {
                        i.opacity = imp.opacity as f32;
                        i
                    })
                }
            };
            if let Some(i) = instance.take() {
                out.push(i);
            }
        }
    }
    out
}

/// Outline instances for the selection (drawn above all ink).
pub fn build_outlines(
    atlas: &ShapeAtlas,
    doc: &PressDocument,
    overlay: &Overlay,
    accent: [f32; 3],
    invalid: [f32; 3],
) -> Vec<Instance> {
    let Some(selected) = overlay.selected else { return vec![] };
    let color = if overlay.drag_valid || overlay.drag == (0, 0) { accent } else { invalid };
    doc.all_pieces()
        .filter(|(pass, p)| pass.visible && selected.contains(&*p.uid))
        .filter_map(|(_, p)| {
            let p = PlacedPiece { x: p.x + overlay.drag.0, y: p.y + overlay.drag.1, ..p.clone() };
            instance(atlas, &p, color, 1.0, FLAG_OUTLINE, None)
        })
        .collect()
}

/// Ghost previews of a pending placement.
pub fn build_ghosts(
    atlas: &ShapeAtlas,
    ghosts: &[(PlacedPiece, [f32; 3])],
    valid: bool,
    invalid: [f32; 3],
) -> Vec<Instance> {
    ghosts
        .iter()
        .filter_map(|(p, color)| {
            let flags = FLAG_GHOST | if valid { 0 } else { FLAG_INVALID };
            instance(atlas, p, if valid { *color } else { invalid }, if valid { 0.62 } else { 0.5 }, flags, None)
        })
        .collect()
}

pub const _PAD: f32 = PAD;

fn invert(m: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    let det = m[0] * m[3] - m[1] * m[2];
    let (dx, dy) = (x - m[4], y - m[5]);
    ((m[3] * dx - m[2] * dy) / det, (-m[1] * dx + m[0] * dy) / det)
}

/// Is board point (x, y) on the printing surface of `p`?
pub fn piece_contains(atlas: &ShapeAtlas, p: &PlacedPiece, x: f64, y: f64) -> bool {
    let Some(index) = catalog().piece_index(&p.piece_id) else { return false };
    let b = brickpress_core::geometry::bounds(p);
    if x < b.x as f64 || y < b.y as f64 || x > b.right() as f64 || y > b.bottom() as f64 {
        return false;
    }
    let m = piece_matrix(&catalog().pieces[index], p, 1.0);
    let (lx, ly) = invert(&m, x, y);
    atlas.sample(atlas.rects[index], lx as f32, ly as f32) < 0.0
}

/// The topmost visible piece under a board point (exact surface, not its box).
pub fn hit_test<'a>(
    atlas: &ShapeAtlas,
    doc: &'a PressDocument,
    x: f64,
    y: f64,
) -> Option<(&'a brickpress_core::InkPass, &'a PlacedPiece)> {
    for pass in doc.passes.iter().rev().filter(|p| p.visible) {
        for p in pass.pieces.iter().rev() {
            if piece_contains(atlas, p, x, y) {
                return Some((pass, p));
            }
        }
    }
    None
}

/// A small CPU-rendered design preview (premultiplied RGBA), for thumbnails
/// of presets and documents.
pub fn preview_rgba(
    atlas: &ShapeAtlas,
    pieces: &[(PlacedPiece, [u8; 3])],
    width: f64,
    height: f64,
    size: u32,
) -> (u32, u32, Vec<u8>) {
    let scale = size as f64 * 0.9 / width.max(height).max(1.0);
    let (w, h) = (size, size);
    let (ox, oy) = ((w as f64 - width * scale) / 2.0, (h as f64 - height * scale) / 2.0);
    let mut out = vec![0u8; (w * h * 4) as usize];
    let gap = catalog().design_gap_studs() as f32 / 2.0;
    for (p, rgb) in pieces {
        let Some(index) = catalog().piece_index(&p.piece_id) else { continue };
        let m = piece_matrix(&catalog().pieces[index], p, 1.0);
        let b = brickpress_core::geometry::bounds(p);
        let x0 = ((b.x as f64 * scale + ox).floor().max(0.0)) as u32;
        let y0 = ((b.y as f64 * scale + oy).floor().max(0.0)) as u32;
        let x1 = ((b.right() as f64 * scale + ox).ceil() as u32).min(w);
        let y1 = ((b.bottom() as f64 * scale + oy).ceil() as u32).min(h);
        let aa = (1.0 / scale) as f32;
        for py in y0..y1 {
            for px in x0..x1 {
                let bx = (px as f64 + 0.5 - ox) / scale;
                let by = (py as f64 + 0.5 - oy) / scale;
                let (lx, ly) = invert(&m, bx, by);
                let d = atlas.sample(atlas.rects[index], lx as f32, ly as f32) + gap;
                let a = (0.5 - d / aa).clamp(0.0, 1.0);
                if a <= 0.0 {
                    continue;
                }
                let i = ((py * w + px) * 4) as usize;
                for c in 0..3 {
                    out[i + c] = (rgb[c] as f32 * a + out[i + c] as f32 * (1.0 - a)) as u8;
                }
                out[i + 3] = (255.0 * a + out[i + 3] as f32 * (1.0 - a)) as u8;
            }
        }
    }
    (w, h, out)
}
