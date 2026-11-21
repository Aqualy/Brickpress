//! The 2D board: rendering through the GPU callback and all pointer and
//! keyboard interaction (select, move, marquee, place strokes, pan, zoom).

use crate::app::BrickpressApp;
use crate::commands::Command;
use crate::gpu::Board2DCallback;
use crate::icons;
use crate::tabs::DocumentTab;
use crate::theme::{self, space, tokens, RADIUS};
use crate::widgets;
use brickpress_core::catalog::get_piece;
use brickpress_core::geometry::{bounds, footprint, Rect as StudRect};
use brickpress_core::placement_path::placement_path;
use brickpress_core::{EditorCore, Id, Mode, Tool};
use brickpress_render::atlas::ShapeAtlas;
use brickpress_render::board2d::{FrameOptions, View2D};
use brickpress_render::scene::{
    board_params, build_ghosts, build_outlines, build_pieces, hit_test, Instance, Overlay, RenderMode,
};
use egui::{
    pos2, vec2, Color32, CornerRadius, CursorIcon, Key, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub const STUD_POINTS: f32 = 32.0;
const MIN_SCALE: f32 = 0.05;
const MAX_SCALE: f32 = 16.0;

#[derive(Clone, Debug, Default)]
enum Drag {
    #[default]
    None,
    Pan,
    Move {
        origin: (f64, f64),
        offset: (i32, i32),
        valid: bool,
    },
    Marquee {
        start: (f64, f64),
        end: (f64, f64),
        additive: bool,
    },
    Paint {
        last: (f64, f64),
    },
}

pub struct CanvasState {
    /// `None` fits the board; otherwise 1.0 = 32 points per stud.
    zoom: Option<f32>,
    scale: f32,
    pan: Vec2,
    viewport: Rect,
    drag: Drag,
    space_held: bool,
    pub cursor_cell: Option<(i32, i32)>,
    pub placement_problem: Option<String>,
    keyboard_cursor: Option<(i32, i32)>,
    cache_key: u64,
    pieces: Arc<Vec<Instance>>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: None,
            scale: 1.0,
            pan: Vec2::ZERO,
            viewport: Rect::NOTHING,
            drag: Drag::None,
            space_held: false,
            cursor_cell: None,
            placement_problem: None,
            keyboard_cursor: None,
            cache_key: u64::MAX,
            pieces: Arc::new(Vec::new()),
        }
    }
}

impl CanvasState {
    pub fn fit(&mut self) {
        self.zoom = None;
        self.pan = Vec2::ZERO;
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.zoom = Some(scale.clamp(MIN_SCALE, MAX_SCALE));
    }

    pub fn zoom_percent(&self) -> i32 {
        (self.scale * 100.0).round() as i32
    }

    fn fit_scale(&self, board: (f32, f32)) -> f32 {
        let available = self.viewport.size() - vec2(112.0, 140.0);
        (available.x / (board.0 * STUD_POINTS)).min(available.y / (board.1 * STUD_POINTS)).clamp(MIN_SCALE, MAX_SCALE)
    }

    fn origin(&self, board: (f32, f32)) -> Pos2 {
        let size = vec2(board.0, board.1) * STUD_POINTS * self.scale;
        self.viewport.center() + self.pan - size / 2.0 - vec2(0.0, 20.0)
    }

    /// Zooms by `factor`, keeping `anchor` (screen point) fixed.
    pub fn zoom_by(&mut self, factor: f32, anchor: Option<Pos2>) {
        let anchor = anchor.unwrap_or(self.viewport.center());
        let before = self.scale;
        let after = (before * factor).clamp(MIN_SCALE, MAX_SCALE);
        // The board centre moves with the anchor: c' = a + (c - a) * after / before.
        let center = self.viewport.center() + self.pan - vec2(0.0, 20.0);
        let next_center = anchor + (center - anchor) * (after / before);
        self.pan = next_center - self.viewport.center() + vec2(0.0, 20.0);
        self.zoom = Some(after);
        self.scale = after;
    }

    fn to_board(&self, p: Pos2, board: (f32, f32)) -> (f64, f64) {
        let o = self.origin(board);
        let s = (STUD_POINTS * self.scale) as f64;
        (((p.x - o.x) as f64) / s, ((p.y - o.y) as f64) / s)
    }

    fn to_screen(&self, x: f64, y: f64, board: (f32, f32)) -> Pos2 {
        let o = self.origin(board);
        let s = STUD_POINTS * self.scale;
        pos2(o.x + x as f32 * s, o.y + y as f32 * s)
    }

    fn stud_rect(&self, r: StudRect, board: (f32, f32)) -> Rect {
        Rect::from_min_max(
            self.to_screen(r.x as f64, r.y as f64, board),
            self.to_screen(r.right() as f64, r.bottom() as f64, board),
        )
    }
}

/// The anchor for placement: the cursor cell, shifted so larger pieces are
/// centred on the pointer.
fn anchor(core: &EditorCore, cell: (i32, i32)) -> (i32, i32) {
    if let Some(preset) = core.active_preset() {
        let swapped = core.preset_rotation % 180 != 0;
        let (w, h) = if swapped { (preset.height, preset.width) } else { (preset.width, preset.height) };
        return (cell.0 - (w - 1) / 2, cell.1 - (h - 1) / 2);
    }
    match get_piece(&core.active_piece_id) {
        Some(piece) => {
            let size = footprint(piece, core.placement_rotation);
            (cell.0 - (size.width - 1) / 2, cell.1 - (size.height - 1) / 2)
        }
        None => cell,
    }
}

fn cell_of(p: (f64, f64)) -> (i32, i32) {
    (p.0.floor() as i32, p.1.floor() as i32)
}

pub fn show(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let viewport = ui.max_rect();
    let atlas = app.atlas.clone();
    let (show_grid, modal) = (app.show_grid, app.modal_open());
    let tab = &mut app.tabs[app.active];
    tab.canvas.viewport = viewport;
    let doc_board = tab.core.doc().board;
    let board = (doc_board.width as f32, doc_board.height as f32);
    if tab.canvas.zoom.is_none() {
        tab.canvas.scale = tab.canvas.fit_scale(board);
    } else {
        tab.canvas.scale = tab.canvas.zoom.unwrap_or(1.0);
    }

    let response = ui.allocate_rect(viewport, Sense::click_and_drag());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Canvas,
            true,
            format!("{}, {} by {} studs", tab.core.doc().name, doc_board.width, doc_board.height),
        )
    });
    let pointer = response.hover_pos();
    let space = ui.input(|i| i.key_down(Key::Space)) && !ui.ctx().text_edit_focused();
    tab.canvas.space_held = space;

    handle_scroll(tab, ui, &response);
    let board_point = pointer.map(|p| tab.canvas.to_board(p, board));
    tab.canvas.cursor_cell =
        board_point.filter(|p| p.0 >= 0.0 && p.1 >= 0.0 && p.0 < board.0 as f64 && p.1 < board.1 as f64).map(cell_of);

    let notice = handle_pointer(tab, &atlas, ui, &response, board_point);
    if response.has_focus() || !ui.ctx().text_edit_focused() {
        handle_keys(tab, &atlas, ui, board, modal);
    }
    if response.clicked() || response.drag_started() {
        response.request_focus();
    }

    // ---- paint -------------------------------------------------------------
    let painter = ui.painter_at(viewport);
    let board_rect =
        tab.canvas.stud_rect(StudRect { x: 0, y: 0, width: doc_board.width, height: doc_board.height }, board);
    painter.add(
        egui::epaint::Shadow { offset: [0, 6], blur: 28, spread: 0, color: t.shadow }
            .as_shape(board_rect, CornerRadius::same(2)),
    );
    paint_board(tab, &atlas, show_grid, ui, viewport, board_rect);
    paint_overlays(tab, ui, &painter, board);
    if tab.core.doc().piece_count() == 0 && tab.core.tool != Tool::Place && tab.canvas.drag_is_none() {
        let hint = "Choose a piece in the library, then click or drag on the board.";
        painter.text(
            board_rect.center_bottom() + vec2(0.0, 22.0),
            egui::Align2::CENTER_TOP,
            hint,
            egui::FontId::proportional(12.5),
            t.text_secondary,
        );
    }
    if response.has_focus() {
        painter.rect_stroke(
            viewport.shrink(1.0),
            CornerRadius::ZERO,
            Stroke::new(2.0, t.focus.gamma_multiply(0.5)),
            StrokeKind::Inside,
        );
    }

    // Cursor.
    let cursor = match (&tab.canvas.drag, tab.core.tool) {
        (Drag::Pan, _) => CursorIcon::Grabbing,
        (_, Tool::Hand) => CursorIcon::Grab,
        _ if space => CursorIcon::Grab,
        (Drag::Move { .. }, _) => CursorIcon::Grabbing,
        (_, Tool::Place) => CursorIcon::Crosshair,
        _ => CursorIcon::Default,
    };
    if response.hovered() {
        ui.ctx().set_cursor_icon(cursor);
    }
    if let Some(text) = notice {
        app.notify(text);
    }
    // Context menu on pieces or the empty board.
    response.context_menu(|ui| context_menu(app, ui, board_point));
    floating_toolbar(app, ui, viewport);
}

impl CanvasState {
    fn drag_is_none(&self) -> bool {
        matches!(self.drag, Drag::None)
    }
}

fn handle_scroll(tab: &mut DocumentTab, ui: &Ui, response: &egui::Response) {
    if !response.hovered() {
        return;
    }
    let (scroll, zoom, command) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers.command));
    let pointer = response.hover_pos();
    if zoom != 1.0 {
        tab.canvas.zoom_by(zoom, pointer);
    } else if scroll != Vec2::ZERO {
        if command {
            tab.canvas.zoom_by((scroll.y / 200.0).exp(), pointer);
        } else {
            if tab.canvas.zoom.is_none() {
                tab.canvas.zoom = Some(tab.canvas.scale);
            }
            tab.canvas.pan += scroll;
        }
    }
}

/// Pointer gestures on the board. Returns a message for the person, if any.
fn handle_pointer(
    tab: &mut DocumentTab,
    atlas: &ShapeAtlas,
    ui: &Ui,
    response: &egui::Response,
    point: Option<(f64, f64)>,
) -> Option<String> {
    let shift = ui.input(|i| i.modifiers.shift);
    let pan_gesture = tab.core.tool == Tool::Hand || tab.canvas.space_held || ui.input(|i| i.pointer.middle_down());

    // Middle-click (without dragging) picks a piece's shape and orientation.
    if response.middle_clicked() {
        if let Some(p) = point {
            if let Some((_, piece)) = hit_test(atlas, tab.core.doc(), p.0, p.1) {
                let uid = piece.uid.clone();
                tab.core.pick_piece(&uid);
            }
        }
        return None;
    }

    if response.drag_started() {
        let start = point?;
        if pan_gesture {
            tab.canvas.drag = Drag::Pan;
        } else if response.drag_started_by(egui::PointerButton::Primary) {
            match tab.core.tool {
                Tool::Place if tab.core.mode == Mode::Design => {
                    if tab.core.active_preset_id.is_none() && tab.core.begin_placement_stroke() {
                        let a = anchor(&tab.core, cell_of(start));
                        tab.core.paint_placement(a.0, a.1);
                        tab.canvas.drag = Drag::Paint { last: start };
                    }
                }
                _ => {
                    let hit = hit_test(atlas, tab.core.doc(), start.0, start.1)
                        .map(|(pass, p)| (pass.editable(), p.uid.clone()));
                    match hit {
                        Some((true, uid)) => {
                            if !tab.core.is_selected(&uid) {
                                tab.core.select(&uid, shift);
                            }
                            tab.canvas.drag = Drag::Move { origin: start, offset: (0, 0), valid: true };
                        }
                        _ => tab.canvas.drag = Drag::Marquee { start, end: start, additive: shift },
                    }
                }
            }
        }
    }

    if response.dragged() {
        match tab.canvas.drag.clone() {
            Drag::Pan => {
                if tab.canvas.zoom.is_none() {
                    tab.canvas.zoom = Some(tab.canvas.scale);
                }
                tab.canvas.pan += response.drag_delta();
            }
            Drag::Paint { last } => {
                if let Some(p) = point {
                    for cell in placement_path(last, p, tab.core.doc().board) {
                        let a = anchor(&tab.core, cell);
                        tab.core.paint_placement(a.0, a.1);
                    }
                    tab.canvas.drag = Drag::Paint { last: p };
                }
            }
            Drag::Move { origin, offset, .. } => {
                if let Some(p) = point {
                    let next = ((p.0 - origin.0).round() as i32, (p.1 - origin.1).round() as i32);
                    if next != offset {
                        let valid = tab.core.can_move_selected(next.0, next.1);
                        tab.canvas.drag = Drag::Move { origin, offset: next, valid };
                    }
                }
            }
            Drag::Marquee { start, additive, .. } => {
                if let Some(p) = point {
                    tab.canvas.drag = Drag::Marquee { start, end: p, additive };
                }
            }
            Drag::None => {}
        }
        if response.dragged_by(egui::PointerButton::Secondary) && tab.core.placement_stroke_active() {
            tab.core.finish_placement_stroke(true);
            tab.canvas.drag = Drag::None;
        }
    }

    if response.drag_stopped() {
        match std::mem::take(&mut tab.canvas.drag) {
            Drag::Paint { .. } => tab.core.finish_placement_stroke(false),
            Drag::Move { offset, valid, .. } => {
                if offset != (0, 0) {
                    if valid {
                        tab.core.move_selected(offset.0, offset.1);
                    } else {
                        return Some("There is no room there. The pieces stayed where they were.".into());
                    }
                }
            }
            Drag::Marquee { start, end, additive } => {
                let (x0, x1) = (start.0.min(end.0), start.0.max(end.0));
                let (y0, y1) = (start.1.min(end.1), start.1.max(end.1));
                let hits: Vec<Id> = tab
                    .core
                    .doc()
                    .all_pieces()
                    .filter(|(pass, _)| pass.editable())
                    .filter(|(_, p)| {
                        let b = bounds(p);
                        (b.x as f64) < x1 && (b.right() as f64) > x0 && (b.y as f64) < y1 && (b.bottom() as f64) > y0
                    })
                    .map(|(_, p)| p.uid.clone())
                    .collect();
                tab.core.select_many(hits, additive);
            }
            _ => {}
        }
    }

    if response.clicked() {
        let p = point?;
        if pan_gesture {
            return None;
        }
        match tab.core.tool {
            Tool::Place if tab.core.mode == Mode::Design => {
                let a = anchor(&tab.core, cell_of(p));
                tab.core.place(a.0, a.1);
            }
            _ => match hit_test(atlas, tab.core.doc(), p.0, p.1)
                .map(|(pass, piece)| (pass.editable(), pass.id.clone(), piece.uid.clone()))
            {
                Some((true, _, uid)) => tab.core.select(&uid, shift),
                Some((false, _, _)) => {
                    return Some("That piece is on a locked or hidden pass. Unlock it in Inks to edit it.".into());
                }
                None if !shift => tab.core.clear_selection(),
                None => {}
            },
        }
    }
    if response.double_clicked() && tab.core.tool == Tool::Select {
        if let Some(p) = point {
            if let Some((pass, _)) = hit_test(atlas, tab.core.doc(), p.0, p.1) {
                let ids: Vec<Id> = pass.pieces.iter().map(|x| x.uid.clone()).collect();
                tab.core.select_many(ids, false);
            }
        }
    }

    // Context menu on pieces or the empty board.
    if response.secondary_clicked() && !tab.core.placement_stroke_active() {
        if let Some(p) = point {
            match hit_test(atlas, tab.core.doc(), p.0, p.1).map(|(_, x)| x.uid.clone()) {
                Some(uid) if !tab.core.is_selected(&uid) => tab.core.select(&uid, false),
                _ => {}
            }
        }
    }
    None
}

fn context_menu(app: &mut BrickpressApp, ui: &mut Ui, point: Option<(f64, f64)>) {
    use crate::topbar::menu_item;
    if !app.core().selected_pieces().is_empty() {
        for command in [Command::Rotate, Command::MirrorHorizontal, Command::MirrorVertical] {
            menu_item(ui, app, command);
        }
        ui.separator();
        for command in [Command::Cut, Command::Copy, Command::Duplicate, Command::Delete] {
            menu_item(ui, app, command);
        }
        ui.separator();
        menu_item(ui, app, Command::BringForward);
        menu_item(ui, app, Command::SendBackward);
        ui.menu_button(format!("{}  Move to Ink Pass", icons::LAYERS), |ui| {
            for pass in app.core().doc().passes.clone() {
                let r = ui.add_enabled(
                    pass.editable(),
                    egui::Button::new(
                        egui::RichText::new(format!("● {}", pass.name)).color(theme::color32(&pass.color)),
                    ),
                );
                if r.clicked() {
                    app.core_mut().move_to_pass(&pass.id);
                    ui.close();
                }
            }
        });
        ui.separator();
        menu_item(ui, app, Command::SavePreset);
        if let Some(p) = point {
            if let Some(uid) = hit_test(&app.atlas, app.core().doc(), p.0, p.1).map(|(_, x)| x.uid.clone()) {
                if ui.button(format!("{}  Pick Shape", icons::PIPETTE)).clicked() {
                    app.core_mut().pick_piece(&uid);
                    ui.close();
                }
            }
        }
    } else {
        menu_item(ui, app, Command::Paste);
        menu_item(ui, app, Command::SelectAll);
        ui.separator();
        menu_item(ui, app, Command::AddTracing);
        menu_item(ui, app, Command::ZoomFit);
    }
}

fn handle_keys(tab: &mut DocumentTab, atlas: &ShapeAtlas, ui: &Ui, board: (f32, f32), modal: bool) {
    if ui.ctx().text_edit_focused() || modal {
        return;
    }
    let (left, right, up, down, shift, enter, pick) = ui.input(|i| {
        (
            i.key_pressed(Key::ArrowLeft),
            i.key_pressed(Key::ArrowRight),
            i.key_pressed(Key::ArrowUp),
            i.key_pressed(Key::ArrowDown),
            i.modifiers.shift,
            i.key_pressed(Key::Enter),
            i.key_pressed(Key::I),
        )
    });
    let step = if shift { 4 } else { 1 };
    let dx = (right as i32 - left as i32) * step;
    let dy = (down as i32 - up as i32) * step;
    match tab.core.tool {
        Tool::Hand => {
            if dx != 0 || dy != 0 {
                if tab.canvas.zoom.is_none() {
                    tab.canvas.zoom = Some(tab.canvas.scale);
                }
                tab.canvas.pan -= vec2(dx as f32, dy as f32) * STUD_POINTS * tab.canvas.scale;
            }
        }
        Tool::Place => {
            let (w, h) = (board.0 as i32, board.1 as i32);
            let cursor = tab.canvas.keyboard_cursor.get_or_insert((w / 2, h / 2));
            if dx != 0 || dy != 0 {
                *cursor = ((cursor.0 + dx).clamp(0, w - 1), (cursor.1 + dy).clamp(0, h - 1));
            }
            let cell = *cursor;
            if enter {
                let a = anchor(&tab.core, cell);
                tab.core.place(a.0, a.1);
            }
            if pick {
                if let Some((_, p)) = hit_test(atlas, tab.core.doc(), cell.0 as f64 + 0.5, cell.1 as f64 + 0.5) {
                    let uid = p.uid.clone();
                    tab.core.pick_piece(&uid);
                }
            }
        }
        Tool::Select => {
            if (dx != 0 || dy != 0) && !tab.core.selected_pieces().is_empty() {
                tab.core.move_selected(dx, dy);
            }
        }
    }
    if tab.core.tool != Tool::Place {
        tab.canvas.keyboard_cursor = None;
    }
}

fn scene_key(tab: &DocumentTab, drag: (i32, i32)) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    // The renderer caches one scene; another tab must never match it.
    tab.id.hash(&mut h);
    tab.core.generation().hash(&mut h);
    (tab.core.mode == Mode::Print).hash(&mut h);
    drag.hash(&mut h);
    if drag != (0, 0) {
        tab.core.selected.hash(&mut h);
    }
    h.finish()
}

fn paint_board(tab: &mut DocumentTab, atlas: &ShapeAtlas, show_grid: bool, ui: &Ui, viewport: Rect, board_rect: Rect) {
    let t = tokens(ui.ctx());
    let ppp = ui.ctx().pixels_per_point();
    let mode = if tab.core.mode == Mode::Print { RenderMode::Print } else { RenderMode::Design };
    let drag = match tab.canvas.drag {
        Drag::Move { offset, .. } => offset,
        _ => (0, 0),
    };
    let selected: HashSet<&str> = tab.core.selected.iter().map(|s| &**s).collect();
    let key = scene_key(tab, drag);
    if key != tab.canvas.cache_key {
        let overlay = Overlay { selected: Some(&selected), drag, drag_valid: true };
        tab.canvas.pieces = Arc::new(build_pieces(atlas, tab.core.doc(), mode, &overlay));
        tab.canvas.cache_key = key;
    }
    // Overlays: outlines and placement ghosts, rebuilt every frame (small).
    let drag_valid = matches!(
        tab.canvas.drag,
        Drag::Move { valid: true, .. } | Drag::None | Drag::Pan | Drag::Paint { .. } | Drag::Marquee { .. }
    );
    let accent = [t.accent_text.r() as f32 / 255.0, t.accent_text.g() as f32 / 255.0, t.accent_text.b() as f32 / 255.0];
    let invalid = [t.danger.r() as f32 / 255.0, t.danger.g() as f32 / 255.0, t.danger.b() as f32 / 255.0];
    let accent = if mode == RenderMode::Design { [0.0, 0.341, 0.847] } else { accent };
    let overlay_state = Overlay { selected: Some(&selected), drag, drag_valid };
    let mut overlay = if mode == RenderMode::Design && selected.len() <= 4000 {
        build_outlines(atlas, tab.core.doc(), &overlay_state, accent, invalid)
    } else {
        Vec::new()
    };
    tab.canvas.placement_problem = None;
    let ghost_cell = tab.canvas.cursor_cell.or(tab.canvas.keyboard_cursor);
    if tab.core.tool == Tool::Place && tab.core.mode == Mode::Design && !tab.core.placement_stroke_active() {
        if let Some(cell) = ghost_cell {
            let a = anchor(&tab.core, cell);
            let ghosts: Vec<(brickpress_core::PlacedPiece, [f32; 3])> = if tab.core.active_preset().is_some() {
                tab.core
                    .preset_preview(a.0, a.1)
                    .into_iter()
                    .map(|row| (row.piece, brickpress_render::scene::srgb(&row.color)))
                    .collect()
            } else {
                let color = brickpress_render::scene::srgb(&tab.core.active_pass().color);
                tab.core.ghosts(a.0, a.1).into_iter().map(|g| (g, color)).collect()
            };
            let problem = tab.core.placement_problem(a.0, a.1);
            overlay.extend(build_ghosts(atlas, &ghosts, problem.is_none(), invalid));
            tab.canvas.placement_problem = problem;
        }
    }
    let doc = tab.core.doc();
    let show_grid = show_grid && doc.options.grid != brickpress_core::GridStyle::Off && mode == RenderMode::Design;
    let params = board_params(doc, mode, true, show_grid, tab.core.preferences.grid_appearance);
    let options = FrameOptions { grid_color: [0.455, 0.494, 0.553, 1.0], grid: show_grid, paper: true };
    let origin = (board_rect.min - viewport.min) * ppp;
    let view = View2D {
        origin: [origin.x, origin.y],
        stud_px: STUD_POINTS * tab.canvas.scale * ppp,
        target: [viewport.width() * ppp, viewport.height() * ppp],
    };
    let trace = tab.trace.as_ref().filter(|tr| tr.meta.visible && mode == RenderMode::Design).map(|tr| {
        (
            tr.upload.clone(),
            [tr.meta.x as f32, tr.meta.y as f32, tr.meta.width as f32, tr.meta.height as f32],
            tr.meta.opacity as f32,
        )
    });
    let callback =
        Board2DCallback { view, params, options, scene_key: key, pieces: tab.canvas.pieces.clone(), overlay, trace };
    ui.painter().add(egui_wgpu::Callback::new_paint_callback(viewport, callback));
}

fn paint_overlays(tab: &DocumentTab, ui: &Ui, painter: &egui::Painter, board: (f32, f32)) {
    let t = tokens(ui.ctx());
    let doc = tab.core.doc();
    let board_rect =
        tab.canvas.stud_rect(StudRect { x: 0, y: 0, width: doc.board.width, height: doc.board.height }, board);
    // Symmetry axes.
    if tab.core.mode == Mode::Design {
        let dash = |a: Pos2, b: Pos2| {
            painter.extend(Shape::dashed_line(&[a, b], Stroke::new(1.0, t.accent_text.gamma_multiply(0.7)), 6.0, 5.0));
        };
        if doc.options.symmetry.mirrors_x() {
            dash(board_rect.center_top() - vec2(0.0, 8.0), board_rect.center_bottom() + vec2(0.0, 8.0));
        }
        if doc.options.symmetry.mirrors_y() {
            dash(board_rect.left_center() - vec2(8.0, 0.0), board_rect.right_center() + vec2(8.0, 0.0));
        }
    }
    // Marquee.
    if let Drag::Marquee { start, end, .. } = tab.canvas.drag {
        let r = Rect::from_two_pos(
            tab.canvas.to_screen(start.0, start.1, board),
            tab.canvas.to_screen(end.0, end.1, board),
        );
        painter.rect(
            r,
            CornerRadius::ZERO,
            t.accent.gamma_multiply(0.12),
            Stroke::new(1.0, t.accent_text),
            StrokeKind::Inside,
        );
    }
    // Keyboard placement cursor.
    if let (Tool::Place, Some((x, y))) = (tab.core.tool, tab.canvas.keyboard_cursor) {
        let r = tab.canvas.stud_rect(StudRect { x, y, width: 1, height: 1 }, board);
        painter.rect_stroke(r, CornerRadius::same(2), Stroke::new(2.0, t.focus), StrokeKind::Outside);
    }
    // Hover cell in select mode, faint.
    if let (Tool::Select, Some((x, y)), Drag::None) = (tab.core.tool, tab.canvas.cursor_cell, &tab.canvas.drag) {
        if tab.canvas.scale > 0.6 {
            let r = tab.canvas.stud_rect(StudRect { x, y, width: 1, height: 1 }, board);
            painter.rect_stroke(
                r,
                CornerRadius::same(2),
                Stroke::new(1.0, Color32::from_black_alpha(40)),
                StrokeKind::Inside,
            );
        }
    }
    // Board size badge in the top-left corner.
    let label = format!("{} × {}", doc.board.width, doc.board.height);
    painter.text(
        board_rect.left_top() - vec2(0.0, 8.0),
        egui::Align2::LEFT_BOTTOM,
        label,
        egui::FontId::monospace(11.0),
        t.text_secondary,
    );
}

fn floating_toolbar(app: &mut BrickpressApp, ui: &mut Ui, viewport: Rect) {
    let t = tokens(ui.ctx());
    let width = 404.0;
    let rect = Rect::from_center_size(pos2(viewport.center().x, viewport.bottom() - 34.0), vec2(width, 44.0));
    let mut child =
        ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
    egui::Frame::new()
        .fill(t.surface)
        .stroke(Stroke::new(1.0, t.divider))
        .corner_radius(CornerRadius::same(RADIUS + 6))
        .shadow(egui::Shadow { offset: [0, 4], blur: 16, spread: 0, color: t.shadow })
        .inner_margin(egui::Margin::symmetric(6, 6))
        .show(&mut child, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for command in [Command::SelectTool, Command::PlaceTool, Command::HandTool] {
                tool(app, ui, command);
            }
            separator(ui);
            for command in [Command::Rotate, Command::MirrorHorizontal, Command::MirrorVertical] {
                tool(app, ui, command);
            }
            separator(ui);
            tool(app, ui, Command::ZoomOut);
            let label = format!("{}%", app.tab().canvas.zoom_percent());
            let r = ui.add_sized(
                vec2(52.0, 28.0),
                egui::Button::new(egui::RichText::new(label).font(egui::FontId::monospace(12.0))).frame(false),
            );
            if widgets::tooltip(r, "Actual size", Command::ZoomActual.shortcut_text(ui.ctx()).as_deref()).clicked() {
                app.tab_mut().canvas.set_scale(1.0);
            }
            tool(app, ui, Command::ZoomIn);
            tool(app, ui, Command::ZoomFit);
        });
    let _ = space::SM;
}

fn separator(ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(9.0, 20.0), Sense::hover());
    ui.painter().line_segment([rect.center_top(), rect.center_bottom()], Stroke::new(1.0, t.divider));
}

fn tool(app: &mut BrickpressApp, ui: &mut Ui, command: Command) {
    let meta = command.meta();
    let checked = command.checked(app).unwrap_or(false);
    let ok = command.availability(app);
    let shortcut = command.shortcut_text(ui.ctx());
    let r = ui
        .add_enabled_ui(ok.is_ok(), |ui| widgets::icon_button(ui, meta.icon, meta.label, shortcut.as_deref(), checked))
        .inner;
    if let Err(reason) = ok {
        r.clone().on_disabled_hover_text(reason);
    }
    if r.clicked() {
        let ctx = ui.ctx().clone();
        app.run(command, &ctx);
    }
}
