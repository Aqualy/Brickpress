//! Brickpress core: the project format, the piece catalog, placement geometry
//! and every editing operation. No GPU or UI code lives here, so the GUI, the
//! MCP server and tests share exactly the same rules.

pub mod catalog;
pub mod document;
pub mod editor;
pub mod geometry;
pub mod history;
pub mod ops;
pub mod persistence;
pub mod placement_path;
pub mod rng;
pub mod settings;
pub mod silhouette;
pub mod spatial;

pub use catalog::{catalog, get_piece, Catalog, Category, Piece};
pub use document::*;
pub use editor::{EditorCore, Mode, Tool};
