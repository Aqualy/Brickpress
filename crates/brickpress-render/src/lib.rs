//! GPU rendering for Brickpress, independent of the UI toolkit.
//!
//! The same pipelines draw the editor canvas, library thumbnails, PNG/SVG
//! exports and MCP previews, so what you see is what you export.

pub mod atlas;
pub mod board2d;
pub mod export;
pub mod gpu;
pub mod print;
pub mod scene;
pub mod svg;
pub mod three;

pub use wgpu;
