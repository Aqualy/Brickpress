//! Occupancy index and placement validation.
//!
//! Collision has two stages: the cached rotated/mirrored stud masks pick
//! neighbours that share a cell, then the exact surfaces decide. The index is
//! a dense per-cell list, built once per document version and reused for every
//! hover check, instead of being rebuilt on each call.

use crate::catalog::{catalog, PoseId};
use crate::document::{Board, PlacedPiece, PressDocument};
use crate::geometry::{bounds, inside_board, pose_of};
use crate::silhouette::poses_overlap;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceError {
    UnknownPiece,
    NotOnGrid,
    OutOfBounds,
    Overlap,
}

impl fmt::Display for PlaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PlaceError::UnknownPiece => "unknown catalog piece",
            PlaceError::NotOnGrid => "position is not on the stud grid",
            PlaceError::OutOfBounds => "leaves the board",
            PlaceError::Overlap => "overlaps another piece",
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct Occupant {
    id: u32,
    pose: PoseId,
    x: i32,
    y: i32,
}

#[derive(Clone)]
pub struct OccupancyIndex {
    board: Board,
    allow_overlap: bool,
    cells: Vec<SmallVec<[Occupant; 2]>>,
    next_id: u32,
}

impl OccupancyIndex {
    /// Indexes every piece of every pass (hidden and locked passes still
    /// occupy the plate), except those in `ignored`.
    pub fn build(doc: &PressDocument, ignored: &HashSet<&str>) -> Self {
        let mut index = Self::empty(doc);
        if !index.allow_overlap {
            for (_, p) in doc.all_pieces() {
                if !ignored.contains(&*p.uid) {
                    index.insert(p);
                }
            }
        }
        index
    }

    pub fn empty(doc: &PressDocument) -> Self {
        let board = doc.board;
        let len = (board.width.max(0) * board.height.max(0)) as usize;
        Self { board, allow_overlap: doc.options.allow_overlap, cells: vec![SmallVec::new(); len], next_id: 0 }
    }

    fn cell(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.board.width && y < self.board.height).then(|| (y * self.board.width + x) as usize)
    }

    fn insert(&mut self, p: &PlacedPiece) {
        let Some(pose) = pose_of(p) else { return };
        let occupant = Occupant { id: self.next_id, pose, x: p.x, y: p.y };
        self.next_id += 1;
        for &(cx, cy) in &catalog().pose(pose).cells {
            if let Some(i) = self.cell(p.x + cx, p.y + cy) {
                self.cells[i].push(occupant);
            }
        }
    }

    /// Checks a group of candidates against the index and against each other.
    /// Nothing is reserved; call [`add`](Self::add) after a successful check.
    pub fn check(&self, candidates: &[PlacedPiece]) -> Result<(), (usize, PlaceError)> {
        let mut pending: HashMap<usize, SmallVec<[Occupant; 2]>> = HashMap::new();
        for (n, p) in candidates.iter().enumerate() {
            let Some(pose) = pose_of(p) else { return Err((n, PlaceError::UnknownPiece)) };
            if !inside_board(&bounds(p), self.board) {
                return Err((n, PlaceError::OutOfBounds));
            }
            if self.allow_overlap {
                continue;
            }
            let me = Occupant { id: u32::MAX - n as u32, pose, x: p.x, y: p.y };
            let mut checked: SmallVec<[u32; 8]> = SmallVec::new();
            for &(cx, cy) in &catalog().pose(pose).cells {
                let Some(i) = self.cell(p.x + cx, p.y + cy) else { continue };
                let others = self.cells[i].iter().chain(pending.get(&i).into_iter().flatten());
                for other in others {
                    if checked.contains(&other.id) {
                        continue;
                    }
                    checked.push(other.id);
                    if poses_overlap(pose, other.pose, (other.x - p.x, other.y - p.y)) {
                        return Err((n, PlaceError::Overlap));
                    }
                }
            }
            for &(cx, cy) in &catalog().pose(pose).cells {
                if let Some(i) = self.cell(p.x + cx, p.y + cy) {
                    pending.entry(i).or_default().push(me);
                }
            }
        }
        Ok(())
    }

    pub fn can_place(&self, candidates: &[PlacedPiece]) -> bool {
        self.check(candidates).is_ok()
    }

    /// Reserves accepted candidates, for placement strokes.
    pub fn add(&mut self, candidates: &[PlacedPiece]) {
        if !self.allow_overlap {
            for p in candidates {
                self.insert(p);
            }
        }
    }
}

/// One-shot check; see [`OccupancyIndex`] for repeated checks.
pub fn can_place(doc: &PressDocument, candidates: &[PlacedPiece], ignored: &HashSet<&str>) -> bool {
    OccupancyIndex::build(doc, ignored).can_place(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::document::create_document;
    use std::sync::Arc;

    fn placed(id: &str, x: i32, y: i32, rotation: i32) -> PlacedPiece {
        PlacedPiece::new("test", id, x, y, rotation, 171)
    }
    fn with_uid(mut p: PlacedPiece, uid: &str) -> PlacedPiece {
        p.uid = uid.into();
        p
    }
    fn set(doc: &mut PressDocument, pieces: Vec<PlacedPiece>) {
        doc.passes[0].pieces = Arc::new(pieces);
    }
    fn none() -> HashSet<&'static str> {
        HashSet::new()
    }

    #[test]
    fn uses_grid_mask_holes_instead_of_bounding_boxes() {
        let mut doc = create_document(false);
        set(&mut doc, vec![placed("14719", 0, 0, 0)]);
        assert!(can_place(&doc, &[placed("3070", 1, 1, 0)], &none()));
        assert!(!can_place(&doc, &[placed("3070", 0, 1, 0)], &none()));
    }

    #[test]
    fn fits_the_quarter_round_into_a_quarter_ring_at_every_pose() {
        for rotation in [0, 90, 180, 270] {
            for mx in [false, true] {
                for my in [false, true] {
                    let mut doc = create_document(false);
                    let corner = [(0, 0), (1, 0), (1, 1), (0, 1)][(rotation / 90) as usize];
                    let ring = with_uid(placed("27925", 4, 4, rotation), "ring").with_mirror(mx, my);
                    let qx = 4 + if mx { 1 - corner.0 } else { corner.0 };
                    let qy = 4 + if my { 1 - corner.1 } else { corner.1 };
                    let quarter = with_uid(placed("25269", qx, qy, rotation), "quarter").with_mirror(mx, my);
                    set(&mut doc, vec![ring.clone()]);
                    assert!(can_place(&doc, std::slice::from_ref(&quarter), &none()), "{rotation} {mx} {my}");
                    let flipped = PlacedPiece { rotation: (rotation + 180) % 360, ..quarter.clone() };
                    assert!(!can_place(&doc, &[flipped], &none()));
                    assert!(!can_place(&doc, &[placed("3070", quarter.x, quarter.y, 0)], &none()));
                    set(&mut doc, vec![quarter]);
                    assert!(can_place(&doc, &[ring], &none()));
                }
            }
        }
    }

    #[test]
    fn checks_curved_overlaps_within_groups_and_hidden_passes() {
        let mut doc = create_document(false);
        let ring = with_uid(placed("27925", 4, 4, 0), "ring");
        let quarter = with_uid(placed("25269", 4, 4, 0), "quarter");
        assert!(can_place(&doc, &[ring.clone(), quarter.clone()], &none()));
        let dup = with_uid(quarter.clone(), "duplicate");
        assert!(!can_place(&doc, &[ring.clone(), quarter.clone(), dup], &none()));
        set(&mut doc, vec![ring, quarter.clone()]);
        doc.passes[0].visible = false;
        doc.passes[0].locked = true;
        assert!(!can_place(&doc, &[placed("3070", 4, 4, 0)], &none()));
        assert!(can_place(&doc, &[quarter], &HashSet::from(["quarter"])));
    }

    #[test]
    fn complementary_triangles_but_not_containment_or_identical_curves() {
        let mut doc = create_document(false);
        set(&mut doc, vec![placed("35787", 4, 4, 0)]);
        assert!(can_place(&doc, &[placed("35787", 4, 4, 180)], &none()));
        assert!(!can_place(&doc, &[placed("3068", 4, 4, 0)], &none()));
        set(&mut doc, vec![placed("98138", 4, 4, 0)]);
        assert!(!can_place(&doc, &[placed("98138", 4, 4, 0)], &none()));
        assert!(can_place(&doc, &[placed("98138", 5, 4, 0)], &none()));
    }

    #[test]
    fn rotated_masks_candidates_bounds_and_explicit_overlap() {
        let mut doc = create_document(false);
        set(&mut doc, vec![placed("3069", 0, 0, 90)]);
        assert!(!can_place(&doc, &[placed("3070", 0, 1, 0)], &none()));
        assert!(can_place(&doc, &[placed("3070", 1, 1, 0)], &none()));
        assert!(!can_place(&doc, &[placed("3069", 15, 0, 0)], &none()));
        assert!(!can_place(&doc, &[placed("3070", 0, 0, 0), placed("3070", 0, 0, 0)], &none()));
        doc.options.allow_overlap = true;
        assert!(can_place(&doc, &[placed("3070", 0, 0, 0)], &none()));
    }

    #[test]
    fn ignores_moving_originals() {
        let mut doc = create_document(false);
        set(&mut doc, vec![placed("14719", 0, 0, 0)]);
        assert!(can_place(&doc, &[placed("14719", 0, 0, 90)], &HashSet::from(["test"])));
    }

    #[test]
    fn rejected_groups_do_not_reserve_partial_placements() {
        let mut doc = create_document(false);
        let tile = |x| with_uid(PlacedPiece::new("t", "3070", x, 1, 0, 1), &format!("tile-{x}"));
        set(&mut doc, vec![tile(1)]);
        let mut index = OccupancyIndex::build(&doc, &HashSet::new());
        assert!(!index.can_place(&[tile(2), tile(1)]));
        assert!(index.can_place(&[tile(2)]));
        index.add(&[tile(2)]);
        assert!(!index.can_place(&[tile(2)]));
    }

    #[test]
    fn reports_why_a_placement_failed() {
        let doc = create_document(false);
        let index = OccupancyIndex::build(&doc, &HashSet::new());
        assert_eq!(index.check(&[placed("nope", 0, 0, 0)]), Err((0, PlaceError::UnknownPiece)));
        assert_eq!(index.check(&[placed("3070", -1, 0, 0)]), Err((0, PlaceError::OutOfBounds)));
    }
}
