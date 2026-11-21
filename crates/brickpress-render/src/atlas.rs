//! Signed-distance fields for every catalog piece, packed into one atlas.
//!
//! Distances are in studs of the canonical (unrotated, unscaled) piece and are
//! negative inside the surface. The shader insets, erodes, outlines and
//! anti-aliases from this one field, at any zoom, without per-piece geometry.

use brickpress_core::catalog::{catalog, FillRule};
use brickpress_core::silhouette::{parse_contours_with, Point};
use rayon::prelude::*;

/// Texels per stud.
pub const RESOLUTION: u32 = 64;
/// Distance field margin around each footprint, in studs.
pub const PAD: f32 = 0.25;
/// Atlas side length in texels.
pub const ATLAS_SIZE: u32 = 2048;

#[derive(Clone, Copy, Debug)]
pub struct AtlasRect {
    /// Top-left texel.
    pub x: u32,
    pub y: u32,
    /// Size in texels, including the margin.
    pub width: u32,
    pub height: u32,
}

impl AtlasRect {
    /// `[u0, v0, du, dv]`: uv = (u0, v0) + (local + PAD) * (du, dv).
    pub fn uv_transform(&self) -> [f32; 4] {
        let s = RESOLUTION as f32 / ATLAS_SIZE as f32;
        [self.x as f32 / ATLAS_SIZE as f32, self.y as f32 / ATLAS_SIZE as f32, s, s]
    }
}

pub struct ShapeAtlas {
    /// Distances in studs, row-major, `ATLAS_SIZE²` values (f32 on the CPU).
    pub texels: Vec<f32>,
    /// Indexed like `catalog().pieces`.
    pub rects: Vec<AtlasRect>,
}

fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 { (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (cx, cy) = (a.x + t * dx - p.x, a.y + t * dy - p.y);
    (cx * cx + cy * cy).sqrt()
}

fn inside(p: Point, contours: &[Vec<Point>], rule: FillRule) -> bool {
    let mut winding = 0i32;
    for c in contours {
        for i in 0..c.len() {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            let side = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
            if a.y <= p.y && b.y > p.y && side > 0.0 {
                winding += 1;
            } else if a.y > p.y && b.y <= p.y && side < 0.0 {
                winding -= 1;
            }
        }
    }
    match rule {
        FillRule::Evenodd => winding % 2 != 0,
        FillRule::Nonzero => winding != 0,
    }
}

/// Exact signed distance from `p` to the contours.
pub fn signed_distance(p: Point, contours: &[Vec<Point>], rule: FillRule) -> f64 {
    let mut best = f64::INFINITY;
    for c in contours {
        for i in 0..c.len() {
            best = best.min(segment_distance(p, c[i], c[(i + 1) % c.len()]));
        }
    }
    if inside(p, contours, rule) {
        -best
    } else {
        best
    }
}

/// Texel size of a piece footprint including the margin.
fn texel_size(w: i32, h: i32) -> (u32, u32) {
    let pad = (PAD * RESOLUTION as f32) as u32;
    (w as u32 * RESOLUTION + 2 * pad, h as u32 * RESOLUTION + 2 * pad)
}

impl ShapeAtlas {
    pub fn build() -> Self {
        let pieces = &catalog().pieces;
        // Shelf packing, tallest first.
        let mut order: Vec<usize> = (0..pieces.len()).collect();
        order.sort_by_key(|&i| {
            let f = &pieces[i].footprint;
            std::cmp::Reverse((texel_size(f.width_studs, f.height_studs).1, f.width_studs))
        });
        let mut rects = vec![AtlasRect { x: 0, y: 0, width: 0, height: 0 }; pieces.len()];
        let (mut x, mut y, mut shelf) = (0u32, 0u32, 0u32);
        for &i in &order {
            let f = &pieces[i].footprint;
            let (w, h) = texel_size(f.width_studs, f.height_studs);
            if x + w > ATLAS_SIZE {
                x = 0;
                y += shelf;
                shelf = 0;
            }
            assert!(y + h <= ATLAS_SIZE, "shape atlas overflow");
            rects[i] = AtlasRect { x, y, width: w, height: h };
            x += w;
            shelf = shelf.max(h);
        }
        // A finer flattening than collision needs: the field must follow curves exactly.
        let contours: Vec<Option<Vec<Vec<Point>>>> =
            pieces.iter().map(|p| parse_contours_with(&p.geometry.path, 2e-4).ok()).collect();
        let size = ATLAS_SIZE as usize;
        let mut texels = vec![PAD; size * size];
        let rows: Vec<(usize, Vec<f32>)> = (0..pieces.len())
            .into_par_iter()
            .flat_map_iter(|i| {
                let rect = rects[i];
                let rule = pieces[i].geometry.fill_rule;
                let contours = contours[i].clone();
                (0..rect.height).map(move |ty| {
                    let row: Vec<f32> = (0..rect.width)
                        .map(|tx| match &contours {
                            Some(c) => {
                                let p = Point {
                                    x: (tx as f64 + 0.5) / RESOLUTION as f64 - PAD as f64,
                                    y: (ty as f64 + 0.5) / RESOLUTION as f64 - PAD as f64,
                                };
                                (signed_distance(p, c, rule) as f32).clamp(-PAD * 4.0, PAD)
                            }
                            None => PAD,
                        })
                        .collect();
                    ((rect.y + ty) as usize * size + rect.x as usize, row)
                })
            })
            .collect();
        for (offset, row) in rows {
            texels[offset..offset + row.len()].copy_from_slice(&row);
        }
        Self { texels, rects }
    }

    pub fn texels_f16(&self) -> Vec<half::f16> {
        self.texels.iter().map(|&v| half::f16::from_f32(v)).collect()
    }

    /// An alpha-mask preview of a piece, `size` pixels on its longer side,
    /// fitted with a margin. Used for library thumbnails.
    pub fn thumbnail(&self, piece: usize, size: u32) -> (u32, u32, Vec<u8>) {
        let f = &catalog().pieces[piece].footprint;
        let (w, h) = (f.width_studs as f32, f.height_studs as f32);
        let scale = (size as f32 * 0.86) / w.max(h);
        let (pw, ph) = (size, size);
        let rect = self.rects[piece];
        let mut out = vec![0u8; (pw * ph) as usize];
        for py in 0..ph {
            for px in 0..pw {
                let lx = (px as f32 + 0.5 - (pw as f32 - w * scale) / 2.0) / scale;
                let ly = (py as f32 + 0.5 - (ph as f32 - h * scale) / 2.0) / scale;
                let d = self.sample(rect, lx, ly);
                let aa = 1.0 / scale;
                out[(py * pw + px) as usize] = ((0.5 - d / aa).clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
        (pw, ph, out)
    }

    /// Bilinear sample of a piece's field at canonical local coordinates.
    pub fn sample(&self, rect: AtlasRect, x: f32, y: f32) -> f32 {
        let fx = (x + PAD) * RESOLUTION as f32 - 0.5;
        let fy = (y + PAD) * RESOLUTION as f32 - 0.5;
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let at = |ix: f32, iy: f32| {
            if ix < 0.0 || iy < 0.0 || ix >= rect.width as f32 || iy >= rect.height as f32 {
                return PAD;
            }
            self.texels[(rect.y as usize + iy as usize) * ATLAS_SIZE as usize + rect.x as usize + ix as usize]
        };
        let top = at(x0, y0) * (1.0 - tx) + at(x0 + 1.0, y0) * tx;
        let bottom = at(x0, y0 + 1.0) * (1.0 - tx) + at(x0 + 1.0, y0 + 1.0) * tx;
        top * (1.0 - ty) + bottom * ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_are_signed_and_exact_for_a_tile() {
        let atlas = ShapeAtlas::build();
        let i = catalog().piece_index("3070").unwrap();
        let r = atlas.rects[i];
        assert!((atlas.sample(r, 0.5, 0.5) + 0.5).abs() < 0.01, "centre depth");
        assert!(atlas.sample(r, 0.1, 0.5) < 0.0);
        assert!((atlas.sample(r, 0.1, 0.5) + 0.1).abs() < 0.01);
        assert!((atlas.sample(r, 1.1, 0.5) - 0.1).abs() < 0.01);
    }

    #[test]
    fn the_quarter_round_is_curved() {
        let atlas = ShapeAtlas::build();
        let i = catalog().piece_index("25269").unwrap();
        let r = atlas.rects[i];
        // Rotation 0 puts the curve's centre at one corner; the opposite corner is outside.
        let corners = [(0.05, 0.05), (0.95, 0.05), (0.05, 0.95), (0.95, 0.95)];
        let inside = corners.iter().filter(|(x, y)| atlas.sample(r, *x, *y) < 0.0).count();
        assert_eq!(inside, 3);
    }
}
