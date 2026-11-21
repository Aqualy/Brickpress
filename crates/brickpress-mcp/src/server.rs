//! The MCP tool surface.

use crate::live::LiveClient;
use base64::Engine;
use brickpress_core::catalog::{catalog, rotate_mask, Category};
use brickpress_core::ops::{self, LegendEntry, Op, OpResult, PieceSpec};
use brickpress_core::persistence::document::{create_document, serialize_document};
use brickpress_core::persistence::preferences::GridAppearance;
use brickpress_core::{EditorCore, GridStyle, PressDocument, Symmetry};
use brickpress_render::atlas::ShapeAtlas;
use brickpress_render::board2d::Renderer2D;
use brickpress_render::export::{encode_png, render_rgba, ExportMode, ExportOptions, EXPORT_FORMAT};
use brickpress_render::gpu::Gpu;
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub enum Backend {
    Headless { core: Box<EditorCore>, file: Option<PathBuf> },
    Live(LiveClient),
}

struct Renderer {
    gpu: Gpu,
    r2d: Renderer2D,
}

pub struct State {
    backend: Backend,
    root: PathBuf,
    renderer: Option<Renderer>,
    atlas: Option<Arc<ShapeAtlas>>,
}

impl State {
    fn call(&mut self, op: Op) -> OpResult {
        match &mut self.backend {
            Backend::Headless { core, .. } => ops::apply(core, op),
            Backend::Live(client) => client.call(op),
        }
    }

    fn document(&mut self) -> Result<PressDocument, String> {
        match &self.backend {
            Backend::Headless { core, .. } => Ok(core.doc().clone()),
            Backend::Live(_) => {
                let value = self.call(Op::GetDocument)?;
                brickpress_core::persistence::document::parse_document_value(&value).map_err(|e| e.0)
            }
        }
    }

    fn renderer(&mut self) -> Result<&Renderer, String> {
        if self.renderer.is_none() {
            let gpu = Gpu::headless().map_err(|e| e.to_string())?;
            let atlas = self.atlas.get_or_insert_with(|| Arc::new(ShapeAtlas::build())).clone();
            let r2d = Renderer2D::new(&gpu.device, &gpu.queue, EXPORT_FORMAT, atlas);
            self.renderer = Some(Renderer { gpu, r2d });
        }
        Ok(self.renderer.as_ref().expect("created"))
    }

    /// Resolves a path under the workspace root; refuses anything outside it.
    fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let candidate = Path::new(path);
        let joined = if candidate.is_absolute() { candidate.to_path_buf() } else { self.root.join(candidate) };
        let parent = joined.parent().ok_or("Give a file path.")?;
        let parent = parent.canonicalize().map_err(|_| format!("The folder {} does not exist.", parent.display()))?;
        let root = self.root.canonicalize().map_err(|e| e.to_string())?;
        if !parent.starts_with(&root) {
            return Err(format!("Paths must stay inside the workspace root {}.", root.display()));
        }
        Ok(parent.join(joined.file_name().ok_or("Give a file name.")?))
    }
}

#[derive(Clone)]
pub struct BrickpressMcp {
    state: Arc<Mutex<State>>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

fn text(value: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![ContentBlock::text(value)]))
}

fn json_result(result: OpResult) -> Result<CallToolResult, McpError> {
    match result {
        Ok(value) => Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&value).unwrap_or_default(),
        )])),
        Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(e)])),
    }
}

fn failure(message: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![ContentBlock::text(message.into())]))
}

// ---- parameters --------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewParams {
    /// Document name.
    #[serde(default)]
    pub name: Option<String>,
    /// Board width in studs (1–128, default 16).
    #[serde(default)]
    pub width: Option<i32>,
    /// Board height in studs (1–128, default 16).
    #[serde(default)]
    pub height: Option<i32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PathParams {
    /// File path, relative to the workspace root.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SaveParams {
    /// Where to save (relative to the workspace root). Defaults to the opened file.
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CatalogParams {
    /// Filter: rectilinear, round, curve, wedge or special.
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PieceParams {
    /// Catalog id ("3070") or name ("Tile 1×1").
    pub piece: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PieceSpecParam {
    /// Catalog id or name.
    pub piece: String,
    /// Column of the top-left corner of the rotated footprint (studs, from 0).
    pub x: i32,
    /// Row of the top-left corner of the rotated footprint (studs, from 0).
    pub y: i32,
    /// Clockwise rotation: 0, 90, 180 or 270.
    #[serde(default)]
    pub rotation: i32,
    #[serde(default)]
    pub mirror_x: bool,
    #[serde(default)]
    pub mirror_y: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlaceParams {
    pub pieces: Vec<PieceSpecParam>,
    /// Ink pass id, name or index. Defaults to the active pass.
    #[serde(default)]
    pub pass: Option<String>,
    /// Place all or nothing.
    #[serde(default)]
    pub atomic: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LegendParam {
    pub piece: String,
    #[serde(default)]
    pub rotation: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GridParams {
    /// Rows of characters, one per stud: `#` 1×1 tile, `0`–`3` 1×1 quarter round
    /// at rotation n×90 (0 = curve at bottom-right, 1 bottom-left, 2 top-left,
    /// 3 top-right), `.` or space empty. Other characters need a legend entry.
    pub rows: Vec<String>,
    /// Column of the grid's top-left cell.
    #[serde(default)]
    pub x: i32,
    /// Row of the grid's top-left cell.
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub pass: Option<String>,
    /// Extra characters, e.g. {"o": {"piece": "98138"}} for round 1×1 tiles.
    #[serde(default)]
    pub legend: BTreeMap<String, LegendParam>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryParams {
    /// [x, y, width, height] in studs.
    #[serde(default)]
    pub rect: Option<[i32; 4]>,
    #[serde(default)]
    pub pass: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveParams {
    #[serde(default)]
    pub uids: Vec<String>,
    /// Also remove every piece touching [x, y, width, height].
    #[serde(default)]
    pub rect: Option<[i32; 4]>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveParams {
    pub uids: Vec<String>,
    pub dx: i32,
    pub dy: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddPassParams {
    /// Hex ink colour, e.g. #ce4936.
    pub color: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdatePassParams {
    /// Pass id, name or index.
    pub pass: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub visible: Option<bool>,
    #[serde(default)]
    pub locked: Option<bool>,
    /// New position in the print order (0 prints first).
    #[serde(default)]
    pub index: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PassParams {
    pub pass: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResizeParams {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PrintParams {
    /// Clean, Fresh Press, Normal, Worn, Dry Ink or Heavy Pressure.
    #[serde(default)]
    pub preset: Option<String>,
    /// Any of inkAmount, pressure, inkCoverage (0.7–1), rollerConsistency,
    /// pieceWear, edgeWear, paperTooth, textureScale (0.4–2.5), misalignment,
    /// registrationError, printRoughness; 0–1 unless noted.
    #[serde(default)]
    pub settings: BTreeMap<String, f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PaperParams {
    /// Bright White, Natural White, Warm Ivory, Recycled, Newsprint or Heavy Cotton.
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub grain: Option<f64>,
    #[serde(default)]
    pub tooth: Option<f64>,
    #[serde(default)]
    pub fibers: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OptionsParams {
    /// off, points, lines or squares.
    #[serde(default)]
    pub grid: Option<String>,
    /// 4 or 8.
    #[serde(default)]
    pub major_grid: Option<i32>,
    /// off, x, y or both: pieces placed in the GUI are mirrored automatically.
    #[serde(default)]
    pub symmetry: Option<String>,
    #[serde(default)]
    pub allow_overlap: Option<bool>,
    /// Restrict to standard-height pieces a real press can print together.
    #[serde(default)]
    pub physical: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameParams {
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PreviewParams {
    /// design (flat vector colours) or print (simulated letterpress). Default print.
    #[serde(default)]
    pub mode: Option<String>,
    /// Width in pixels, 64–2048 (default 768).
    #[serde(default)]
    pub width: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportParams {
    /// Output file, relative to the workspace root. `.png` or `.svg`.
    pub path: String,
    /// design or print. Default print.
    #[serde(default)]
    pub mode: Option<String>,
    /// Width in pixels, 64–12000 (default 2048).
    #[serde(default)]
    pub width: Option<u32>,
    /// Include the paper (default true).
    #[serde(default)]
    pub paper: Option<bool>,
    /// Draw the stud grid (default false).
    #[serde(default)]
    pub grid: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Empty {}

fn mode(value: Option<&str>, default: ExportMode) -> Result<ExportMode, String> {
    match value.map(str::to_ascii_lowercase).as_deref() {
        None => Ok(default),
        Some("design") => Ok(ExportMode::Design),
        Some("print") => Ok(ExportMode::Print),
        Some(other) => Err(format!("Unknown mode “{other}”; use design or print.")),
    }
}

fn parse_enum<T: serde::de::DeserializeOwned>(value: Option<String>, what: &str) -> Result<Option<T>, String> {
    value
        .map(|v| serde_json::from_value(Value::String(v.to_lowercase())).map_err(|_| format!("Unknown {what} “{v}”.")))
        .transpose()
}

#[tool_router]
impl BrickpressMcp {
    pub fn new(backend: Backend, root: PathBuf) -> Self {
        Self {
            state: Arc::new(Mutex::new(State { backend, root, renderer: None, atlas: None })),
            tool_router: Self::tool_router(),
        }
    }

    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut state)
    }

    fn op(&self, op: Op) -> Result<CallToolResult, McpError> {
        json_result(self.with(|s| s.call(op)))
    }

    #[tool(
        description = "Start a new, empty project (headless mode). In live mode, ask the person to create it in Brickpress."
    )]
    fn document_new(&self, Parameters(p): Parameters<NewParams>) -> Result<CallToolResult, McpError> {
        self.with(|s| match &mut s.backend {
            Backend::Live(_) => failure("In live mode the person manages files in the Brickpress window."),
            Backend::Headless { core, file } => {
                let mut doc = create_document(false);
                if let Some(name) = p.name {
                    doc.name = name.chars().take(160).collect();
                }
                let (w, h) = (p.width.unwrap_or(doc.board.width), p.height.unwrap_or(doc.board.height));
                if !(1..=128).contains(&w) || !(1..=128).contains(&h) {
                    return failure("Board dimensions are whole studs from 1 to 128.");
                }
                doc.board = brickpress_core::Board { width: w, height: h };
                core.replace_document(doc, false);
                *file = None;
                json_result(Ok(ops::info(core)))
            }
        })
    }

    #[tool(description = "Open a .brickpress.json (or legacy .legopress.json) project from the workspace.")]
    fn document_open(&self, Parameters(p): Parameters<PathParams>) -> Result<CallToolResult, McpError> {
        self.with(|s| {
            let path = match s.resolve(&p.path) {
                Ok(path) => path,
                Err(e) => return failure(e),
            };
            match &mut s.backend {
                Backend::Live(_) => failure("In live mode the person manages files in the Brickpress window."),
                Backend::Headless { core, file } => {
                    let text = match std::fs::read_to_string(&path) {
                        Ok(t) => t,
                        Err(e) => return failure(format!("Could not read {}: {e}", path.display())),
                    };
                    match core.load(&text, false) {
                        Ok(()) => {
                            core.take_notice();
                            *file = Some(path);
                            json_result(Ok(ops::info(core)))
                        }
                        Err(e) => failure(e.0),
                    }
                }
            }
        })
    }

    #[tool(description = "Save the project (headless mode) to its file or a new path in the workspace.")]
    fn document_save(&self, Parameters(p): Parameters<SaveParams>) -> Result<CallToolResult, McpError> {
        self.with(|s| {
            let target = match p.path.as_deref().map(|p| s.resolve(p)).transpose() {
                Ok(t) => t,
                Err(e) => return failure(e),
            };
            match &mut s.backend {
                Backend::Live(_) => failure("In live mode the person saves from the Brickpress window."),
                Backend::Headless { core, file } => {
                    let Some(path) = target.or_else(|| file.clone()) else {
                        return failure("Give a path such as designs/poster.brickpress.json.");
                    };
                    match std::fs::write(&path, serialize_document(core.doc())) {
                        Ok(()) => {
                            core.mark_saved();
                            *file = Some(path.clone());
                            text(format!("Saved {}.", path.display()))
                        }
                        Err(e) => failure(format!("Could not save: {e}")),
                    }
                }
            }
        })
    }

    #[tool(
        description = "Summary of the document: board size, ink passes (with ids and colours), piece count, print and paper settings."
    )]
    fn document_info(&self, Parameters(_): Parameters<Empty>) -> Result<CallToolResult, McpError> {
        self.op(Op::Info)
    }

    #[tool(description = "The full project JSON (version 1 format).")]
    fn document_get(&self, Parameters(_): Parameters<Empty>) -> Result<CallToolResult, McpError> {
        self.op(Op::GetDocument)
    }

    #[tool(
        description = "Text picture of the board: one character per stud, a letter per ink pass, 0–3 for quarter rounds."
    )]
    fn board_ascii(&self, Parameters(_): Parameters<Empty>) -> Result<CallToolResult, McpError> {
        match self.with(|s| s.call(Op::BoardAscii)) {
            Ok(Value::String(s)) => text(s),
            other => json_result(other),
        }
    }

    #[tool(
        description = "List catalog pieces: id, name, category, footprint (studs), allowed rotations and whether they are standard height."
    )]
    fn catalog_list(&self, Parameters(p): Parameters<CatalogParams>) -> Result<CallToolResult, McpError> {
        let filter = p.category.as_deref().map(str::to_lowercase);
        let pieces: Vec<Value> = catalog()
            .pieces
            .iter()
            .filter(|piece| {
                filter.as_deref().is_none_or(|c| piece.category.id() == c || piece.category.label().to_lowercase() == c)
            })
            .map(|piece| {
                json!({
                    "id": piece.id, "name": piece.name, "category": piece.category.id(),
                    "width": piece.footprint.width_studs, "height": piece.footprint.height_studs,
                    "rotations": piece.allowed_rotations, "standardHeight": piece.is_physical(),
                })
            })
            .collect();
        json_result(Ok(json!({ "count": pieces.len(), "categories": Category::ALL.map(|c| c.id()), "pieces": pieces })))
    }

    #[tool(description = "Details of one piece, with its occupancy mask drawn for every rotation (# = occupied stud).")]
    fn catalog_piece(&self, Parameters(p): Parameters<PieceParams>) -> Result<CallToolResult, McpError> {
        let Some(piece) = ops::resolve_piece(&p.piece) else {
            return failure(format!("Unknown piece “{}”.", p.piece));
        };
        let masks: BTreeMap<String, Vec<String>> = piece
            .allowed_rotations
            .iter()
            .map(|r| {
                let mask = rotate_mask(&piece.grid_mask, *r);
                (
                    r.to_string(),
                    mask.iter().map(|row| row.iter().map(|&v| if v == 1 { '#' } else { '.' }).collect()).collect(),
                )
            })
            .collect();
        json_result(Ok(json!({
            "id": piece.id, "name": piece.name, "category": piece.category.id(),
            "width": piece.footprint.width_studs, "height": piece.footprint.height_studs,
            "surfaceHeightPlates": piece.surface_height_plates, "standardHeight": piece.is_physical(),
            "fidelity": piece.geometry.fidelity, "masks": masks,
            "notes": "x/y is the top-left of the rotated footprint. Curved pieces may share a stud when their surfaces do not overlap.",
        })))
    }

    #[tool(
        description = "Place pieces as one undoable step. Each result says whether it was placed or why not (overlap, off the board, rotation not allowed)."
    )]
    fn pieces_place(&self, Parameters(p): Parameters<PlaceParams>) -> Result<CallToolResult, McpError> {
        let pieces = p
            .pieces
            .into_iter()
            .map(|s| PieceSpec {
                piece: s.piece,
                x: s.x,
                y: s.y,
                rotation: s.rotation,
                mirror_x: s.mirror_x,
                mirror_y: s.mirror_y,
            })
            .collect();
        self.op(Op::PlacePieces { pieces, pass: p.pass, atomic: p.atomic })
    }

    #[tool(
        description = "Draw with 1×1 pieces from rows of characters (good for letterforms and pixel shapes). See the rows field for the notation."
    )]
    fn pieces_place_grid(&self, Parameters(p): Parameters<GridParams>) -> Result<CallToolResult, McpError> {
        let legend =
            p.legend.into_iter().map(|(k, v)| (k, LegendEntry { piece: v.piece, rotation: v.rotation })).collect();
        self.op(Op::PlaceGrid { rows: p.rows, x: p.x, y: p.y, pass: p.pass, legend })
    }

    #[tool(
        description = "List placed pieces (uid, piece, position, size, rotation, pass), optionally inside a rectangle or on one pass."
    )]
    fn pieces_query(&self, Parameters(p): Parameters<QueryParams>) -> Result<CallToolResult, McpError> {
        self.op(Op::QueryPieces { rect: p.rect, pass: p.pass })
    }

    #[tool(description = "Remove pieces by uid and/or everything touching a rectangle. Locked passes are left alone.")]
    fn pieces_remove(&self, Parameters(p): Parameters<RemoveParams>) -> Result<CallToolResult, McpError> {
        self.op(Op::RemovePieces { uids: p.uids, rect: p.rect })
    }

    #[tool(description = "Move pieces by whole studs; fails without changes if they would overlap or leave the board.")]
    fn pieces_move(&self, Parameters(p): Parameters<MoveParams>) -> Result<CallToolResult, McpError> {
        self.op(Op::MovePieces { uids: p.uids, dx: p.dx, dy: p.dy })
    }

    #[tool(description = "Add an ink pass (a colour run). Later passes print on top.")]
    fn pass_add(&self, Parameters(p): Parameters<AddPassParams>) -> Result<CallToolResult, McpError> {
        self.op(Op::AddPass { color: p.color, name: p.name })
    }

    #[tool(description = "Rename, recolour, show/hide, lock/unlock or reorder an ink pass.")]
    fn pass_update(&self, Parameters(p): Parameters<UpdatePassParams>) -> Result<CallToolResult, McpError> {
        self.op(Op::UpdatePass {
            pass: p.pass,
            name: p.name,
            color: p.color,
            visible: p.visible,
            locked: p.locked,
            index: p.index,
        })
    }

    #[tool(description = "Delete an ink pass and its pieces (at least one pass must remain).")]
    fn pass_remove(&self, Parameters(p): Parameters<PassParams>) -> Result<CallToolResult, McpError> {
        self.op(Op::RemovePass { pass: p.pass })
    }

    #[tool(description = "Resize the board (1–128 studs per side). Pieces must still fit.")]
    fn board_resize(&self, Parameters(p): Parameters<ResizeParams>) -> Result<CallToolResult, McpError> {
        self.op(Op::ResizeBoard { width: p.width, height: p.height })
    }

    #[tool(description = "Set the simulated press: a preset and/or individual settings.")]
    fn print_settings_set(&self, Parameters(p): Parameters<PrintParams>) -> Result<CallToolResult, McpError> {
        self.op(Op::SetPrint { preset: p.preset, settings: p.settings })
    }

    #[tool(description = "Set the paper stock and texture.")]
    fn paper_set(&self, Parameters(p): Parameters<PaperParams>) -> Result<CallToolResult, McpError> {
        self.op(Op::SetPaper { preset: p.preset, color: p.color, grain: p.grain, tooth: p.tooth, fibers: p.fibers })
    }

    #[tool(
        description = "Set document options: grid style, major grid, symmetry, overlap and physical (standard height only) mode."
    )]
    fn options_set(&self, Parameters(p): Parameters<OptionsParams>) -> Result<CallToolResult, McpError> {
        let grid: Option<GridStyle> = match parse_enum(p.grid, "grid style") {
            Ok(g) => g,
            Err(e) => return failure(e),
        };
        let symmetry: Option<Symmetry> = match parse_enum(p.symmetry, "symmetry") {
            Ok(s) => s,
            Err(e) => return failure(e),
        };
        self.op(Op::SetOptions {
            grid,
            major_grid: p.major_grid,
            symmetry,
            allow_overlap: p.allow_overlap,
            physical: p.physical,
        })
    }

    #[tool(description = "Rename the document.")]
    fn document_rename(&self, Parameters(p): Parameters<RenameParams>) -> Result<CallToolResult, McpError> {
        self.op(Op::Rename { name: p.name })
    }

    #[tool(description = "Give every piece and pass a new random print variation.")]
    fn print_reseed(&self, Parameters(_): Parameters<Empty>) -> Result<CallToolResult, McpError> {
        self.op(Op::Reseed)
    }

    #[tool(description = "Undo the last change (the person sees it undone too in live mode).")]
    fn undo(&self, Parameters(_): Parameters<Empty>) -> Result<CallToolResult, McpError> {
        self.op(Op::Undo)
    }

    #[tool(description = "Redo the last undone change.")]
    fn redo(&self, Parameters(_): Parameters<Empty>) -> Result<CallToolResult, McpError> {
        self.op(Op::Redo)
    }

    #[tool(
        description = "Render the board as a PNG image to look at your work. Use mode design to check placement, print for the letterpress look."
    )]
    async fn render_preview(&self, Parameters(p): Parameters<PreviewParams>) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<String, String> {
            let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
            let doc = s.document()?;
            let mode = mode(p.mode.as_deref(), ExportMode::Print)?;
            let width = p.width.unwrap_or(768).clamp(64, 2048);
            let r = s.renderer()?;
            let options = ExportOptions { mode, paper: true, grid: mode == ExportMode::Design, width };
            let image = render_rgba(&r.gpu.device, &r.gpu.queue, &r.r2d, &doc, GridAppearance::Flat, &options)
                .map_err(|e| e.to_string())?;
            let png = encode_png(&image).map_err(|e| e.to_string())?;
            Ok(base64::engine::general_purpose::STANDARD.encode(png))
        })
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        match result {
            Ok(data) => Ok(CallToolResult::success(vec![ContentBlock::image(data, "image/png")])),
            Err(e) => failure(e),
        }
    }

    #[tool(description = "Export the artwork to a PNG or SVG file in the workspace.")]
    async fn export_image(&self, Parameters(p): Parameters<ExportParams>) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<String, String> {
            let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
            let path = s.resolve(&p.path)?;
            let doc = s.document()?;
            let mode = mode(p.mode.as_deref(), ExportMode::Print)?;
            let options = ExportOptions {
                mode,
                paper: p.paper.unwrap_or(true),
                grid: p.grid.unwrap_or(false),
                width: p.width.unwrap_or(2048),
            };
            let svg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
            let r = s.renderer()?;
            let bytes = if svg {
                let masks = if mode == ExportMode::Print {
                    brickpress_render::svg::print_pass_masks(&r.gpu.device, &r.gpu.queue, &r.r2d, &doc, &options)
                        .map_err(|e| e.to_string())?
                } else {
                    Vec::new()
                };
                brickpress_render::svg::render_svg(&doc, &options, &masks).map_err(|e| e.to_string())?.into_bytes()
            } else {
                let image = render_rgba(&r.gpu.device, &r.gpu.queue, &r.r2d, &doc, GridAppearance::Flat, &options)
                    .map_err(|e| e.to_string())?;
                encode_png(&image).map_err(|e| e.to_string())?
            };
            std::fs::write(&path, bytes).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
            Ok(format!("Exported {}.", path.display()))
        })
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        match result {
            Ok(message) => text(message),
            Err(e) => failure(e),
        }
    }
}

#[tool_handler]
impl ServerHandler for BrickpressMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("brickpress", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}

const INSTRUCTIONS: &str = "Brickpress composes artwork from LEGO-style tiles on a stud grid and simulates letterpress prints of it.\n\
Coordinates are whole studs; x grows right, y grows down; a piece's x/y is the top-left of its rotated footprint.\n\
Ink passes are colour runs printed in order. Pieces may not overlap unless allowOverlap is on (curved pieces can share a stud when their surfaces fit together).\n\
Workflow: document_info → catalog_list → pieces_place / pieces_place_grid → render_preview (mode design to check placement, print for the result) → document_save or export_image.\n\
Every editing tool is one undoable step. In live mode your edits appear in the person's open Brickpress window.";
