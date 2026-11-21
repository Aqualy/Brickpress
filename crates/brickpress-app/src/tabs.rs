//! Open documents. Each tab owns its editor (document, history, selection),
//! the file it belongs to, its tracing guide and its 2D and 3D views. The
//! window's shared state (presets, preferences, clipboard, the piece being
//! placed) lives in the active tab's editor and moves with activation.
//!
//! The strip of document tabs sits above the canvas, and the canvas area
//! shows an empty state when no document is open.

use crate::app::{BrickpressApp, Pending, TraceState, View};
use crate::canvas::CanvasState;
use crate::canvas3d::Canvas3DState;
use crate::commands::Command;
use crate::icons;
use crate::storage::{self, TrackedFile};
use crate::theme::{self, space, tokens};
use crate::widgets::{self, ButtonKind};
use brickpress_core::EditorCore;
use brickpress_render::three::camera::OrbitCamera;
use egui::{vec2, Align, Layout, RichText, Ui};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Matches the panel tabs beside it, so the lines under them meet.
pub const STRIP_HEIGHT: f32 = 36.0;

static NEXT_TAB: AtomicU64 = AtomicU64::new(1);

pub struct DocumentTab {
    /// Unique in this session; keys the renderer's caches and widget ids.
    pub id: u64,
    /// Stable across restarts; names this tab's recovery record.
    pub key: uuid::Uuid,
    pub core: EditorCore,
    pub file: Option<TrackedFile>,
    /// Serialization at the last open/save, for dirty tracking.
    pub saved_text: Option<String>,
    pub trace: Option<TraceState>,
    pub view: View,
    pub canvas: CanvasState,
    pub camera: OrbitCamera,
    pub canvas3d: Canvas3DState,
    pub(crate) autosave_generation: u64,
    pub(crate) autosave_due: Option<Instant>,
}

impl DocumentTab {
    /// A tab for `core`. `saved_text` is what counts as clean (`None`: unsaved).
    pub fn new(core: EditorCore, file: Option<TrackedFile>, saved_text: Option<String>) -> Self {
        Self {
            id: NEXT_TAB.fetch_add(1, Ordering::Relaxed),
            key: uuid::Uuid::new_v4(),
            autosave_generation: core.generation(),
            core,
            file,
            saved_text,
            trace: None,
            view: View::TwoD,
            canvas: CanvasState::default(),
            camera: OrbitCamera::default(),
            canvas3d: Canvas3DState::default(),
            autosave_due: None,
        }
    }

    /// A clean document: what it was when opened or saved.
    pub fn clean(core: EditorCore, file: Option<TrackedFile>) -> Self {
        let text = core.project_text();
        Self::new(core, file, Some(text))
    }

    pub fn title(&self) -> String {
        self.file.as_ref().map(|f| f.name()).unwrap_or_else(|| self.core.doc().name.clone())
    }

    pub fn dirty(&self) -> bool {
        self.core.dirty
    }

    /// Never saved and never touched: Open replaces it instead of adding a tab.
    pub fn pristine(&self) -> bool {
        self.file.is_none() && !self.core.dirty && !self.core.can_undo() && !self.core.can_redo()
    }

    pub fn recovery_key(&self) -> String {
        storage::tab_recovery_key(&self.key)
    }

    /// Where the file is, for tooltips.
    pub fn location(&self) -> String {
        match &self.file {
            Some(f) => f.path.display().to_string(),
            None => "Not saved yet".into(),
        }
    }
}

/// The document tabs above the canvas.
pub fn strip(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), STRIP_HEIGHT), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, t.panel);
    ui.painter().line_segment(
        [rect.left_bottom() - vec2(0.0, 0.5), rect.right_bottom() - vec2(0.0, 0.5)],
        egui::Stroke::new(1.0, t.divider),
    );
    let inner = rect.with_min_x(rect.left() + space::SM).with_min_y(rect.top() + 5.0);
    let mut activate = None;
    let mut close = None;
    let mut new = false;
    let active_id = app.active_tab().map(|t| t.id);
    let scroll_to = ui.data(|d| d.get_temp::<u64>(egui::Id::unique("tab-scrolled"))) != active_id;
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Max)), |ui| {
        ui.style_mut().always_scroll_the_only_direction = true;
        egui::ScrollArea::horizontal()
            .id_salt("document-tabs")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let mut previous: Option<(egui::Rect, bool)> = None;
                    for (index, tab) in app.tabs.iter().enumerate() {
                        let active = Some(tab.id) == active_id;
                        let r = widgets::document_tab(
                            ui,
                            egui::Id::unique(("document-tab", tab.id)),
                            &tab.title(),
                            tab.dirty(),
                            active,
                            &tab.location(),
                        );
                        if active && scroll_to {
                            r.response.scroll_to_me(None);
                        }
                        // Hairlines between inactive tabs.
                        let rect = r.response.rect;
                        if let Some((before, before_active)) = previous {
                            if !active && !before_active {
                                let x = ((before.right() + rect.left()) / 2.0).round() + 0.5;
                                ui.painter().line_segment(
                                    [egui::pos2(x, rect.top() + 9.0), egui::pos2(x, rect.bottom() - 9.0)],
                                    egui::Stroke::new(1.0, t.divider),
                                );
                            }
                        }
                        previous = Some((rect, active));
                        if r.close || r.response.middle_clicked() {
                            close = Some(tab.id);
                        } else if r.response.clicked() {
                            activate = Some(index);
                        }
                    }
                    ui.add_space(space::XS);
                    ui.allocate_ui_with_layout(
                        vec2(28.0, STRIP_HEIGHT - 5.0),
                        Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            let shortcut = Command::New.shortcut_text(ui.ctx());
                            let r = widgets::icon_button_sized(
                                ui,
                                icons::PLUS,
                                "New project",
                                shortcut.as_deref(),
                                false,
                                24.0,
                            );
                            new = r.clicked();
                        },
                    );
                });
            });
    });
    if scroll_to {
        let key = egui::Id::unique("tab-scrolled");
        ui.data_mut(|d| match active_id {
            Some(id) => {
                d.insert_temp(key, id);
            }
            None => d.remove::<u64>(key),
        });
    }
    if let Some(index) = activate {
        app.activate(index);
    }
    if let Some(id) = close {
        app.request(Pending::CloseTab(id));
    }
    if new {
        let ctx = ui.ctx().clone();
        app.run(Command::New, &ctx);
    }
}

/// The canvas area when no document is open.
pub fn empty_state(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let ctx = ui.ctx().clone();
    let area = ui.max_rect();
    let width = 420.0f32.min(area.width() - 2.0 * space::XL);
    let column = egui::Rect::from_center_size(area.center() - vec2(0.0, 40.0), vec2(width, 360.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(column).layout(Layout::top_down(Align::Center)), |ui| {
        ui.label(RichText::new(icons::FILE).font(egui::FontId::new(34.0, theme::icon_family())).color(t.text_disabled));
        ui.add_space(space::MD);
        ui.label(RichText::new("No project open").font(egui::FontId::new(15.0, theme::semibold())).color(t.text));
        ui.add_space(2.0);
        for line in ["Create a new project or open one.", "Presets in the library can open as projects too."] {
            ui.label(RichText::new(line).size(12.5).color(t.text_secondary));
        }
        ui.add_space(space::LG);
        ui.horizontal(|ui| {
            let buttons = 2.0 * 128.0 + space::SM;
            ui.add_space(((ui.available_width() - buttons) / 2.0).max(0.0));
            ui.spacing_mut().item_spacing.x = space::SM;
            let new =
                widgets::button_sized(ui, ButtonKind::Primary, Some(icons::FILE_PLUS), "New Project", Some(128.0));
            if widgets::tooltip(new, "New project", Command::New.shortcut_text(&ctx).as_deref()).clicked() {
                app.run(Command::New, &ctx);
            }
            let open = widgets::button_sized(ui, ButtonKind::Secondary, Some(icons::FOLDER_OPEN), "Open…", Some(128.0));
            if widgets::tooltip(open, "Open a project", Command::Open.shortcut_text(&ctx).as_deref()).clicked() {
                app.run(Command::Open, &ctx);
            }
        });
        if !app.recent.is_empty() {
            ui.add_space(space::XL);
            let list = width.min(340.0);
            ui.allocate_ui_with_layout(vec2(list, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(space::SM);
                    ui.label(
                        RichText::new("Recent")
                            .font(egui::FontId::new(11.5, theme::semibold()))
                            .color(t.text_secondary),
                    );
                });
            });
            ui.add_space(space::XS);
            for path in app.recent.clone().into_iter().take(6) {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let folder = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let r = widgets::list_row(ui, icons::HISTORY, &name, &folder, list);
                if r.on_hover_text(path.display().to_string()).clicked() {
                    app.open_path(path);
                }
            }
        }
    });
}
