//! The version-1 project format (`.brickpress.json`).
//!
//! Field names and shapes match Brickpress 1.x exactly, so files written by
//! either version open in the other. Passes share their piece lists through
//! `Arc`, which keeps undo snapshots cheap: an edit copies only the passes it
//! touches.

use serde::{Deserialize, Serialize, Serializer};
use std::sync::Arc;

pub type Id = Arc<str>;

/// Serializes integral floats without a fraction (`1` rather than `1.0`), as
/// JavaScript's `JSON.stringify` does.
pub(crate) fn js_number<S: Serializer>(value: &f64, s: S) -> Result<S::Ok, S::Error> {
    if value.fract() == 0.0 && value.abs() < 9.0e15 {
        s.serialize_i64(*value as i64)
    } else {
        s.serialize_f64(*value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacedPiece {
    pub uid: Id,
    pub piece_id: Id,
    pub x: i32,
    pub y: i32,
    pub rotation: i32,
    pub seed: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_x: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_y: Option<bool>,
}

impl PlacedPiece {
    pub fn new(uid: impl Into<Id>, piece_id: impl Into<Id>, x: i32, y: i32, rotation: i32, seed: i64) -> Self {
        Self { uid: uid.into(), piece_id: piece_id.into(), x, y, rotation, seed, mirror_x: None, mirror_y: None }
    }
    pub fn mx(&self) -> bool {
        self.mirror_x.unwrap_or(false)
    }
    pub fn my(&self) -> bool {
        self.mirror_y.unwrap_or(false)
    }
    /// Stores a mirror flag the way 1.x does: absent when false is implied.
    pub fn with_mirror(mut self, mirror_x: bool, mirror_y: bool) -> Self {
        self.mirror_x = mirror_x.then_some(true);
        self.mirror_y = mirror_y.then_some(true);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InkPass {
    pub id: Id,
    pub name: String,
    pub color: String,
    pub visible: bool,
    pub locked: bool,
    pub registration_seed: i64,
    pub pieces: Arc<Vec<PlacedPiece>>,
}

impl InkPass {
    pub fn pieces_mut(&mut self) -> &mut Vec<PlacedPiece> {
        Arc::make_mut(&mut self.pieces)
    }
    pub fn editable(&self) -> bool {
        self.visible && !self.locked
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintSettings {
    #[serde(serialize_with = "js_number")]
    pub ink_amount: f64,
    #[serde(serialize_with = "js_number")]
    pub pressure: f64,
    #[serde(serialize_with = "js_number")]
    pub ink_coverage: f64,
    #[serde(serialize_with = "js_number")]
    pub roller_consistency: f64,
    #[serde(serialize_with = "js_number")]
    pub piece_wear: f64,
    #[serde(serialize_with = "js_number")]
    pub edge_wear: f64,
    #[serde(serialize_with = "js_number")]
    pub paper_tooth: f64,
    #[serde(serialize_with = "js_number")]
    pub texture_scale: f64,
    #[serde(serialize_with = "js_number")]
    pub misalignment: f64,
    #[serde(serialize_with = "js_number")]
    pub registration_error: f64,
    #[serde(serialize_with = "js_number")]
    pub print_roughness: f64,
}

/// Identifies one print control, for UI, MCP and validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrintKey {
    InkAmount,
    Pressure,
    InkCoverage,
    RollerConsistency,
    PieceWear,
    EdgeWear,
    PaperTooth,
    TextureScale,
    Misalignment,
    RegistrationError,
    PrintRoughness,
}

impl PrintSettings {
    pub fn get(&self, key: PrintKey) -> f64 {
        match key {
            PrintKey::InkAmount => self.ink_amount,
            PrintKey::Pressure => self.pressure,
            PrintKey::InkCoverage => self.ink_coverage,
            PrintKey::RollerConsistency => self.roller_consistency,
            PrintKey::PieceWear => self.piece_wear,
            PrintKey::EdgeWear => self.edge_wear,
            PrintKey::PaperTooth => self.paper_tooth,
            PrintKey::TextureScale => self.texture_scale,
            PrintKey::Misalignment => self.misalignment,
            PrintKey::RegistrationError => self.registration_error,
            PrintKey::PrintRoughness => self.print_roughness,
        }
    }
    pub fn get_mut(&mut self, key: PrintKey) -> &mut f64 {
        match key {
            PrintKey::InkAmount => &mut self.ink_amount,
            PrintKey::Pressure => &mut self.pressure,
            PrintKey::InkCoverage => &mut self.ink_coverage,
            PrintKey::RollerConsistency => &mut self.roller_consistency,
            PrintKey::PieceWear => &mut self.piece_wear,
            PrintKey::EdgeWear => &mut self.edge_wear,
            PrintKey::PaperTooth => &mut self.paper_tooth,
            PrintKey::TextureScale => &mut self.texture_scale,
            PrintKey::Misalignment => &mut self.misalignment,
            PrintKey::RegistrationError => &mut self.registration_error,
            PrintKey::PrintRoughness => &mut self.print_roughness,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperSettings {
    pub preset: String,
    pub color: String,
    #[serde(serialize_with = "js_number")]
    pub grain: f64,
    #[serde(serialize_with = "js_number")]
    pub tooth: f64,
    #[serde(serialize_with = "js_number")]
    pub fibers: f64,
    #[serde(serialize_with = "js_number")]
    pub brightness: f64,
    pub seed: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GridStyle {
    Off,
    Points,
    Lines,
    Squares,
}

impl GridStyle {
    pub const ALL: [GridStyle; 4] = [GridStyle::Off, GridStyle::Points, GridStyle::Lines, GridStyle::Squares];
    pub fn label(self) -> &'static str {
        match self {
            GridStyle::Off => "Off",
            GridStyle::Points => "Points",
            GridStyle::Lines => "Lines",
            GridStyle::Squares => "Squares",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Symmetry {
    Off,
    X,
    Y,
    Both,
}

impl Symmetry {
    pub const ALL: [Symmetry; 4] = [Symmetry::Off, Symmetry::X, Symmetry::Y, Symmetry::Both];
    pub fn mirrors_x(self) -> bool {
        matches!(self, Symmetry::X | Symmetry::Both)
    }
    pub fn mirrors_y(self) -> bool {
        matches!(self, Symmetry::Y | Symmetry::Both)
    }
    pub fn label(self) -> &'static str {
        match self {
            Symmetry::Off => "Off",
            Symmetry::X => "Horizontal",
            Symmetry::Y => "Vertical",
            Symmetry::Both => "Both",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocOptions {
    pub grid: GridStyle,
    pub major_grid: i32,
    pub allow_overlap: bool,
    pub physical: bool,
    pub symmetry: Symmetry,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PressDocument {
    pub version: u32,
    pub name: String,
    pub board: Board,
    pub paper: PaperSettings,
    pub print_settings: PrintSettings,
    pub passes: Vec<InkPass>,
    pub swatches: Vec<String>,
    pub options: DocOptions,
}

impl PressDocument {
    pub fn pass(&self, id: &str) -> Option<&InkPass> {
        self.passes.iter().find(|p| &*p.id == id)
    }
    pub fn pass_mut(&mut self, id: &str) -> Option<&mut InkPass> {
        self.passes.iter_mut().find(|p| &*p.id == id)
    }
    pub fn pass_index(&self, id: &str) -> Option<usize> {
        self.passes.iter().position(|p| &*p.id == id)
    }
    pub fn piece_count(&self) -> usize {
        self.passes.iter().map(|p| p.pieces.len()).sum()
    }
    /// Every placed piece with its pass, in paint order.
    pub fn all_pieces(&self) -> impl Iterator<Item = (&InkPass, &PlacedPiece)> {
        self.passes.iter().flat_map(|pass| pass.pieces.iter().map(move |p| (pass, p)))
    }
    pub fn find_piece(&self, uid: &str) -> Option<(&InkPass, &PlacedPiece)> {
        self.all_pieces().find(|(_, p)| &*p.uid == uid)
    }
}

pub fn valid_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// Parses `#rrggbb` into bytes.
pub fn parse_color(color: &str) -> Option<[u8; 3]> {
    if !valid_color(color) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&color[i..i + 2], 16).ok();
    Some([byte(1)?, byte(3)?, byte(5)?])
}

pub fn new_id() -> Id {
    uuid::Uuid::new_v4().to_string().into()
}

pub fn new_seed() -> i64 {
    fastrand::i64(0..2_147_483_647)
}
