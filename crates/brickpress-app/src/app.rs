//! Application state, the document tabs and their lifecycle, and the frame loop.

use crate::agents::AgentServer;
use crate::commands::{handle_shortcuts, Command};
use crate::gpu::TraceUpload;
use crate::storage::{self, Fingerprint, StateStore, StorageError, TrackedFile};
use crate::tabs::DocumentTab;
use crate::theme;
use crate::toasts::{ToastKind, Toasts};
use brickpress_core::persistence::document::{create_document, parse_document, serialize_document};
use brickpress_core::persistence::preferences::{parse_preferences, serialize_preferences};
use brickpress_core::persistence::presets::{document_from_preset, parse_presets, serialize_presets};
use brickpress_core::persistence::tracing::TraceMeta;
use brickpress_core::{EditorCore, Id, Mode, PressDocument, Tool};
use brickpress_render::atlas::ShapeAtlas;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    #[default]
    TwoD,
    ThreeD,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InspectorTab {
    Design,
    Inks,
    Print,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LibraryTab {
    Pieces,
    Presets,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelLayout {
    pub library: bool,
    pub inspector: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Png,
    Svg,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportSettings {
    pub format: ExportFormat,
    /// `None` follows the current view.
    pub print: Option<bool>,
    pub width: u32,
    pub paper: bool,
    pub grid: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self { format: ExportFormat::Png, print: None, width: 2048, paper: true, grid: false }
    }
}

/// What to do after the unsaved-changes question is answered.
#[derive(Clone, Debug, PartialEq)]
pub enum Pending {
    /// Close the tab with this id.
    CloseTab(u64),
    /// Quit; every tab with unsaved changes is asked about in turn.
    Quit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Confirm {
    ClearBoard,
    RemovePass(Id),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Modal {
    Unsaved(Pending),
    Conflict,
    Confirm(Confirm),
    SavePreset { name: String, selection_only: bool },
    RenamePreset { id: String, name: String },
    SavePrintPreset { name: String },
    Palette { query: String, index: usize },
    Shortcuts,
    About,
    Notices,
}

/// An open tab, as remembered between sessions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TabRecord {
    /// Names the tab's recovery record (`recovery-<key>`).
    pub key: String,
    /// The file it belongs to, with its fingerprint at the last open or save.
    pub file: Option<(PathBuf, Option<String>)>,
    /// Unsaved changes are kept in the tab's recovery record.
    pub dirty: bool,
    pub view: View,
    pub trace: Option<TraceMeta>,
}

/// Remembered between sessions (in `app.json`, beside the 1.x records).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub recent: Vec<PathBuf>,
    /// The active tab's file (single-document versions reopen it).
    pub last_file: Option<PathBuf>,
    /// The file the 1.x-compatible `recovery` record belongs to, with its fingerprint at the last save.
    pub recovery_file: Option<(PathBuf, Option<String>)>,
    pub layout: PanelLayout,
    pub view: View,
    pub show_grid: bool,
    pub export: ExportSettings,
    pub agents: bool,
    pub first_run_done: bool,
    /// Open tabs in order. Empty in state written by single-document versions.
    pub tabs: Vec<TabRecord>,
    pub active_tab: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            recent: Vec::new(),
            last_file: None,
            recovery_file: None,
            layout: PanelLayout { library: true, inspector: true },
            view: View::TwoD,
            show_grid: true,
            export: ExportSettings::default(),
            agents: false,
            first_run_done: false,
            tabs: Vec::new(),
            active_tab: 0,
        }
    }
}

pub struct TraceState {
    pub meta: TraceMeta,
    pub upload: TraceUpload,
}

/// Messages from background work (exports, file writes).
pub enum JobResult {
    Done(String),
    Failed(String),
}

struct StateWrite {
    key: String,
    value: Option<String>,
}

pub const NO_DOCUMENT: &str = "Open or create a project first";

pub struct BrickpressApp {
    /// Open documents, in tab order.
    pub tabs: Vec<DocumentTab>,
    /// Index of the active tab (meaningful while `tabs` is not empty).
    pub active: usize,
    /// Holds the window's shared editor state (presets, preferences,
    /// clipboard) while no document is open.
    shelf: EditorCore,
    pub atlas: Arc<ShapeAtlas>,
    pub store: Option<StateStore>,
    pub show_grid: bool,
    pub layout: PanelLayout,
    pub inspector_tab: InspectorTab,
    pub library_tab: LibraryTab,
    pub library_search: String,
    pub library_category: Option<brickpress_core::Category>,
    /// The preset last chosen in the library (for Open as New Project).
    pub library_preset: Option<String>,
    pub export: ExportSettings,
    pub modal: Option<Modal>,
    pub toasts: Toasts,
    pub recent: Vec<PathBuf>,
    pub thumbnails: Vec<egui::TextureHandle>,
    pub preset_thumbnails: std::collections::HashMap<String, (u64, egui::TextureHandle)>,
    pub agents: AgentServer,
    pub render_device: Option<(brickpress_render::wgpu::Device, brickpress_render::wgpu::Queue)>,
    pub jobs: Vec<Receiver<JobResult>>,
    pub busy: usize,
    pub quitting: bool,
    presets_saved: u64,
    preferences_saved: u64,
    state_writer: Option<Sender<StateWrite>>,
    app_state: AppState,
}

impl BrickpressApp {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>) -> Self {
        Self::with_data_dir(cc, open, StateStore::default_root())
    }

    /// Like [`new`](Self::new) with device state kept in `data_dir` (tests use a temporary folder).
    pub fn with_data_dir(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>, data_dir: PathBuf) -> Self {
        theme::install(&cc.egui_ctx);
        cc.egui_ctx.set_theme(egui::ThemePreference::System);
        let atlas = Arc::new(ShapeAtlas::build());
        let render_device = cc.wgpu_render_state.as_ref().map(|rs| {
            crate::gpu::install(rs, atlas.clone());
            (rs.device.clone(), rs.queue.clone())
        });
        let mut app = Self::headless(atlas, render_device, data_dir);
        app.load_thumbnails(&cc.egui_ctx);
        app.restore(open);
        if app.agents.enabled() {
            app.agents.start(&cc.egui_ctx);
        }
        app
    }

    /// The app without a window (also used by tests), showing the starter document.
    pub fn headless(
        atlas: Arc<ShapeAtlas>,
        render_device: Option<(brickpress_render::wgpu::Device, brickpress_render::wgpu::Queue)>,
        data_dir: PathBuf,
    ) -> Self {
        let store = StateStore::open(data_dir).ok();
        let state_writer = store.clone().map(|store| {
            let (tx, rx) = channel::<StateWrite>();
            std::thread::Builder::new()
                .name("brickpress-state".into())
                .spawn(move || {
                    for write in rx {
                        if let Err(e) = store.write(&write.key, write.value.as_deref()) {
                            log::warn!("Could not write {}: {e}", write.key);
                        }
                    }
                })
                .expect("spawn state writer");
            tx
        });
        Self {
            tabs: vec![DocumentTab::clean(EditorCore::default(), None)],
            active: 0,
            shelf: EditorCore::new(create_document(false)),
            atlas,
            store,
            show_grid: true,
            layout: PanelLayout { library: true, inspector: true },
            inspector_tab: InspectorTab::Design,
            library_tab: LibraryTab::Pieces,
            library_search: String::new(),
            library_category: None,
            library_preset: None,
            export: ExportSettings::default(),
            modal: None,
            toasts: Toasts::default(),
            recent: Vec::new(),
            thumbnails: Vec::new(),
            preset_thumbnails: Default::default(),
            agents: AgentServer::default(),
            render_device,
            jobs: Vec::new(),
            busy: 0,
            quitting: false,
            presets_saved: 0,
            preferences_saved: 0,
            state_writer,
            app_state: AppState::default(),
        }
    }

    fn load_thumbnails(&mut self, ctx: &egui::Context) {
        let catalog = brickpress_core::catalog();
        self.thumbnails = (0..catalog.pieces.len())
            .map(|i| {
                let (w, h, alpha) = self.atlas.thumbnail(i, 96);
                let pixels: Vec<egui::Color32> = alpha.iter().map(|&a| egui::Color32::from_white_alpha(a)).collect();
                let image = egui::ColorImage::new([w as usize, h as usize], pixels);
                ctx.load_texture(format!("piece-{}", catalog.pieces[i].id), image, egui::TextureOptions::LINEAR)
            })
            .collect();
    }

    // ---- the active document -----------------------------------------------

    pub fn has_document(&self) -> bool {
        !self.tabs.is_empty()
    }

    pub fn active_tab(&self) -> Option<&DocumentTab> {
        self.tabs.get(self.active)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut DocumentTab> {
        self.tabs.get_mut(self.active)
    }

    /// The active tab. Only for code that runs while a document is open.
    pub fn tab(&self) -> &DocumentTab {
        self.active_tab().expect("a document is open")
    }

    /// The active tab. Only for code that runs while a document is open.
    pub fn tab_mut(&mut self) -> &mut DocumentTab {
        self.active_tab_mut().expect("a document is open")
    }

    /// The active document's editor. With no document open this is an
    /// empty editor that only holds the shared state (presets, preferences).
    pub fn core(&self) -> &EditorCore {
        self.active_tab().map_or(&self.shelf, |t| &t.core)
    }

    pub fn core_mut(&mut self) -> &mut EditorCore {
        match self.tabs.get_mut(self.active) {
            Some(tab) => &mut tab.core,
            None => &mut self.shelf,
        }
    }

    pub fn view(&self) -> View {
        self.active_tab().map_or(View::TwoD, |t| t.view)
    }

    pub fn modal_open(&self) -> bool {
        self.modal.is_some()
    }

    pub fn notify(&mut self, text: impl Into<String>) {
        self.toasts.push(ToastKind::Info, text);
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.toasts.push(ToastKind::Error, text);
    }

    pub fn document_title(&self) -> String {
        self.active_tab().map(|t| t.title()).unwrap_or_else(|| "No project open".into())
    }

    /// Whether the active document has unsaved changes.
    pub fn dirty(&self) -> bool {
        self.active_tab().is_some_and(|t| t.dirty())
    }

    // ---- commands ----------------------------------------------------------

    pub fn run(&mut self, command: Command, ctx: &egui::Context) {
        if let Err(reason) = command.availability(self) {
            let quiet = matches!(command, Command::Deselect | Command::Delete)
                || (!self.has_document() && command.is_plain_key());
            if !quiet {
                self.notify(reason);
            }
            return;
        }
        match command {
            Command::New => self.new_tab(),
            Command::Open => self.open_dialog(),
            Command::CloseTab => {
                if let Some(id) = self.active_tab().map(|t| t.id) {
                    self.request(Pending::CloseTab(id));
                }
            }
            Command::NextTab => self.cycle_tab(1),
            Command::PreviousTab => self.cycle_tab(-1),
            Command::OpenPresetAsDocument => {
                if let Some(id) = self.preset_target() {
                    self.open_preset_document(&id);
                }
            }
            Command::Save => {
                self.save(false);
            }
            Command::SaveAs => {
                self.save(true);
            }
            Command::Export => {
                self.layout.inspector = true;
                self.inspector_tab = InspectorTab::Export;
            }
            Command::ImportPresets => self.import_presets(),
            Command::ExportPresets => self.export_presets(),
            Command::Quit => self.request(Pending::Quit),
            Command::Undo => {
                self.core_mut().undo();
            }
            Command::Redo => {
                self.core_mut().redo();
            }
            Command::Cut => self.core_mut().cut(),
            Command::Copy => self.core_mut().copy(),
            Command::Paste => self.core_mut().paste(),
            Command::Duplicate => self.core_mut().duplicate(),
            Command::Delete => {
                self.core_mut().remove_selected();
            }
            Command::SelectAll => {
                let core = self.core_mut();
                core.set_tool(Tool::Select);
                core.select_all();
            }
            Command::Deselect => {
                let core = self.core_mut();
                if core.active_preset_id.is_some() {
                    core.cancel_preset();
                    self.library_preset = None;
                } else if core.placement_stroke_active() {
                    core.finish_placement_stroke(true);
                } else if core.tool == Tool::Place && core.selected.is_empty() {
                    core.set_tool(Tool::Select);
                } else {
                    core.clear_selection();
                }
            }
            Command::SelectTool => self.core_mut().set_tool(Tool::Select),
            Command::PlaceTool => {
                let core = self.core_mut();
                if core.mode != Mode::Design {
                    core.set_mode(Mode::Design);
                }
                let id = core.active_piece_id.clone();
                core.choose_piece(&id);
            }
            Command::HandTool => self.core_mut().set_tool(Tool::Hand),
            Command::Rotate => self.core_mut().rotate(),
            Command::MirrorHorizontal => self.core_mut().mirror(true),
            Command::MirrorVertical => self.core_mut().mirror(false),
            Command::BringForward => {
                self.core_mut().reorder_selected(1);
            }
            Command::SendBackward => {
                self.core_mut().reorder_selected(-1);
            }
            Command::DesignView => {
                let tab = self.tab_mut();
                tab.view = View::TwoD;
                tab.core.set_mode(Mode::Design);
                if self.inspector_tab == InspectorTab::Print {
                    self.inspector_tab = InspectorTab::Design;
                }
            }
            Command::PrintView => {
                let tab = self.tab_mut();
                tab.view = View::TwoD;
                tab.core.set_mode(Mode::Print);
                if tab.core.tool == Tool::Place {
                    tab.core.set_tool(Tool::Select);
                }
                if self.inspector_tab == InspectorTab::Design {
                    self.inspector_tab = InspectorTab::Print;
                }
            }
            Command::View3D => {
                let tab = self.tab_mut();
                tab.view = View::ThreeD;
                tab.core.set_mode(Mode::Design);
            }
            Command::ZoomIn => self.zoom_by(1.25),
            Command::ZoomOut => self.zoom_by(0.8),
            Command::ZoomFit => self.zoom_fit(),
            Command::ZoomActual => self.zoom_to(1.0),
            Command::ToggleGrid => self.show_grid = !self.show_grid,
            Command::ToggleLibrary => self.layout.library = !self.layout.library,
            Command::ToggleInspector => self.layout.inspector = !self.layout.inspector,
            Command::ResetLayout => {
                self.layout = PanelLayout { library: true, inspector: true };
                ctx.memory_mut(|m| m.reset_areas());
                ctx.data_mut(|d| d.clear());
            }
            Command::CommandPalette => self.modal = Some(Modal::Palette { query: String::new(), index: 0 }),
            Command::Shortcuts => self.modal = Some(Modal::Shortcuts),
            Command::AddPass => {
                let core = self.core_mut();
                let color = crate::inspector::next_swatch(core.doc());
                core.add_pass(&color);
                self.inspector_tab = InspectorTab::Inks;
                self.layout.inspector = true;
            }
            Command::Reseed => {
                self.core_mut().reseed();
                self.notify("New print variation.");
            }
            Command::ClearBoard => self.modal = Some(Modal::Confirm(Confirm::ClearBoard)),
            Command::AddTracing => self.choose_trace(),
            Command::RemoveTracing => self.remove_trace(),
            Command::SavePreset => {
                let selection_only = !self.core().selected_pieces().is_empty();
                self.modal = Some(Modal::SavePreset { name: String::new(), selection_only })
            }
            Command::AgentConnections => {
                let enabled = !self.agents.enabled();
                self.agents.set_enabled(enabled);
                if enabled {
                    self.agents.start(ctx);
                    self.notify("Agents can now connect with `brickpress mcp --live`.");
                } else {
                    self.agents.stop();
                    self.notify("Agent connections are off.");
                }
                self.persist_app_state();
            }
            Command::About => self.modal = Some(Modal::About),
            Command::Notices => self.modal = Some(Modal::Notices),
        }
    }

    pub fn zoom_by(&mut self, factor: f32) {
        let Some(tab) = self.active_tab_mut() else { return };
        match tab.view {
            View::TwoD => tab.canvas.zoom_by(factor, None),
            View::ThreeD => tab.canvas3d.zoom_by(&mut tab.camera, factor),
        }
    }
    pub fn zoom_fit(&mut self) {
        let Some(tab) = self.active_tab_mut() else { return };
        match tab.view {
            View::TwoD => tab.canvas.fit(),
            View::ThreeD => tab.camera.set_zoom(brickpress_render::three::camera::Zoom::Fit),
        }
    }
    pub fn zoom_to(&mut self, scale: f32) {
        let Some(tab) = self.active_tab_mut() else { return };
        match tab.view {
            View::TwoD => tab.canvas.set_scale(scale),
            View::ThreeD => tab.camera.set_zoom(brickpress_render::three::camera::Zoom::Scale(scale)),
        }
    }

    // ---- tabs --------------------------------------------------------------

    /// The tab that holds the shared editor state, or `None` for the shelf.
    fn holder(&self) -> Option<usize> {
        self.has_document().then_some(self.active)
    }

    /// Moves the shared editor state between tabs (`None`: the shelf).
    fn move_shared(&mut self, from: Option<usize>, to: Option<usize>) {
        match (from, to) {
            (Some(f), Some(t)) if f != t => {
                let (source, target) = if f < t {
                    let (left, right) = self.tabs.split_at_mut(t);
                    (&mut left[f], &mut right[0])
                } else {
                    let (left, right) = self.tabs.split_at_mut(f);
                    (&mut right[0], &mut left[t])
                };
                target.core.adopt_shared(&mut source.core);
            }
            (Some(f), None) => self.shelf.adopt_shared(&mut self.tabs[f].core),
            (None, Some(t)) => self.tabs[t].core.adopt_shared(&mut self.shelf),
            _ => {}
        }
    }

    /// Makes the tab at `index` the active one.
    pub fn activate(&mut self, index: usize) {
        if index >= self.tabs.len() || index == self.active {
            return;
        }
        // Gestures never span tabs.
        let previous = &mut self.tabs[self.active].core;
        previous.finish_placement_stroke(false);
        previous.end_live();
        self.move_shared(Some(self.active), Some(index));
        self.active = index;
        self.persist_trace();
        self.sync_recovery();
    }

    fn cycle_tab(&mut self, step: isize) {
        let n = self.tabs.len() as isize;
        if n > 1 {
            self.activate((self.active as isize + step).rem_euclid(n) as usize);
        }
    }

    /// Adds `tab` and activates it. With `replace_pristine`, an untouched,
    /// never-saved document that is the only tab makes way for it.
    pub fn add_tab(&mut self, tab: DocumentTab, replace_pristine: bool) {
        let replace = replace_pristine && self.tabs.len() == 1 && self.tabs[0].pristine();
        let from = self.holder();
        self.tabs.push(tab);
        let to = self.tabs.len() - 1;
        self.move_shared(from, Some(to));
        self.active = to;
        if replace {
            let old = self.tabs.remove(0);
            self.active -= 1;
            self.write_state(old.recovery_key(), None);
        }
        self.persist_trace();
        self.sync_recovery();
    }

    pub fn new_tab(&mut self) {
        self.add_tab(DocumentTab::clean(EditorCore::new(create_document(false)), None), false);
    }

    fn tab_index_for(&self, path: &Path) -> Option<usize> {
        let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let wanted = canonical(path);
        self.tabs.iter().position(|t| t.file.as_ref().is_some_and(|f| f.path == path || canonical(&f.path) == wanted))
    }

    /// Closes a tab without asking (see [`request`](Self::request)).
    pub fn close_tab_now(&mut self, id: u64) {
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else { return };
        let len = self.tabs.len();
        if index == self.active {
            let next = (len > 1).then(|| if index + 1 < len { index + 1 } else { index - 1 });
            self.move_shared(Some(index), next);
            let tab = self.tabs.remove(index);
            self.active = next.map_or(0, |n| if n > index { n - 1 } else { n });
            self.write_state(tab.recovery_key(), None);
        } else {
            let tab = self.tabs.remove(index);
            if index < self.active {
                self.active -= 1;
            }
            self.write_state(tab.recovery_key(), None);
        }
        self.persist_trace();
        self.sync_recovery();
    }

    /// Runs `pending` now, or asks about unsaved changes first.
    pub fn request(&mut self, pending: Pending) {
        match pending {
            Pending::CloseTab(id) => {
                let Some(index) = self.tabs.iter().position(|t| t.id == id) else { return };
                if self.tabs[index].dirty() {
                    self.activate(index);
                    self.modal = Some(Modal::Unsaved(Pending::CloseTab(id)));
                } else {
                    self.close_tab_now(id);
                }
            }
            Pending::Quit => match self.tabs.iter().position(|t| t.dirty()) {
                Some(index) => {
                    self.activate(index);
                    self.modal = Some(Modal::Unsaved(Pending::Quit));
                }
                None => self.quitting = true,
            },
        }
    }

    /// Answers the unsaved-changes question for the active tab: save it
    /// first, or discard its changes. Cancelling is closing the dialog.
    pub fn resolve_unsaved(&mut self, pending: Pending, save: bool) {
        self.modal = None;
        if save && !self.save(false) {
            return;
        }
        match pending {
            Pending::CloseTab(id) => self.close_tab_now(id),
            Pending::Quit => {
                if !save {
                    if let Some(id) = self.active_tab().map(|t| t.id) {
                        self.close_tab_now(id);
                    }
                }
                self.request(Pending::Quit);
            }
        }
    }

    /// Other tabs with unsaved changes, for the quit question.
    pub fn other_dirty_tabs(&self) -> usize {
        self.tabs.iter().enumerate().filter(|(i, t)| *i != self.active && t.dirty()).count()
    }

    // ---- document lifecycle ------------------------------------------------

    fn open_dialog(&mut self) {
        let dialog = rfd::FileDialog::new().set_title("Open project").add_filter("Brickpress project", &["json"]);
        let dialog = match self.active_tab().and_then(|t| t.file.as_ref()).and_then(|f| f.path.parent()) {
            Some(dir) => dialog.set_directory(dir),
            None => dialog,
        };
        if let Some(path) = dialog.pick_file() {
            self.open_path(path);
        }
    }

    /// Opens a project in a new tab, or activates the tab that already has it.
    /// A failure leaves every open document untouched.
    pub fn open_path(&mut self, path: PathBuf) {
        if let Some(index) = self.tab_index_for(&path) {
            self.activate(index);
            return;
        }
        match read_project(&path) {
            Ok((doc, print)) => {
                let tab = DocumentTab::clean(
                    EditorCore::new(doc),
                    Some(TrackedFile { path: path.clone(), baseline: Some(print) }),
                );
                self.add_tab(tab, true);
                self.add_recent(path);
                self.persist_app_state();
                self.notify(format!("Opened {}.", self.document_title()));
            }
            Err(e) => self.error(format!(
                "Could not open {}: {e}",
                path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
            )),
        }
    }

    /// The preset that Open as New Project acts on.
    pub fn preset_target(&self) -> Option<String> {
        let core = self.core();
        let exists = |id: &String| core.presets.iter().any(|p| &p.id == id);
        self.library_preset.clone().filter(exists).or_else(|| core.active_preset_id.clone().filter(exists))
    }

    /// Opens a library preset as a new, unsaved project in its own tab.
    pub fn open_preset_document(&mut self, id: &str) {
        let Some(preset) = self.core().presets.iter().find(|p| p.id == id).cloned() else { return };
        let mut core = EditorCore::new(document_from_preset(&preset));
        core.dirty = true;
        self.add_tab(DocumentTab::new(core, None, None), true);
        self.notify(format!("Opened “{}” as a new project.", preset.name));
    }

    fn add_recent(&mut self, path: PathBuf) {
        self.recent.retain(|p| *p != path);
        self.recent.insert(0, path);
        self.recent.truncate(10);
    }

    /// Saves the active document to its file, or asks where. Returns true when saved.
    pub fn save(&mut self, save_as: bool) -> bool {
        let Some(tab) = self.active_tab() else { return false };
        let text = tab.core.project_text();
        let mut target = match (&tab.file, save_as) {
            (Some(file), false) => file.clone(),
            _ => {
                let name = format!("{}.brickpress.json", brickpress_render::export::slug(&tab.core.doc().name));
                let dialog = rfd::FileDialog::new()
                    .set_title("Save project")
                    .add_filter("Brickpress project", &["json"])
                    .set_file_name(name);
                let dialog = match tab.file.as_ref().and_then(|f| f.path.parent()) {
                    Some(dir) => dialog.set_directory(dir),
                    None => dialog,
                };
                let Some(path) = dialog.save_file() else { return false };
                let baseline = storage::fingerprint(&path, storage::INPUT_LIMIT).ok().flatten();
                TrackedFile { path, baseline }
            }
        };
        match target.write(text.as_bytes(), storage::INPUT_LIMIT) {
            Ok(()) => {
                let path = target.path.clone();
                let tab = self.tab_mut();
                tab.file = Some(target);
                tab.saved_text = Some(text);
                tab.core.mark_saved();
                tab.autosave_due = None;
                let key = tab.recovery_key();
                self.write_state(key, None);
                self.add_recent(path);
                self.sync_recovery();
                self.notify(format!("Saved {}.", self.document_title()));
                true
            }
            Err(StorageError::Conflict) => {
                self.modal = Some(Modal::Conflict);
                false
            }
            Err(e) => {
                self.error(format!("Could not save: {e}"));
                false
            }
        }
    }

    // ---- presets and preferences -------------------------------------------

    fn import_presets(&mut self) {
        let Some(path) =
            rfd::FileDialog::new().set_title("Import presets").add_filter("Brickpress presets", &["json"]).pick_file()
        else {
            return;
        };
        let result = storage::read_limited(&path, storage::INPUT_LIMIT)
            .map_err(|e| e.to_string())
            .and_then(|(bytes, _)| self.core_mut().import_presets(&String::from_utf8_lossy(&bytes)).map_err(|e| e.0));
        match result {
            Ok(_) => {
                self.library_tab = LibraryTab::Presets;
                self.layout.library = true;
            }
            Err(e) => self.error(format!("Could not import presets: {e}")),
        }
    }

    fn export_presets(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export presets")
            .add_filter("Brickpress presets", &["json"])
            .set_file_name("library.brickpress-presets.json")
            .save_file()
        else {
            return;
        };
        match storage::atomic_replace(&path, serialize_presets(&self.core().presets).as_bytes()) {
            Ok(()) => self.notify(format!("Exported {} presets.", self.core().presets.len())),
            Err(e) => self.error(format!("Could not export presets: {e}")),
        }
    }

    // ---- tracing guide ------------------------------------------------------

    pub fn choose_trace(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Choose a tracing image")
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif", "bmp", "avif"])
            .pick_file()
        else {
            return;
        };
        match crate::trace::load_file(&path) {
            Ok((bytes, upload, mime)) => {
                let board = self.core().doc().board;
                let mut meta = TraceMeta {
                    asset_id: String::new(),
                    mime: mime.into(),
                    name: path.file_name().map(|n| n.to_string_lossy().chars().take(160).collect()).unwrap_or_default(),
                    image_width: upload.size[0] as f64,
                    image_height: upload.size[1] as f64,
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                    opacity: 0.4,
                    visible: true,
                    locked: true,
                };
                meta.fit(board.width as f64, board.height as f64);
                if let Some(store) = &self.store {
                    match store.write_asset(&bytes) {
                        Ok(id) => meta.asset_id = id,
                        Err(e) => {
                            self.error(format!("The tracing image could not be stored: {e}"));
                            return;
                        }
                    }
                }
                let tab = self.tab_mut();
                tab.trace = Some(TraceState { meta, upload });
                if tab.core.mode != Mode::Design {
                    tab.core.set_mode(Mode::Design);
                }
                self.persist_trace();
                self.notify("Tracing image added beneath your pieces. It is not part of the project or exports.");
            }
            Err(e) => self.error(e),
        }
    }

    pub fn remove_trace(&mut self) {
        if let Some(tab) = self.active_tab_mut() {
            tab.trace = None;
        }
        self.persist_trace();
    }

    /// Saves the tracing guides: each tab's in `app.json`, the active tab's
    /// also in the 1.x `trace-metadata` record. Images no tab uses are removed.
    pub fn persist_trace(&mut self) {
        let active = self.active_tab().and_then(|t| t.trace.as_ref());
        let text = active.map(|t| serde_json::to_string(&t.meta).expect("serializable"));
        self.write_state("trace-metadata", text);
        if let Some(store) = &self.store {
            let keep: Vec<&str> =
                self.tabs.iter().filter_map(|t| t.trace.as_ref().map(|tr| tr.meta.asset_id.as_str())).collect();
            let _ = store.prune_assets(&keep);
        }
        self.persist_app_state();
    }

    // ---- persistence ---------------------------------------------------------

    fn write_state(&self, key: impl Into<String>, value: Option<String>) {
        if let Some(tx) = &self.state_writer {
            let _ = tx.send(StateWrite { key: key.into(), value });
        }
    }

    /// The tab mirrored into the 1.x `recovery` record: the active tab when it
    /// has unsaved changes, otherwise the first tab that has.
    fn mirror_tab(&self) -> Option<&DocumentTab> {
        self.active_tab().filter(|t| t.dirty()).or_else(|| self.tabs.iter().find(|t| t.dirty()))
    }

    /// Rewrites the 1.x `recovery` record and `app.json` for the current tabs.
    fn sync_recovery(&mut self) {
        let text = self.mirror_tab().map(|t| serialize_document(t.core.doc()));
        self.write_state("recovery", text);
        self.persist_app_state();
    }

    pub fn persist_app_state(&mut self) {
        let record = |f: &TrackedFile| (f.path.clone(), f.baseline.as_ref().map(|b| b.to_hex()));
        let state = AppState {
            recent: self.recent.clone(),
            last_file: self.active_tab().and_then(|t| t.file.as_ref()).map(|f| f.path.clone()),
            recovery_file: self.mirror_tab().and_then(|t| t.file.as_ref()).map(record),
            layout: self.layout,
            view: self.view(),
            show_grid: self.show_grid,
            export: self.export,
            agents: self.agents.enabled(),
            first_run_done: true,
            tabs: self
                .tabs
                .iter()
                .map(|t| TabRecord {
                    key: t.key.to_string(),
                    file: t.file.as_ref().map(record),
                    dirty: t.dirty(),
                    view: t.view,
                    trace: t.trace.as_ref().map(|tr| tr.meta.clone()),
                })
                .collect(),
            active_tab: self.active,
        };
        self.app_state = state.clone();
        self.write_state("app", Some(serde_json::to_string_pretty(&state).expect("serializable")));
    }

    /// Debounced recovery autosave for every tab, plus preset and preference persistence.
    fn autosave(&mut self, ctx: &egui::Context) {
        let pointer_busy = ctx.input(|i| i.pointer.any_down());
        let now = Instant::now();
        let mut writes = Vec::new();
        for tab in &mut self.tabs {
            let generation = tab.core.generation();
            if generation != tab.autosave_generation {
                tab.autosave_generation = generation;
                refresh_dirty(tab);
                tab.autosave_due = Some(now + Duration::from_millis(800));
            }
            if let Some(due) = tab.autosave_due {
                if now >= due && !pointer_busy && !tab.core.placement_stroke_active() {
                    tab.autosave_due = None;
                    writes.push((tab.recovery_key(), tab.dirty().then(|| serialize_document(tab.core.doc()))));
                } else {
                    ctx.request_repaint_after(Duration::from_millis(250));
                }
            }
        }
        if !writes.is_empty() {
            for (key, value) in writes {
                self.write_state(key, value);
            }
            self.sync_recovery();
        }
        let core = self.core();
        let (presets, preferences) = (core.presets_generation, core.preferences_generation);
        if presets != self.presets_saved {
            self.presets_saved = presets;
            self.write_state("presets", Some(serialize_presets(&self.core().presets)));
        }
        if preferences != self.preferences_saved {
            self.preferences_saved = preferences;
            self.write_state("preferences", Some(serialize_preferences(&self.core().preferences)));
        }
    }

    // ---- restoring the last session -----------------------------------------

    /// Reads a record, setting it aside (with a message) when it is invalid.
    fn read_valid<T>(
        &mut self,
        store: &StateStore,
        key: &str,
        parse: impl FnOnce(&str) -> Result<T, String>,
    ) -> Option<T> {
        match store.read(key) {
            Ok(Some(text)) => match parse(&text) {
                Ok(value) => Some(value),
                Err(e) => {
                    log::warn!("Invalid {key} record: {e}");
                    let _ = store.quarantine(key);
                    self.toasts.push(ToastKind::Error, "A saved record was unreadable and has been set aside.");
                    None
                }
            },
            Ok(None) => None,
            Err(e) => {
                log::warn!("Could not read {key}: {e}");
                None
            }
        }
    }

    fn load_trace(&self, meta: TraceMeta) -> Result<TraceState, String> {
        meta.validate()?;
        let store = self.store.as_ref().ok_or("no storage")?;
        let bytes = store.read_asset(&meta.asset_id).map_err(|e| e.to_string())?;
        let upload = crate::trace::decode(&bytes).map_err(|e| e.to_string())?;
        Ok(TraceState { meta, upload })
    }

    /// Reopens a remembered tab: its unsaved changes when it had some,
    /// otherwise its file. Clean, never-saved tabs have nothing to restore.
    fn restore_tab(&mut self, store: &StateStore, record: &TabRecord) -> Option<DocumentTab> {
        let key = uuid::Uuid::parse_str(&record.key).ok();
        let file = record
            .file
            .clone()
            .map(|(path, digest)| TrackedFile { path, baseline: digest.and_then(|d| Fingerprint::from_hex(&d)) });
        let mut tab = None;
        if let (true, Some(key)) = (record.dirty, key) {
            let doc = self.read_valid(store, &storage::tab_recovery_key(&key), |t| parse_document(t).map_err(|e| e.0));
            tab = doc.map(|doc| {
                let mut core = EditorCore::new(doc);
                core.dirty = true;
                DocumentTab::new(core, file.clone(), None)
            });
        }
        if tab.is_none() {
            let path = file.map(|f| f.path).filter(|p| p.exists())?;
            match read_project(&path) {
                Ok((doc, print)) => {
                    tab = Some(DocumentTab::clean(
                        EditorCore::new(doc),
                        Some(TrackedFile { path, baseline: Some(print) }),
                    ))
                }
                Err(e) => {
                    log::warn!("Could not reopen {}: {e}", path.display());
                    return None;
                }
            }
        }
        let mut tab = tab?;
        if let Some(key) = key {
            tab.key = key;
        }
        tab.view = record.view;
        if let Some(meta) = record.trace.clone() {
            match self.load_trace(meta) {
                Ok(trace) => tab.trace = Some(trace),
                Err(e) => log::warn!("Could not restore a tracing guide: {e}"),
            }
        }
        Some(tab)
    }

    /// Restores the last session: every open tab (with its unsaved changes),
    /// or for state from a single-document version the recovery record and
    /// then the last file; the starter document on first run. A project
    /// passed on the command line opens in a tab of its own.
    fn restore(&mut self, open: Option<PathBuf>) {
        let Some(store) = self.store.clone() else {
            self.toasts
                .push(ToastKind::Error, "Device storage is unavailable. Changes will not be recovered after a crash.");
            if let Some(path) = open {
                self.open_path(path);
            }
            return;
        };
        if let Ok(Some(text)) = store.read("app") {
            match serde_json::from_str::<AppState>(&text) {
                Ok(state) => self.app_state = state,
                Err(_) => {
                    let _ = store.quarantine("app");
                }
            }
        }
        let state = self.app_state.clone();
        self.recent = state.recent.iter().filter(|p| p.exists()).cloned().collect();
        self.layout = state.layout;
        self.show_grid = state.show_grid;
        self.export = state.export;
        self.agents.set_enabled(state.agents);

        if let Some(presets) = self.read_valid(&store, "presets", |t| parse_presets(t).map_err(|e| e.0)) {
            self.core_mut().presets = presets;
        }
        if let Some(preferences) = self.read_valid(&store, "preferences", |t| parse_preferences(t).map_err(|e| e.0)) {
            self.core_mut().preferences = preferences;
        }
        self.presets_saved = self.core().presets_generation;
        self.preferences_saved = self.core().preferences_generation;

        let mut restored = Vec::new();
        let mut active = 0;
        let mut legacy_trace = None;
        if !state.tabs.is_empty() {
            for (i, record) in state.tabs.iter().enumerate() {
                if let Some(tab) = self.restore_tab(&store, record) {
                    restored.push(tab);
                    if i <= state.active_tab {
                        active = restored.len() - 1;
                    }
                }
            }
        } else {
            // State from 1.x or a single-document Brickpress 2.
            legacy_trace = self.read_valid(&store, "trace-metadata", |t| {
                serde_json::from_str::<TraceMeta>(t).map_err(|e| e.to_string())
            });
            let recovered = self.read_valid(&store, "recovery", |t| parse_document(t).map_err(|e| e.0));
            if let Some(doc) = recovered {
                let file = state.recovery_file.clone().map(|(path, digest)| TrackedFile {
                    path,
                    baseline: digest.and_then(|d| Fingerprint::from_hex(&d)),
                });
                let mut core = EditorCore::new(doc);
                core.dirty = true;
                restored.push(DocumentTab::new(core, file, None));
            } else if let Some(path) = state.last_file.clone().filter(|p| p.exists()) {
                match read_project(&path) {
                    Ok((doc, print)) => restored.push(DocumentTab::clean(
                        EditorCore::new(doc),
                        Some(TrackedFile { path, baseline: Some(print) }),
                    )),
                    Err(e) => log::warn!("Could not reopen {}: {e}", path.display()),
                }
            }
            if let Some(tab) = restored.first_mut() {
                tab.view = state.view;
            }
        }
        let recovered = restored.iter().filter(|t| t.dirty()).count();
        if restored.is_empty() && state.first_run_done {
            restored.push(DocumentTab::clean(EditorCore::new(create_document(false)), None));
        }
        if !restored.is_empty() {
            // The starter tab holds the presets and preferences read above.
            let mut starter = std::mem::replace(&mut self.tabs, restored);
            self.tabs[active].core.adopt_shared(&mut starter[self.active].core);
            self.active = active;
        }
        if let Some(meta) = legacy_trace {
            match self.load_trace(meta) {
                Ok(trace) => self.tabs[self.active].trace = Some(trace),
                Err(e) => log::warn!("Could not restore the tracing guide: {e}"),
            }
        }
        if recovered > 0 {
            self.notify(if recovered == 1 {
                "Recovered your unsaved changes.".to_string()
            } else {
                format!("Recovered unsaved changes in {recovered} projects.")
            });
        }
        // Recovery records no tab refers to are left over from earlier sessions.
        let keep: Vec<String> = self.tabs.iter().map(|t| t.recovery_key()).collect();
        for key in store.tab_recovery_keys() {
            if !keep.contains(&key) {
                self.write_state(key, None);
            }
        }
        if let Some(path) = open {
            self.open_path(path);
        }
        self.persist_trace();
        self.sync_recovery();
    }

    // ---- frame ---------------------------------------------------------------

    fn poll_jobs(&mut self) {
        let mut finished = Vec::new();
        self.jobs.retain(|rx| match rx.try_recv() {
            Ok(result) => {
                finished.push(result);
                false
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => true,
            Err(_) => false,
        });
        for result in finished {
            self.busy = self.busy.saturating_sub(1);
            match result {
                JobResult::Done(text) => self.toasts.push(ToastKind::Success, text),
                JobResult::Failed(text) => self.error(text),
            }
        }
    }

    pub fn spawn_job(&mut self, work: impl FnOnce() -> JobResult + Send + 'static, ctx: &egui::Context) {
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        self.busy += 1;
        std::thread::spawn(move || {
            let _ = tx.send(work());
            ctx.request_repaint();
        });
        self.jobs.push(rx);
    }

    fn update_title(&self, ctx: &egui::Context) {
        let title = match self.active_tab() {
            Some(tab) => {
                let marker = if tab.dirty() { " — Edited" } else { "" };
                format!("{}{marker} · Brickpress", tab.title())
            }
            None => "Brickpress".to_string(),
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    fn handle_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            if self.tabs.iter().any(|t| t.dirty()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                if self.modal.is_none() {
                    self.request(Pending::Quit);
                }
            } else {
                self.quitting = true;
            }
        }
        if self.quitting {
            // Every tab was saved or discarded. Anything still unsaved (an
            // agent edit that arrived meanwhile) is kept for recovery.
            for tab in &self.tabs {
                let value = tab.dirty().then(|| serialize_document(tab.core.doc()));
                self.write_state(tab.recovery_key(), value);
            }
            self.sync_recovery();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Applies agent operations received over the live connection.
    fn apply_agent_requests(&mut self, ctx: &egui::Context) {
        let requests = self.agents.drain();
        for request in requests {
            let reply = crate::agents::handle(self, request.op);
            let _ = request.reply.send(reply);
        }
        if self.agents.enabled() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

/// A document is clean when it matches what was last opened or saved, even
/// after undoing back to that state.
fn refresh_dirty(tab: &mut DocumentTab) {
    if let Some(saved) = &tab.saved_text {
        let core = &mut tab.core;
        if core.dirty && !core.live_active() && !core.placement_stroke_active() {
            core.dirty = &core.project_text() != saved;
        }
    }
}

/// Reads and validates a project file.
fn read_project(path: &Path) -> Result<(PressDocument, Fingerprint), String> {
    let (bytes, print) = storage::read_limited(path, storage::INPUT_LIMIT).map_err(|e| e.to_string())?;
    let text = String::from_utf8(bytes).map_err(|_| "This file is not a Brickpress project.".to_string())?;
    let doc = parse_document(&text).map_err(|e| e.0)?;
    Ok((doc, print))
}

impl eframe::App for BrickpressApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_jobs();
        self.apply_agent_requests(&ctx);
        handle_shortcuts(self, &ctx);
        if let Some(text) = self.core_mut().take_notice() {
            self.toasts.push(ToastKind::Info, text);
        }

        crate::topbar::show(self, ui);
        crate::statusbar::show(self, ui);
        if self.layout.library {
            crate::library::show(self, ui);
        }
        if self.layout.inspector {
            crate::inspector::show(self, ui);
        }
        let t = theme::tokens(&ctx);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t.workspace)).show(ui, |ui| {
            if self.has_document() {
                crate::tabs::strip(self, ui);
            }
            if !self.has_document() {
                crate::tabs::empty_state(self, ui);
                return;
            }
            let rest = ui.available_rect_before_wrap();
            ui.scope_builder(egui::UiBuilder::new().max_rect(rest), |ui| match self.view() {
                View::TwoD => crate::canvas::show(self, ui),
                View::ThreeD => crate::canvas3d::show(self, ui),
            });
        });
        crate::dialogs::show(self, &ctx);
        self.toasts.show(&ctx);

        // End slider and colour gestures once the pointer is released.
        if !ctx.input(|i| i.pointer.any_down()) {
            self.core_mut().end_live();
        }
        if let Some(text) = self.core_mut().take_notice() {
            self.toasts.push(ToastKind::Info, text);
        }
        self.autosave(&ctx);
        self.update_title(&ctx);
        self.handle_close(&ctx);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.persist_app_state();
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(dir: &Path) -> BrickpressApp {
        let mut app = BrickpressApp::headless(Arc::new(ShapeAtlas::build()), None, dir.to_path_buf());
        app.restore(None);
        app
    }

    /// Runs the debounced autosave now and waits for the writes to land.
    fn flush(app: &mut BrickpressApp, data: &Path) {
        app.autosave(&egui::Context::default()); // notices the edits
        for tab in &mut app.tabs {
            tab.autosave_due = Some(Instant::now() - Duration::from_secs(1));
        }
        app.autosave(&egui::Context::default());
        // Closing the channel lets the writer finish what is queued.
        app.state_writer = None;
        let store = StateStore::open(data.into()).unwrap();
        let expected = app.tabs.iter().filter(|t| t.dirty()).count();
        let started = Instant::now();
        while store.tab_recovery_keys().len() < expected && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    fn project(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(format!("{name}.brickpress.json"));
        let mut doc = create_document(false);
        doc.name = name.into();
        std::fs::write(&path, serialize_document(&doc)).unwrap();
        path
    }

    #[test]
    fn tabs_open_switch_share_presets_and_close() {
        let data = tempfile::tempdir().unwrap();
        let files = tempfile::tempdir().unwrap();
        let mut app = app(data.path());
        assert_eq!(app.tabs.len(), 1, "the starter document");
        assert!(app.tabs[0].pristine());
        let a = project(files.path(), "alpha");
        app.open_path(a.clone());
        assert_eq!(app.tabs.len(), 1, "an untouched starter makes way");
        assert_eq!(app.document_title(), "alpha.brickpress.json");
        app.open_path(project(files.path(), "beta"));
        assert_eq!((app.tabs.len(), app.active), (2, 1));
        app.open_path(a);
        assert_eq!((app.tabs.len(), app.active), (2, 0), "an open file is activated, not opened twice");

        app.core_mut().choose_piece("3070");
        app.core_mut().place(1, 1);
        app.core_mut().save_preset("Dot", false);
        assert!(app.dirty());
        app.activate(1);
        assert!(!app.dirty() && app.tabs[0].dirty(), "edits stay in their tab");
        assert_eq!(app.core().presets.len(), 1, "presets follow the active tab");
        assert_eq!(app.core().active_piece_id, "3070");

        let first = app.tabs[0].id;
        app.request(Pending::CloseTab(first));
        assert!(matches!(app.modal, Some(Modal::Unsaved(Pending::CloseTab(_)))));
        assert_eq!(app.active, 0, "the tab being asked about is shown");
        app.resolve_unsaved(Pending::CloseTab(first), false);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.core().presets.len(), 1);
        let last = app.tabs[0].id;
        app.request(Pending::CloseTab(last));
        assert!(!app.has_document());
        assert_eq!(app.core().presets.len(), 1, "presets survive with no document open");
        assert!(Command::Undo.availability(&app).is_err());
        assert!(Command::New.availability(&app).is_ok());
        let id = app.core().presets[0].id.clone();
        app.open_preset_document(&id);
        assert_eq!(app.tabs.len(), 1);
        assert!(app.dirty());
        assert_eq!(app.core().doc().name, "Dot");
        assert_eq!(app.core().doc().piece_count(), 1);
    }

    #[test]
    fn quitting_asks_about_each_unsaved_tab() {
        let data = tempfile::tempdir().unwrap();
        let mut app = app(data.path());
        app.core_mut().reseed();
        app.new_tab();
        app.new_tab();
        app.core_mut().reseed();
        app.activate(1);
        app.request(Pending::Quit);
        assert_eq!(app.active, 0, "the first unsaved tab is asked about first");
        assert!(matches!(app.modal, Some(Modal::Unsaved(Pending::Quit))));
        assert_eq!(app.other_dirty_tabs(), 1);
        app.resolve_unsaved(Pending::Quit, false);
        assert!(!app.quitting);
        assert_eq!(app.tabs.len(), 2);
        assert!(app.dirty(), "then the next one");
        app.modal = None; // Cancel
        assert!(!app.quitting, "cancelling keeps the window open");
        app.request(Pending::Quit);
        app.resolve_unsaved(Pending::Quit, false);
        assert!(app.quitting);
        assert_eq!(app.tabs.len(), 1, "the clean tab stays for the next session");
    }

    #[test]
    fn every_unsaved_tab_is_recovered_after_a_crash() {
        let data = tempfile::tempdir().unwrap();
        let files = tempfile::tempdir().unwrap();
        let mut app = app(data.path());
        app.open_path(project(files.path(), "saved"));
        app.core_mut().rename("Edited file");
        app.new_tab();
        app.core_mut().rename("Draft");
        app.new_tab(); // clean and untitled: nothing to restore
        app.activate(1);
        flush(&mut app, data.path());
        let store = StateStore::open(data.path().into()).unwrap();
        let mirror = store.read("recovery").unwrap().expect("the 1.x record");
        assert!(mirror.contains("Draft"), "the active unsaved tab is the 1.x record");
        drop(app); // crash

        let app = self::app(data.path());
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.tabs[0].core.doc().name, "Edited file");
        assert!(app.tabs[0].file.is_some(), "reattached to its file");
        assert_eq!(app.tabs[1].core.doc().name, "Draft");
        assert!(app.tabs.iter().all(|t| t.dirty()));
        assert_eq!(app.active, 1);
    }
}
