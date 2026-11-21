//! Grid geometry. Placement origins are the top-left of the *rotated* footprint.

use crate::catalog::{catalog, get_piece, normalize_rotation, Piece, PoseId};
use crate::document::{Board, PlacedPiece, PressDocument, Symmetry};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn right(&self) -> i32 {
        self.x + self.width
    }
    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right() && other.x < self.right() && self.y < other.bottom() && other.y < self.bottom()
    }
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Rect { x, y, width: self.right().max(other.right()) - x, height: self.bottom().max(other.bottom()) - y }
    }
}

pub fn footprint(piece: &Piece, rotation: i32) -> Size {
    let swap = normalize_rotation(rotation) % 180 != 0;
    let (w, h) = (piece.footprint.width_studs, piece.footprint.height_studs);
    if swap {
        Size { width: h, height: w }
    } else {
        Size { width: w, height: h }
    }
}

pub fn pose_of(p: &PlacedPiece) -> Option<PoseId> {
    catalog().pose_id(&p.piece_id, p.rotation, p.mx(), p.my())
}

/// Rotated and mirrored collision mask of a placed piece.
pub fn grid_mask(p: &PlacedPiece) -> Vec<Vec<u8>> {
    pose_of(p).map(|id| catalog().pose(id).mask.clone()).unwrap_or_default()
}

pub fn occupied_cells(p: &PlacedPiece) -> Vec<(i32, i32)> {
    pose_of(p).map(|id| catalog().pose(id).cells.iter().map(|&(x, y)| (p.x + x, p.y + y)).collect()).unwrap_or_default()
}

/// Bounding box of a placed piece; a zero box for unknown pieces.
pub fn bounds(p: &PlacedPiece) -> Rect {
    let size = get_piece(&p.piece_id).map_or(Size { width: 0, height: 0 }, |piece| footprint(piece, p.rotation));
    Rect { x: p.x, y: p.y, width: size.width, height: size.height }
}

pub fn bounds_of<'a>(pieces: impl IntoIterator<Item = &'a PlacedPiece>) -> Option<Rect> {
    pieces.into_iter().map(bounds).reduce(|a, b| a.union(&b))
}

pub fn inside_board(rect: &Rect, board: Board) -> bool {
    rect.x >= 0 && rect.y >= 0 && rect.right() <= board.width && rect.bottom() <= board.height
}

/// The next allowed rotation, centred on the old footprint. Half-stud ties
/// resolve toward the original origin; symmetric rounding prevents drift
/// after four quarter turns.
pub fn next_rotation(p: &PlacedPiece) -> PlacedPiece {
    let Some(piece) = get_piece(&p.piece_id) else { return p.clone() };
    let rotations = &piece.allowed_rotations;
    let index = rotations.iter().position(|&r| r == p.rotation).map_or(0, |i| i + 1);
    let rotation = rotations[index % rotations.len()];
    let old = footprint(piece, p.rotation);
    let next = footprint(piece, rotation);
    PlacedPiece {
        rotation,
        x: p.x + (old.width - next.width) / 2,
        y: p.y + (old.height - next.height) / 2,
        ..p.clone()
    }
}

/// The piece plus its mirror images under the document's symmetry, deduplicated by origin.
pub fn symmetry_placements(p: &PlacedPiece, doc: &PressDocument) -> Vec<PlacedPiece> {
    symmetry_placements_on(p, doc.board, doc.options.symmetry)
}

pub fn symmetry_placements_on(p: &PlacedPiece, board: Board, symmetry: Symmetry) -> Vec<PlacedPiece> {
    let b = bounds(p);
    let mut list = vec![p.clone()];
    if symmetry.mirrors_x() {
        let mx = !p.mx();
        list.push(PlacedPiece { x: board.width - p.x - b.width, ..p.clone() }.with_mirror(mx, p.my()));
    }
    if symmetry.mirrors_y() {
        let flipped: Vec<_> = list
            .iter()
            .map(|item| {
                let my = !item.my();
                let mx = item.mx();
                PlacedPiece { y: board.height - item.y - b.height, ..item.clone() }.with_mirror(mx, my)
            })
            .collect();
        list.extend(flipped);
    }
    let mut seen = std::collections::HashSet::new();
    list.retain(|item| seen.insert((item.x, item.y)));
    list
}

/// Affine transform (a, b, c, d, e, f) mapping canonical piece coordinates to
/// board studs, as SVG `matrix(a b c d e f)`: translate to the footprint
/// centre, mirror, rotate, scale, then translate the piece centre to the origin.
pub fn piece_matrix(piece: &Piece, p: &PlacedPiece, scale: f64) -> [f64; 6] {
    let size = footprint(piece, p.rotation);
    let (cx, cy) = (p.x as f64 + size.width as f64 / 2.0, p.y as f64 + size.height as f64 / 2.0);
    let (sx, sy) = (if p.mx() { -1.0 } else { 1.0 }, if p.my() { -1.0 } else { 1.0 });
    let (sin, cos) = match normalize_rotation(p.rotation) {
        90 => (1.0, 0.0),
        180 => (0.0, -1.0),
        270 => (-1.0, 0.0),
        _ => (0.0, 1.0),
    };
    // M = T(c) · S(sx, sy) · R · S(scale) · T(-w/2, -h/2)
    let a = sx * cos * scale;
    let b = sy * sin * scale;
    let c = -sx * sin * scale;
    let d = sy * cos * scale;
    let (hw, hh) = (piece.footprint.width_studs as f64 / 2.0, piece.footprint.height_studs as f64 / 2.0);
    let e = cx - a * hw - c * hh;
    let f = cy - b * hw - d * hh;
    [a, b, c, d, e, f]
}

pub fn apply_matrix(m: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::document::create_document;

    fn placed(id: &str, x: i32, y: i32, rotation: i32) -> PlacedPiece {
        PlacedPiece::new("test", id, x, y, rotation, 171)
    }

    #[test]
    fn rotates_non_square_footprints() {
        let p = get_piece("3069").unwrap();
        assert_eq!(footprint(p, 90), Size { width: 1, height: 2 });
        assert_eq!(footprint(p, 360), Size { width: 2, height: 1 });
    }

    #[test]
    fn preserves_the_logical_center_when_parity_matches() {
        let p = placed("35787", 4, 4, 0);
        let r = next_rotation(&p);
        assert_eq!(bounds(&r), bounds(&p));
        assert_eq!(r.rotation, 90);
    }

    #[test]
    fn snaps_half_stud_parity_after_rotation() {
        let p = next_rotation(&placed("3069", 4, 4, 0));
        assert_eq!((p.x, p.y), (4, 4));
        let m = piece_matrix(get_piece("3069").unwrap(), &p, 0.975);
        // Centre of the canonical piece lands on the rotated footprint's centre.
        let (cx, cy) = apply_matrix(&m, 1.0, 0.5);
        assert!((cx - 4.5).abs() < 1e-12 && (cy - 5.0).abs() < 1e-12);
    }

    #[test]
    fn returns_to_origin_after_four_rotations() {
        for id in ["3069", "2431", "6636", "8165"] {
            let original = placed(id, 8, 8, 0);
            let mut p = original.clone();
            for _ in 0..4 {
                p = next_rotation(&p);
            }
            assert_eq!(p, original, "{id}");
        }
    }

    #[test]
    fn supports_mirrored_masks() {
        let p = placed("14719", 0, 0, 0).with_mirror(true, false);
        assert_eq!(grid_mask(&p), vec![vec![1, 1], vec![0, 1]]);
        let q = placed("3069", 0, 0, 90);
        assert_eq!(occupied_cells(&q), vec![(0, 0), (0, 1)]);
    }

    #[test]
    fn deduplicated_four_way_symmetry() {
        let mut doc = create_document(false);
        doc.options.symmetry = Symmetry::Both;
        let ghosts = symmetry_placements(&placed("25269", 3, 4, 0), &doc);
        assert_eq!(ghosts.len(), 4);
        assert_eq!((ghosts[3].x, ghosts[3].y, ghosts[3].mx(), ghosts[3].my()), (12, 11, true, true));
    }

    #[test]
    fn matrix_matches_the_svg_transform_chain() {
        // translate(c) scale(mx my) rotate(r) scale(s) translate(-w/2 -h/2), checked on a corner.
        let piece = get_piece("3069").unwrap();
        let p = placed("3069", 2, 3, 90).with_mirror(true, false);
        let m = piece_matrix(piece, &p, 1.0);
        // Canonical (0,0) → centred (-1,-0.5) → rotate 90° cw (0.5,-1) → mirror x (-0.5,-1) → + centre (2.5, 4).
        let (x, y) = apply_matrix(&m, 0.0, 0.0);
        assert!((x - 2.0).abs() < 1e-12 && (y - 3.0).abs() < 1e-12, "{x},{y}");
    }
}
