//! The read-only letterpress piece catalog (`lego-letterpress-kit`).
//!
//! Malformed entries are dropped with a warning instead of failing, so a newer
//! kit with an unknown entry still loads. Every rotated and mirrored pose of
//! every piece is prepared once: its collision mask and its flattened surface.

use crate::silhouette::{self, Surface};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

const KIT_JSON: &str = include_str!("../../../lego-letterpress-kit/lego-letterpress-kit/letterpress-pieces.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Rectilinear,
    Round,
    Curve,
    Wedge,
    Special,
}

impl Category {
    pub const ALL: [Category; 5] =
        [Category::Rectilinear, Category::Round, Category::Curve, Category::Wedge, Category::Special];
    pub fn label(self) -> &'static str {
        match self {
            Category::Rectilinear => "Basic",
            Category::Round => "Round",
            Category::Curve => "Curves",
            Category::Wedge => "Angular",
            Category::Special => "Special",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Category::Rectilinear => "rectilinear",
            Category::Round => "round",
            Category::Curve => "curve",
            Category::Wedge => "wedge",
            Category::Special => "special",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FillRule {
    Nonzero,
    Evenodd,
}

impl FillRule {
    pub fn svg(self) -> &'static str {
        match self {
            FillRule::Nonzero => "nonzero",
            FillRule::Evenodd => "evenodd",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Footprint {
    pub width_studs: i32,
    pub height_studs: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Geometry {
    pub view_box: Vec<f64>,
    pub path: String,
    pub fill_rule: FillRule,
    #[serde(default)]
    pub fidelity: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Piece {
    pub id: String,
    #[serde(default)]
    pub design_id: String,
    pub name: String,
    pub category: Category,
    pub footprint: Footprint,
    #[serde(default = "one")]
    pub surface_height_plates: f64,
    pub geometry: Geometry,
    pub grid_mask: Vec<Vec<u8>>,
    pub allowed_rotations: Vec<i32>,
    #[serde(default)]
    pub mirror_partner: Option<String>,
    #[serde(default)]
    pub physical_letterpress_ready: bool,
    #[serde(default)]
    pub letterpress_importance: f64,
    #[serde(default)]
    pub default_toolbar: bool,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub reference: String,
}

fn one() -> f64 {
    1.0
}

impl Piece {
    /// Pieces that are one plate high can be printed together on a real press.
    pub fn is_physical(&self) -> bool {
        self.physical_letterpress_ready && self.surface_height_plates == 1.0
    }
    pub fn allows(&self, rotation: i32) -> bool {
        self.allowed_rotations.contains(&rotation)
    }
    fn is_valid(&self) -> bool {
        let w = self.footprint.width_studs;
        let h = self.footprint.height_studs;
        w > 0
            && h > 0
            && !self.geometry.path.is_empty()
            && self.geometry.view_box.len() == 4
            && self.geometry.view_box.iter().all(|v| v.is_finite())
            && !self.allowed_rotations.is_empty()
            && self.allowed_rotations.iter().all(|r| matches!(r, 0 | 90 | 180 | 270))
            && self.grid_mask.len() == h as usize
            && self.grid_mask.iter().all(|row| row.len() == w as usize && row.iter().all(|&n| n <= 1))
    }
}

/// Keeps well-formed, unique entries and warns about the rest.
pub fn parse_pieces(input: &[Value]) -> Vec<Piece> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for entry in input {
        let parsed =
            serde_json::from_value::<Piece>(entry.clone()).ok().filter(|p| p.is_valid() && !seen.contains(&p.id));
        match parsed {
            Some(piece) => {
                seen.insert(piece.id.clone());
                out.push(piece);
            }
            None => {
                let id = entry.get("id").and_then(Value::as_str).unwrap_or("unknown");
                log::warn!("Omitting malformed letterpress catalog entry: {id}");
            }
        }
    }
    out
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinateSystem {
    pub stud_pitch_mm_nominal: f64,
    pub regular_piece_gap_mm_nominal: f64,
    pub default_surface_scale: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultBoard {
    pub width_studs: i32,
    pub height_studs: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintSimulation {
    pub ink_coverage: [f64; 2],
    pub edge_erosion_studs: [f64; 2],
    pub position_jitter_studs: [f64; 2],
    pub rotation_jitter_degrees: [f64; 2],
    pub registration_error_for_multi_color_studs: [f64; 2],
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rendering {
    pub recommended_default_board: DefaultBoard,
    pub recommended_print_simulation: PrintSimulation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawKit {
    name: String,
    coordinate_system: CoordinateSystem,
    rendering: Rendering,
    pieces: Vec<Value>,
}

/// One rotated/mirrored orientation of a catalog piece.
#[derive(Debug)]
pub struct Pose {
    pub piece: usize,
    pub rotation: i32,
    pub mirror_x: bool,
    pub mirror_y: bool,
    /// Footprint after rotation.
    pub width: i32,
    pub height: i32,
    /// Rotated, then mirrored, collision mask (row-major, `height` rows).
    pub mask: Vec<Vec<u8>>,
    /// Offsets of filled mask cells.
    pub cells: Vec<(i32, i32)>,
    /// Surface in pose-local stud coordinates, or `None` for an unparseable path
    /// (collision then falls back to the conservative mask).
    pub surface: Option<Surface>,
}

pub struct Catalog {
    pub name: String,
    pub coordinate_system: CoordinateSystem,
    pub rendering: Rendering,
    pub pieces: Vec<Piece>,
    index: HashMap<String, usize>,
    poses: Vec<Pose>,
    /// Flattened contours in canonical (unrotated, unscaled) piece coordinates.
    pub contours: Vec<Option<Vec<Vec<silhouette::Point>>>>,
}

pub type PoseId = u16;

impl Catalog {
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let raw: RawKit = serde_json::from_str(text)?;
        let pieces = parse_pieces(&raw.pieces);
        let index = pieces.iter().enumerate().map(|(i, p)| (p.id.clone(), i)).collect();
        let scale = raw.coordinate_system.default_surface_scale;
        let contours: Vec<_> = pieces.iter().map(|p| silhouette::parse_contours(&p.geometry.path).ok()).collect();
        let mut poses = Vec::with_capacity(pieces.len() * 16);
        for (i, piece) in pieces.iter().enumerate() {
            for r in 0..4 {
                for mx in [false, true] {
                    for my in [false, true] {
                        let rotation = r * 90;
                        let mut mask = rotate_mask(&piece.grid_mask, rotation);
                        if mx {
                            for row in &mut mask {
                                row.reverse();
                            }
                        }
                        if my {
                            mask.reverse();
                        }
                        let height = mask.len() as i32;
                        let width = mask.first().map_or(0, |r| r.len()) as i32;
                        let cells = mask
                            .iter()
                            .enumerate()
                            .flat_map(|(y, row)| {
                                row.iter().enumerate().filter(|(_, &v)| v != 0).map(move |(x, _)| (x as i32, y as i32))
                            })
                            .collect();
                        let surface = contours[i].as_ref().map(|c| {
                            silhouette::pose_surface(
                                c,
                                piece.geometry.fill_rule,
                                piece.footprint.width_studs as f64,
                                piece.footprint.height_studs as f64,
                                scale,
                                rotation,
                                mx,
                                my,
                            )
                        });
                        poses.push(Pose {
                            piece: i,
                            rotation,
                            mirror_x: mx,
                            mirror_y: my,
                            width,
                            height,
                            mask,
                            cells,
                            surface,
                        });
                    }
                }
            }
        }
        Ok(Self {
            name: raw.name,
            coordinate_system: raw.coordinate_system,
            rendering: raw.rendering,
            pieces,
            index,
            poses,
            contours,
        })
    }

    pub fn piece(&self, id: &str) -> Option<&Piece> {
        self.index.get(id).map(|&i| &self.pieces[i])
    }
    pub fn piece_index(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }
    pub fn surface_scale(&self) -> f64 {
        self.coordinate_system.default_surface_scale
    }
    /// Gap between neighbouring printed surfaces, in studs.
    pub fn design_gap_studs(&self) -> f64 {
        self.coordinate_system.regular_piece_gap_mm_nominal / self.coordinate_system.stud_pitch_mm_nominal
    }
    pub fn pose_id(&self, piece_id: &str, rotation: i32, mirror_x: bool, mirror_y: bool) -> Option<PoseId> {
        let i = self.piece_index(piece_id)?;
        let r = (normalize_rotation(rotation) / 90) as usize;
        Some((i * 16 + r * 4 + (mirror_x as usize) * 2 + mirror_y as usize) as PoseId)
    }
    pub fn pose(&self, id: PoseId) -> &Pose {
        &self.poses[id as usize]
    }
    pub fn by_category(&self, category: Category) -> impl Iterator<Item = &Piece> {
        self.pieces.iter().filter(move |p| p.category == category)
    }
}

pub fn normalize_rotation(r: i32) -> i32 {
    r.rem_euclid(360)
}

/// Rotates a row-major mask clockwise in quarter turns.
pub fn rotate_mask(mask: &[Vec<u8>], rotation: i32) -> Vec<Vec<u8>> {
    let mut result: Vec<Vec<u8>> = mask.to_vec();
    for _ in 0..normalize_rotation(rotation) / 90 {
        let h = result.len();
        let w = result.first().map_or(0, |r| r.len());
        result = (0..w).map(|y| (0..h).rev().map(|x| result[x][y]).collect()).collect();
    }
    result
}

pub static CATALOG: LazyLock<Catalog> =
    LazyLock::new(|| Catalog::from_json(KIT_JSON).expect("the bundled piece catalog is valid JSON"));

pub fn catalog() -> &'static Catalog {
    &CATALOG
}

/// Shorthand used throughout the crate.
pub fn get_piece(id: &str) -> Option<&'static Piece> {
    CATALOG.piece(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_supplied_catalog_and_metadata() {
        let c = catalog();
        assert_eq!(c.pieces.len(), 41);
        assert_eq!(c.surface_scale(), 0.975);
        assert!(get_piece("25269").unwrap().geometry.path.contains('A'));
        assert!(c.contours.iter().all(Option::is_some), "every path flattens");
    }

    #[test]
    fn lookup_default_toolbar_and_height_flags() {
        assert!(get_piece("missing").is_none());
        assert_eq!(catalog().pieces.iter().filter(|p| p.default_toolbar).count(), 10);
        assert_eq!(get_piece("68869").unwrap().surface_height_plates, 2.0);
    }

    #[test]
    fn omits_malformed_and_duplicate_entries() {
        let good: Value = serde_json::from_str(KIT_JSON).unwrap();
        let first = good["pieces"][0].clone();
        let parsed = parse_pieces(&[serde_json::json!({}), first.clone(), first]);
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn rotates_asymmetric_masks_clockwise() {
        let mask = vec![vec![1, 1], vec![1, 0]];
        assert_eq!(rotate_mask(&mask, 90), vec![vec![1, 1], vec![0, 1]]);
        assert_eq!(rotate_mask(&mask, 180), vec![vec![0, 1], vec![1, 1]]);
        assert_eq!(rotate_mask(&mask, 360), mask);
    }

    #[test]
    fn catalog_entries_satisfy_the_kit_schema_rules() {
        // Replaces scripts/validate-catalog.mjs for the fields the editor relies on.
        let raw: Value = serde_json::from_str(KIT_JSON).unwrap();
        let entries = raw["pieces"].as_array().unwrap();
        assert_eq!(entries.len(), catalog().pieces.len(), "no entry was dropped");
        for p in &catalog().pieces {
            assert_eq!(p.geometry.view_box[2], p.footprint.width_studs as f64, "{}", p.id);
            assert_eq!(p.geometry.view_box[3], p.footprint.height_studs as f64, "{}", p.id);
            assert!(p.grid_mask.iter().flatten().any(|&v| v == 1), "{}", p.id);
        }
    }
}
