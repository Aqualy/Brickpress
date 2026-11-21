//! The 3D view: orbit, pan and zoom the build, and place or select pieces
//! with the same rules as the 2D board.

use crate::app::BrickpressApp;
use crate::commands::Command;
use crate::gpu::Board3DCallback;
use crate::icons;
use crate::theme::{tokens, RADIUS};
use crate::widgets;
use brickpress_core::catalog::get_piece;
use brickpress_core::geometry::footprint;
use brickpress_core::{EditorCore, Id, PressDocument, Tool};
use brickpress_render::atlas::ShapeAtlas;
use brickpress_render::scene::hit_test;
use brickpress_render::three::camera::{CameraAction, CameraFrame, OrbitCamera, Projection, Zoom};
use brickpress_render::three::mesh::{piece_height, PLATE_HEIGHT};
use brickpress_render::three::Scene3D;
use egui::{pos2, vec2, CornerRadius, Rect, Sense, Stroke, Ui};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Default)]
pub struct Canvas3DState {
    frame: Option<CameraFrame>,
    hover: Option<(i32, i32)>,
    key: u64,
    scene: Option<Arc<Scene3D>>,
    orbiting: bool,
}

impl Canvas3DState {
    pub fn zoom_by(&mut self, camera: &mut OrbitCamera, factor: f32) {
        camera.zoom = Zoom::Scale((camera.scale * factor).clamp(0.03, 6.0));
    }
}

/// Board cell under a pixel: the top surface of a piece (when the ray hits
/// one) or the plate.
fn pick(
    atlas: &ShapeAtlas,
    doc: &PressDocument,
    frame: &CameraFrame,
    p: egui::Pos2,
    size: Rect,
) -> (Option<(i32, i32)>, Option<Id>) {
    let (w, h) = (doc.board.width as f32, doc.board.height as f32);
    let local = p - size.min;
    let to_board = |world: glam::Vec3| ((world.x + w / 2.0) as f64, (world.z + h / 2.0) as f64);
    for height in [2.0 * PLATE_HEIGHT, PLATE_HEIGHT] {
        if let Some(world) = frame.intersect_plane(local.x, local.y, size.width(), size.height(), height - 0.012) {
            let (bx, by) = to_board(world);
            if let Some((_, piece)) = hit_test(atlas, doc, bx, by) {
                let ph =
                    get_piece(&piece.piece_id).map(|x| piece_height(x.surface_height_plates)).unwrap_or(PLATE_HEIGHT);
                if (ph - height).abs() < 0.01 {
                    return (Some((bx.floor() as i32, by.floor() as i32)), Some(piece.uid.clone()));
                }
            }
        }
    }
    match frame.intersect_plane(local.x, local.y, size.width(), size.height(), 0.0) {
        Some(world) => {
            let (bx, by) = to_board(world);
            let inside = bx >= 0.0 && by >= 0.0 && bx < w as f64 && by < h as f64;
            let hit = hit_test(atlas, doc, bx, by).map(|(_, p)| p.uid.clone());
            (inside.then(|| (bx.floor() as i32, by.floor() as i32)), hit)
        }
        None => (None, None),
    }
}

pub fn show(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let viewport = ui.max_rect();
    let ppp = ui.ctx().pixels_per_point();
    let response = ui.allocate_rect(viewport, Sense::click_and_drag());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Canvas,
            true,
            "3D view of the board. Drag to orbit, right-drag to pan, scroll to zoom.",
        )
    });
    let atlas = app.atlas.clone();
    let tab = &mut app.tabs[app.active];
    let board = (tab.core.doc().board.width as f32, tab.core.doc().board.height as f32);
    let frame = tab.camera.frame(board, viewport.width(), viewport.height(), (96.0, 140.0));

    // Camera controls.
    let (scroll, zoom, shift) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers.shift));
    if response.hovered() {
        if zoom != 1.0 {
            tab.canvas3d.zoom_by(&mut tab.camera, zoom);
        } else if scroll.y != 0.0 {
            tab.canvas3d.zoom_by(&mut tab.camera, (scroll.y / 240.0).exp());
        }
    }
    let placing = tab.core.tool == Tool::Place;
    if response.drag_started() {
        tab.canvas3d.orbiting = true;
    }
    if response.dragged() {
        let d = response.drag_delta();
        let pan = response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
            || shift
            || tab.core.tool == Tool::Hand;
        if pan {
            tab.camera.pan(&frame, d.x, d.y);
        } else {
            tab.camera.orbit(d.x, d.y);
        }
    }
    if response.drag_stopped() {
        tab.canvas3d.orbiting = false;
    }

    // Picking and placement.
    let pointer = response.hover_pos();
    let (cell, hit) = pointer.map(|p| pick(&atlas, tab.core.doc(), &frame, p, viewport)).unwrap_or((None, None));
    tab.canvas3d.hover = cell;
    tab.canvas.cursor_cell = cell;
    let anchor = |core: &EditorCore, c: (i32, i32)| match get_piece(&core.active_piece_id) {
        Some(piece) => {
            let s = footprint(piece, core.placement_rotation);
            (c.0 - (s.width - 1) / 2, c.1 - (s.height - 1) / 2)
        }
        None => c,
    };
    if response.clicked() {
        if placing {
            if let Some(c) = cell {
                let a = anchor(&tab.core, c);
                tab.core.place(a.0, a.1);
            }
        } else {
            match &hit {
                Some(uid) => tab.core.select(uid, shift),
                None => tab.core.clear_selection(),
            }
        }
    }
    if response.middle_clicked() {
        if let Some(uid) = hit.clone() {
            tab.core.pick_piece(&uid);
        }
    }

    // Scene (rebuilt only when the document, selection or ghosts change).
    let mut ghosts = Vec::new();
    let mut ghosts_valid = true;
    tab.canvas.placement_problem = None;
    if placing && !tab.canvas3d.orbiting {
        if let Some(c) = cell {
            let a = anchor(&tab.core, c);
            let color = brickpress_render::scene::srgb(&tab.core.active_pass().color);
            ghosts = tab.core.ghosts(a.0, a.1).into_iter().map(|g| (g, color)).collect();
            let problem = tab.core.placement_problem(a.0, a.1);
            ghosts_valid = problem.is_none();
            tab.canvas.placement_problem = problem;
        }
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    // The renderer caches one scene; another tab must never match it.
    tab.id.hash(&mut h);
    tab.core.generation().hash(&mut h);
    tab.core.selected.hash(&mut h);
    for (g, _) in &ghosts {
        (g.x, g.y, g.rotation, &g.piece_id).hash(&mut h);
    }
    ghosts_valid.hash(&mut h);
    let key = h.finish();
    if tab.canvas3d.key != key || tab.canvas3d.scene.is_none() {
        let selected: HashSet<&str> = tab.core.selected.iter().map(|s| &**s).collect();
        tab.canvas3d.scene = Some(Arc::new(Scene3D::build(tab.core.doc(), &selected, (0, 0), &ghosts, ghosts_valid)));
        tab.canvas3d.key = key;
    }
    let accent = [0.0, 0.341, 0.847];
    let size = [(viewport.width() * ppp).round() as u32, (viewport.height() * ppp).round() as u32];
    let callback = Board3DCallback {
        size,
        camera: frame,
        board,
        accent,
        scene_key: key,
        scene: tab.canvas3d.scene.clone().expect("built"),
    };
    ui.painter().add(egui_wgpu::Callback::new_paint_callback(viewport, callback));
    tab.canvas3d.frame = Some(frame);

    // Floating camera toolbar.
    let rect = Rect::from_center_size(pos2(viewport.center().x, viewport.bottom() - 34.0), vec2(384.0, 44.0));
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
            for command in [Command::SelectTool, Command::PlaceTool] {
                let meta = command.meta();
                let checked = command.checked(app).unwrap_or(false);
                let shortcut = command.shortcut_text(ui.ctx());
                if widgets::icon_button(ui, meta.icon, meta.label, shortcut.as_deref(), checked).clicked() {
                    let ctx = ui.ctx().clone();
                    app.run(command, &ctx);
                }
            }
            if widgets::icon_button(ui, icons::ROTATE_CW, "Rotate", Some("R"), false).clicked() {
                let ctx = ui.ctx().clone();
                app.run(Command::Rotate, &ctx);
            }
            sep(ui);
            if widgets::icon_button(ui, icons::SQUARE, "Top view", None, false).clicked() {
                app.tab_mut().camera.action(&frame, CameraAction::Top);
            }
            if widgets::icon_button(ui, icons::BOX, "Isometric view", None, false).clicked() {
                app.tab_mut().camera.action(&frame, CameraAction::Isometric);
            }
            let ortho = app.tab().camera.projection == Projection::Orthographic;
            if widgets::icon_button(
                ui,
                icons::VIEW,
                if ortho { "Perspective" } else { "Orthographic (no perspective)" },
                None,
                ortho,
            )
            .clicked()
            {
                app.tab_mut().camera.projection =
                    if ortho { Projection::Perspective } else { Projection::Orthographic };
            }
            sep(ui);
            if widgets::icon_button(ui, icons::ZOOM_OUT, "Zoom out", None, false).clicked() {
                zoom_3d(app, 0.8);
            }
            ui.add_sized(
                vec2(48.0, 28.0),
                egui::Label::new(
                    egui::RichText::new(format!("{}%", (app.tab().camera.scale * 100.0).round()))
                        .font(egui::FontId::monospace(12.0)),
                ),
            );
            if widgets::icon_button(ui, icons::ZOOM_IN, "Zoom in", None, false).clicked() {
                zoom_3d(app, 1.25);
            }
            if widgets::icon_button(ui, icons::SCAN, "Fit", None, false).clicked() {
                app.tab_mut().camera.action(&frame, CameraAction::Reset);
            }
        });

    // Keyboard orbit.
    if !ui.ctx().text_edit_focused() && !app.modal_open() {
        let keys = ui.input(|i| {
            [
                (
                    i.key_pressed(egui::Key::ArrowLeft),
                    if i.modifiers.shift { CameraAction::PanLeft } else { CameraAction::Left },
                ),
                (
                    i.key_pressed(egui::Key::ArrowRight),
                    if i.modifiers.shift { CameraAction::PanRight } else { CameraAction::Right },
                ),
                (
                    i.key_pressed(egui::Key::ArrowUp),
                    if i.modifiers.shift { CameraAction::PanUp } else { CameraAction::Up },
                ),
                (
                    i.key_pressed(egui::Key::ArrowDown),
                    if i.modifiers.shift { CameraAction::PanDown } else { CameraAction::Down },
                ),
            ]
        });
        for (pressed, action) in keys {
            if pressed {
                app.tab_mut().camera.action(&frame, action);
            }
        }
    }
    let hint = if placing {
        "Click to place · drag to orbit · right-drag to pan"
    } else {
        "Drag to orbit · right-drag to pan · scroll to zoom"
    };
    ui.painter().text(
        viewport.left_top() + vec2(14.0, 12.0),
        egui::Align2::LEFT_TOP,
        hint,
        egui::FontId::proportional(12.0),
        t.text_secondary,
    );
}

fn zoom_3d(app: &mut BrickpressApp, factor: f32) {
    let tab = app.tab_mut();
    tab.canvas3d.zoom_by(&mut tab.camera, factor);
}

fn sep(ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(9.0, 20.0), Sense::hover());
    ui.painter().line_segment([rect.center_top(), rect.center_bottom()], Stroke::new(1.0, t.divider));
}
