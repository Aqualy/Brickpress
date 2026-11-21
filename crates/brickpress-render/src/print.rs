//! Seeded per-piece and per-pass print variation, computed on the CPU.
//!
//! The jitter, rotation, opacity and registration formulas are the 1.x ones,
//! so the print sliders keep their meaning. The surface texture itself is
//! computed in the shader (see `shaders/pieces.wgsl`).

use brickpress_core::catalog::catalog;
use brickpress_core::geometry::bounds;
use brickpress_core::rng::SeededRandom;
use brickpress_core::{InkPass, PaperSettings, PlacedPiece, PrintSettings};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Impression {
    pub dx: f64,
    pub dy: f64,
    /// Degrees, about the footprint centre.
    pub rotation: f64,
    pub opacity: f64,
}

pub fn impression(piece: &PlacedPiece, s: &PrintSettings) -> Impression {
    let mut random = SeededRandom::new(piece.seed);
    let range = &catalog().rendering.recommended_print_simulation;
    let jitter = s.misalignment * range.position_jitter_studs[1];
    let dx = (random.next_f64() - 0.5) * jitter * 2.0;
    let dy = (random.next_f64() - 0.5) * jitter * 2.0;
    let rotation = (random.next_f64() - 0.5) * range.rotation_jitter_degrees[1] * 2.0 * s.misalignment;
    let opacity = (0.68 + s.ink_amount * 0.32 + s.pressure * 0.08
        - random.next_f64() * (1.0 - s.roller_consistency) * 0.18
        - random.next_f64() * s.piece_wear * 0.1)
        .min(1.0);
    Impression { dx, dy, rotation, opacity }
}

/// Whole-pass registration offset in studs.
pub fn pass_offset(pass: &InkPass, s: &PrintSettings) -> (f64, f64) {
    let mut random = SeededRandom::new(pass.registration_seed);
    let amount = s.registration_error
        * catalog().rendering.recommended_print_simulation.registration_error_for_multi_color_studs[1]
        * 3.0;
    let x = (random.next_f64() - 0.5) * amount * 2.0;
    let y = (random.next_f64() - 0.5) * amount * 2.0;
    (x, y)
}

/// Paper colour after brightness, as `#rrggbb` bytes.
pub fn paper_color(paper: &PaperSettings) -> [u8; 3] {
    let rgb = brickpress_core::parse_color(&paper.color).unwrap_or([255, 255, 255]);
    rgb.map(|c| ((c as f64 * paper.brightness).round()).min(255.0) as u8)
}

/// Edge erosion radius in studs.
pub fn erosion(s: &PrintSettings) -> f64 {
    s.edge_wear * 0.016 * (1.0 - s.pressure * 0.35)
}

/// Rotates the matrix `m` by `degrees` about (cx, cy) and then translates by (dx, dy).
pub fn jitter_matrix(m: [f64; 6], imp: &Impression, piece: &PlacedPiece, offset: (f64, f64)) -> [f64; 6] {
    let b = bounds(piece);
    let (cx, cy) = (b.x as f64 + b.width as f64 / 2.0, b.y as f64 + b.height as f64 / 2.0);
    let (sin, cos) = imp.rotation.to_radians().sin_cos();
    // R about c: p' = R (p - c) + c
    let rot = [cos, sin, -sin, cos, cx - cos * cx + sin * cy, cy - sin * cx - cos * cy];
    let combined = multiply(rot, m);
    [
        combined[0],
        combined[1],
        combined[2],
        combined[3],
        combined[4] + imp.dx + offset.0,
        combined[5] + imp.dy + offset.1,
    ]
}

/// SVG-style matrix product a · b.
pub fn multiply(a: [f64; 6], b: [f64; 6]) -> [f64; 6] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickpress_core::persistence::document::create_document;

    #[test]
    fn deterministic_and_within_1x_ranges() {
        let doc = create_document(true);
        let p = &doc.passes[0].pieces[0];
        let a = impression(p, &doc.print_settings);
        assert_eq!(a, impression(p, &doc.print_settings));
        assert!(a.opacity <= 1.0 && a.opacity > 0.5);
        assert!(a.dx.abs() <= 0.018 * doc.print_settings.misalignment);
        assert_eq!(pass_offset(&doc.passes[0], &doc.print_settings), pass_offset(&doc.passes[0], &doc.print_settings));
    }

    #[test]
    fn no_jitter_keeps_the_matrix() {
        let m = [1.0, 0.0, 0.0, 1.0, 3.0, 4.0];
        let p = PlacedPiece::new("a", "3070", 3, 4, 0, 1);
        let still = Impression { dx: 0.0, dy: 0.0, rotation: 0.0, opacity: 1.0 };
        let out = jitter_matrix(m, &still, &p, (0.0, 0.0));
        for (x, y) in out.iter().zip(m) {
            assert!((x - y).abs() < 1e-12);
        }
    }
}
