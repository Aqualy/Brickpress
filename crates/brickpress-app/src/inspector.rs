//! The right panel: context, inks, print and export.

use crate::app::{BrickpressApp, Confirm, ExportFormat, InspectorTab, JobResult, Modal, View};
use crate::commands::Command;
use crate::icons;
use crate::theme::{self, space, tokens, CONTROL_HEIGHT, RADIUS};
use crate::widgets::{self, ButtonKind, SliderField};
use brickpress_core::catalog::get_piece;
use brickpress_core::persistence::preferences::GridAppearance;
use brickpress_core::settings::{paper_presets, print_presets, PRINT_CONTROLS, PRINT_DEFAULTS, STARTER_SWATCHES};
use brickpress_core::{GridStyle, Mode, PressDocument, PrintKey, Symmetry, Tool};
use brickpress_render::export::{export_dimensions, export_filename, ExportMode, ExportOptions};
use egui::{vec2, Align, CornerRadius, Layout, RichText, Sense, Stroke, StrokeKind, Ui};

pub fn next_swatch(doc: &PressDocument) -> String {
    STARTER_SWATCHES
        .iter()
        .chain(doc.swatches.iter().map(|s| s.as_str()).collect::<Vec<_>>().iter())
        .find(|s| !doc.passes.iter().any(|p| p.color.eq_ignore_ascii_case(s)))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "#315eae".into())
}

pub fn show(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    egui::Panel::right("inspector")
        .resizable(true)
        .default_size(312.0)
        .min_size(268.0)
        .max_size(460.0)
        .frame(
            egui::Frame::new()
                .fill(t.panel)
                .inner_margin(egui::Margin::symmetric(14, 0))
                .stroke(egui::Stroke::new(1.0, t.divider)),
        )
        .show(ui, |ui| {
            widgets::tabs(
                ui,
                &mut app.inspector_tab,
                &[
                    (InspectorTab::Design, "Design"),
                    (InspectorTab::Inks, "Inks"),
                    (InspectorTab::Print, "Print"),
                    (InspectorTab::Export, "Export"),
                ],
            );
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(space::SM);
                if !app.has_document() {
                    widgets::empty_state(
                        ui,
                        icons::SLIDERS_HORIZONTAL,
                        "No project open",
                        "Board, inks, print and export settings appear here for the active project.",
                    );
                    return;
                }
                match app.inspector_tab {
                    InspectorTab::Design => design(app, ui),
                    InspectorTab::Inks => inks(app, ui),
                    InspectorTab::Print => print(app, ui),
                    InspectorTab::Export => export(app, ui),
                }
                ui.add_space(space::XL);
            });
        });
}

fn command_button(app: &mut BrickpressApp, ui: &mut Ui, command: Command) {
    let meta = command.meta();
    let ok = command.availability(app);
    let shortcut = command.shortcut_text(ui.ctx());
    let r = ui
        .add_enabled_ui(ok.is_ok(), |ui| widgets::icon_button(ui, meta.icon, meta.label, shortcut.as_deref(), false))
        .inner;
    if r.clicked() {
        let ctx = ui.ctx().clone();
        app.run(command, &ctx);
    }
}

// ---- Design ------------------------------------------------------------------

fn design(app: &mut BrickpressApp, ui: &mut Ui) {
    let selected = app.core().selected_pieces().len();
    if selected > 0 {
        selection(app, ui, selected);
    } else if app.core().tool == Tool::Place && app.view() == View::TwoD || app.core().tool == Tool::Place {
        placing(app, ui);
    } else {
        widgets::section(ui, "insp-context", "Selection", true, |ui| {
            widgets::secondary_text(
                ui,
                "Click a piece to select it, or drag to select several. Choose a piece in the library to place it.",
            );
        });
    }
    board(app, ui);
    guides(app, ui);
    tracing(app, ui);
}

fn selection(app: &mut BrickpressApp, ui: &mut Ui, count: usize) {
    let title = format!("{count} piece{} selected", if count == 1 { "" } else { "s" });
    widgets::section(ui, "insp-selection", &title, true, |ui| {
        let rows = app.core().selected_pieces();
        let b = brickpress_core::geometry::bounds_of(rows.iter().map(|(_, p)| *p)).expect("non-empty");
        let single = (rows.len() == 1).then(|| (rows[0].1.clone(), rows[0].0.id.clone()));
        let passes: Vec<&str> = rows.iter().map(|(pass, _)| &*pass.id).collect();
        let same_pass = passes.windows(2).all(|w| w[0] == w[1]).then(|| passes[0].to_string());
        if let Some((piece, _)) = &single {
            if let Some(p) = get_piece(&piece.piece_id) {
                ui.label(RichText::new(&p.name).font(egui::FontId::new(13.0, theme::medium())));
                widgets::secondary_text(
                    ui,
                    &format!(
                        "Rotation {}°{}{}",
                        piece.rotation,
                        if piece.mx() { " · mirrored left–right" } else { "" },
                        if piece.my() { " · mirrored top–bottom" } else { "" }
                    ),
                );
                ui.add_space(space::XS);
            }
        }
        let (mut x, mut y) = (b.x, b.y);
        ui.horizontal(|ui| {
            widgets::field_label(ui, "X");
            let rx = ui.add(egui::DragValue::new(&mut x).speed(0.1).range(0..=127));
            ui.add_space(space::SM);
            widgets::field_label(ui, "Y");
            let ry = ui.add(egui::DragValue::new(&mut y).speed(0.1).range(0..=127));
            ui.add_space(space::SM);
            widgets::secondary_text(ui, &format!("{} × {}", b.width, b.height));
            if (rx.changed() || ry.changed()) && (x != b.x || y != b.y) {
                app.core_mut().move_selected(x - b.x, y - b.y);
            }
        });
        ui.horizontal(|ui| {
            widgets::field_label(ui, "Ink pass");
            let current = same_pass
                .as_deref()
                .and_then(|id| app.core().doc().pass(id))
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "Several".into());
            let mut target: Option<String> = None;
            egui::ComboBox::from_id_salt("selection-pass").selected_text(current).width(160.0).show_ui(ui, |ui| {
                for pass in app.core().doc().passes.iter() {
                    let label = format!("● {}", pass.name);
                    let r = ui.add_enabled(
                        pass.editable(),
                        egui::Button::new(RichText::new(label).color(theme::color32(&pass.color)))
                            .selected(same_pass.as_deref() == Some(&*pass.id)),
                    );
                    if r.clicked() {
                        target = Some(pass.id.to_string());
                    }
                }
            });
            if let Some(id) = target {
                app.core_mut().move_to_pass(&id);
            }
        });
        ui.add_space(space::XS);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for command in [
                Command::Rotate,
                Command::MirrorHorizontal,
                Command::MirrorVertical,
                Command::Duplicate,
                Command::BringForward,
                Command::SendBackward,
                Command::Delete,
            ] {
                command_button(app, ui, command);
            }
        });
    });
}

fn placing(app: &mut BrickpressApp, ui: &mut Ui) {
    if let Some(preset) = app.core().active_preset().cloned() {
        widgets::section(ui, "insp-placing", "Placing preset", true, |ui| {
            ui.label(RichText::new(&preset.name).font(egui::FontId::new(13.0, theme::medium())));
            widgets::secondary_text(
                ui,
                &format!(
                    "{}×{} studs · {} pieces · adds {} ink pass{}",
                    preset.width,
                    preset.height,
                    preset.piece_count(),
                    preset.passes.len(),
                    if preset.passes.len() == 1 { "" } else { "es" }
                ),
            );
            ui.add_space(space::XS);
            let mut rotation = app.core().preset_rotation;
            widgets::segmented(
                ui,
                &mut rotation,
                &[(0, "0°", ""), (90, "90°", ""), (180, "180°", ""), (270, "270°", "")],
                true,
            );
            app.core_mut().preset_rotation = rotation;
            ui.add_space(space::XS);
            widgets::secondary_text(ui, "Click the board to place. R rotates, Esc cancels.");
        });
        return;
    }
    let Some(piece) = get_piece(&app.core().active_piece_id) else { return };
    widgets::section(ui, "insp-placing", "Placing", true, |ui| {
        ui.horizontal(|ui| {
            if let Some(index) = brickpress_core::catalog().piece_index(&piece.id) {
                if let Some(tex) = app.thumbnails.get(index) {
                    let t = tokens(ui.ctx());
                    let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                    ui.painter().rect(
                        rect,
                        CornerRadius::same(RADIUS),
                        t.surface,
                        Stroke::new(1.0, t.divider),
                        StrokeKind::Inside,
                    );
                    ui.painter().image(
                        tex.id(),
                        rect.shrink(6.0),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        theme::color32(&app.core().active_pass().color),
                    );
                }
            }
            ui.vertical(|ui| {
                ui.label(RichText::new(&piece.name).font(egui::FontId::new(13.0, theme::medium())));
                widgets::secondary_text(
                    ui,
                    &format!(
                        "{}×{} studs · into {}",
                        piece.footprint.width_studs,
                        piece.footprint.height_studs,
                        app.core().active_pass().name
                    ),
                );
            });
        });
        ui.add_space(space::XS);
        let options: Vec<(i32, String)> = piece.allowed_rotations.iter().map(|r| (*r, format!("{r}°"))).collect();
        let refs: Vec<(i32, &str, &str)> = options.iter().map(|(r, l)| (*r, l.as_str(), "")).collect();
        let mut rotation = if piece.allows(app.core().placement_rotation) {
            app.core().placement_rotation
        } else {
            piece.allowed_rotations[0]
        };
        widgets::segmented(ui, &mut rotation, &refs, true);
        app.core_mut().placement_rotation = rotation;
        ui.add_space(space::XS);
        ui.horizontal(|ui| {
            let mx = app.core().placement_mirror_x;
            if widgets::icon_button(ui, icons::FLIP_HORIZONTAL_2, "Mirror horizontally", Some("Shift+H"), mx).clicked()
            {
                app.core_mut().placement_mirror_x = !mx;
            }
            let my = app.core().placement_mirror_y;
            if widgets::icon_button(ui, icons::FLIP_VERTICAL_2, "Mirror vertically", Some("Shift+V"), my).clicked() {
                app.core_mut().placement_mirror_y = !my;
            }
            ui.add_space(space::SM);
            widgets::secondary_text(ui, "Click or drag on the board. Middle-click a piece to pick it up.");
        });
    });
}

fn board(app: &mut BrickpressApp, ui: &mut Ui) {
    widgets::section(ui, "insp-board", "Board", true, |ui| {
        let current = app.core().doc().board;
        let (mut w, mut h) = (current.width, current.height);
        ui.horizontal(|ui| {
            widgets::field_label(ui, "Size");
            let rw = ui.add(egui::DragValue::new(&mut w).range(1..=128).suffix(" w").speed(0.2));
            ui.label("×");
            let rh = ui.add(egui::DragValue::new(&mut h).range(1..=128).suffix(" h").speed(0.2));
            widgets::secondary_text(ui, "studs");
            if (rw.changed() || rh.changed())
                && !(rw.dragged() || rh.dragged())
                && (w != current.width || h != current.height)
                && app.core_mut().resize(w, h)
            {
                app.tab_mut().canvas.fit();
            }
            if (rw.drag_stopped() || rh.drag_stopped()) && app.core_mut().resize(w, h) {
                app.tab_mut().canvas.fit();
            }
        });
        let mut physical = app.core().doc().options.physical;
        if widgets::toggle_row(
            ui,
            "Standard height only",
            "Only pieces a real press can print together (one plate high).",
            &mut physical,
        )
        .changed()
        {
            app.core_mut().set_physical(physical);
        }
        let mut overlap = app.core().doc().options.allow_overlap;
        if widgets::toggle_row(
            ui,
            "Allow overlapping pieces",
            "Pieces may cover each other. Real tiles cannot.",
            &mut overlap,
        )
        .changed()
        {
            app.core_mut().set_allow_overlap(overlap);
        }
    });
}

fn guides(app: &mut BrickpressApp, ui: &mut Ui) {
    widgets::section(ui, "insp-guides", "Grid & symmetry", true, |ui| {
        let mut grid = app.core().doc().options.grid;
        widgets::field_label(ui, "Grid");
        let options: Vec<(GridStyle, &str, &str)> = GridStyle::ALL.iter().map(|g| (*g, g.label(), "")).collect();
        if widgets::segmented(ui, &mut grid, &options, true).changed() {
            app.core_mut().commit(|d| d.options.grid = grid);
            app.show_grid = grid != GridStyle::Off;
        }
        ui.add_space(space::XS);
        ui.horizontal(|ui| {
            widgets::field_label(ui, "Major lines every");
            let mut major = app.core().doc().options.major_grid;
            if widgets::segmented(ui, &mut major, &[(4, "4", ""), (8, "8", "")], false).changed() {
                app.core_mut().commit(|d| d.options.major_grid = major);
            }
        });
        ui.horizontal(|ui| {
            widgets::field_label(ui, "Studs");
            let mut appearance = app.core().preferences.grid_appearance;
            if widgets::segmented(
                ui,
                &mut appearance,
                &[
                    (GridAppearance::Flat, "Flat", "Draw the grid as marks"),
                    (GridAppearance::Embossed, "Raised", "Draw raised studs"),
                ],
                false,
            )
            .changed()
            {
                app.core_mut().set_grid_appearance(appearance);
            }
        });
        ui.add_space(space::XS);
        widgets::field_label(ui, "Symmetry");
        let mut symmetry = app.core().doc().options.symmetry;
        let options: Vec<(Symmetry, &str, &str)> = vec![
            (Symmetry::Off, "Off", ""),
            (Symmetry::X, "Left–right", "Mirror across the vertical centre line"),
            (Symmetry::Y, "Top–bottom", "Mirror across the horizontal centre line"),
            (Symmetry::Both, "Both", "Mirror into all four quarters"),
        ];
        if widgets::segmented(ui, &mut symmetry, &options, true).changed() {
            app.core_mut().commit(|d| d.options.symmetry = symmetry);
        }
    });
}

fn tracing(app: &mut BrickpressApp, ui: &mut Ui) {
    widgets::section(ui, "insp-tracing", "Tracing image", false, |ui| {
        let Some(mut meta) = app.tab().trace.as_ref().map(|t| t.meta.clone()) else {
            widgets::secondary_text(ui, "Put a reference image under the board to trace. It stays on this device and never appears in projects or exports.");
            ui.add_space(space::XS);
            if widgets::button(ui, ButtonKind::Secondary, Some(icons::IMAGE_PLUS), "Add tracing image…").clicked() {
                app.choose_trace();
            }
            return;
        };
        let t = tokens(ui.ctx());
        ui.label(RichText::new(&meta.name).color(t.text));
        let mut changed = false;
        changed |= widgets::toggle_row(ui, "Visible", "", &mut meta.visible).changed();
        changed |= widgets::toggle_row(ui, "Locked", "A locked guide cannot be moved accidentally.", &mut meta.locked)
            .changed();
        let mut opacity = meta.opacity;
        if (SliderField {
            label: "Opacity",
            help: "",
            value: &mut opacity,
            range: 0.0..=1.0,
            default: 0.4,
            percent: true,
        })
        .show(ui)
        .changed()
        {
            meta.opacity = opacity;
            changed = true;
        }
        let (mut x, mut y, mut w) = (meta.x, meta.y, meta.width);
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!meta.locked, |ui| {
                widgets::field_label(ui, "X");
                changed |= ui.add(egui::DragValue::new(&mut x).speed(0.05).max_decimals(2)).changed();
                widgets::field_label(ui, "Y");
                changed |= ui.add(egui::DragValue::new(&mut y).speed(0.05).max_decimals(2)).changed();
                widgets::field_label(ui, "W");
                changed |=
                    ui.add(egui::DragValue::new(&mut w).speed(0.05).range(0.1..=1024.0).max_decimals(2)).changed();
            });
        });
        if (x, y, w) != (meta.x, meta.y, meta.width) {
            if let Ok(next) = meta.with_layout(x, y, w, meta.opacity) {
                meta = next;
            }
        }
        let board = app.core().doc().board;
        let mut remove = false;
        ui.horizontal(|ui| {
            if widgets::button(ui, ButtonKind::Quiet, Some(icons::SCAN), "Fit").clicked() {
                meta.fit(board.width as f64, board.height as f64);
                changed = true;
            }
            if widgets::button(ui, ButtonKind::Quiet, Some(icons::CROSSHAIR), "Centre").clicked() {
                meta.center(board.width as f64, board.height as f64);
                changed = true;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::button(ui, ButtonKind::Danger, Some(icons::TRASH_2), "Remove").clicked() {
                    remove = true;
                }
            });
        });
        if remove {
            app.remove_trace();
            return;
        }
        if let Some(t) = app.tab_mut().trace.as_mut() {
            if t.meta != meta {
                t.meta = meta;
                changed = true;
            }
        }
        if changed || ui.input(|i| i.pointer.any_released()) {
            app.persist_trace();
        }
    });
}

// ---- Inks ----------------------------------------------------------------------

fn inks(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new("Ink passes").font(egui::FontId::new(12.5, theme::semibold())));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            command_button(app, ui, Command::AddPass);
        });
    });
    widgets::secondary_text(
        ui,
        "Each pass is one colour run. Passes print from top to bottom, so later passes overprint earlier ones.",
    );
    ui.add_space(space::SM);
    let passes = app.core().doc().passes.clone();
    let active = app.core().active_pass().id.clone();
    let mut dropped: Option<(usize, usize)> = None;
    for (index, pass) in passes.iter().enumerate() {
        let is_active = pass.id == active;
        let row = ui.dnd_drop_zone::<usize, _>(egui::Frame::new(), |ui| {
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
            response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, is_active, &pass.name));
            let fill = if is_active {
                t.selected
            } else if response.hovered() {
                t.hover
            } else {
                egui::Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, CornerRadius::same(RADIUS + 1), fill);
            if response.has_focus() {
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(RADIUS + 1),
                    Stroke::new(2.0, t.focus),
                    StrokeKind::Inside,
                );
            }
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect.shrink2(vec2(4.0, 0.0)))
                    .layout(Layout::left_to_right(Align::Center)),
            );
            child.spacing_mut().item_spacing.x = 6.0;
            let grip = child.dnd_drag_source(egui::Id::unique(("pass-drag", &*pass.id)), index, |ui| {
                ui.label(
                    RichText::new(icons::GRIP_VERTICAL)
                        .font(egui::FontId::new(14.0, theme::icon_family()))
                        .color(t.text_disabled),
                );
            });
            grip.response.on_hover_text("Drag to reorder · Alt+↑/↓ while selected");
            let swatch = widgets::swatch(&mut child, theme::color32(&pass.color), false, "Change ink colour", 20.0);
            egui::Popup::from_toggle_button_response(&swatch)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    color_editor(app, ui, &pass.id);
                });
            let name_id = egui::Id::unique(("pass-name", &*pass.id));
            match child.data(|d| d.get_temp::<String>(name_id)) {
                Some(mut text) => {
                    let r = child
                        .add(egui::TextEdit::singleline(&mut text).desired_width(rect.width() - 150.0).char_limit(160));
                    r.request_focus();
                    if r.lost_focus() {
                        if !child.input(|i| i.key_pressed(egui::Key::Escape)) {
                            app.core_mut().rename_pass(&pass.id, &text);
                        }
                        child.data_mut(|d| d.remove::<String>(name_id));
                    } else {
                        child.data_mut(|d| d.insert_temp(name_id, text));
                    }
                }
                None => {
                    let r = child.add(
                        egui::Label::new(RichText::new(&pass.name).color(if pass.visible {
                            t.text
                        } else {
                            t.text_disabled
                        }))
                        .truncate()
                        .sense(Sense::click()),
                    );
                    if r.double_clicked() {
                        child.data_mut(|d| d.insert_temp(name_id, pass.name.clone()));
                    } else if r.clicked() {
                        app.core_mut().active_pass_id = pass.id.clone();
                    }
                    r.on_hover_text("Double-click to rename");
                }
            }
            child.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let lock_icon = if pass.locked { icons::LOCK_KEYHOLE } else { icons::LOCK_KEYHOLE_OPEN };
                if widgets::icon_button_sized(
                    ui,
                    lock_icon,
                    if pass.locked { "Unlock pass" } else { "Lock pass" },
                    None,
                    pass.locked,
                    26.0,
                )
                .clicked()
                {
                    app.core_mut().update_pass(&pass.id, |p| p.locked = !p.locked);
                }
                let eye = if pass.visible { icons::EYE } else { icons::EYE_OFF };
                if widgets::icon_button_sized(
                    ui,
                    eye,
                    if pass.visible { "Hide pass" } else { "Show pass" },
                    None,
                    false,
                    26.0,
                )
                .clicked()
                {
                    app.core_mut().update_pass(&pass.id, |p| p.visible = !p.visible);
                }
                ui.label(
                    RichText::new(pass.pieces.len().to_string())
                        .font(egui::FontId::monospace(11.5))
                        .color(t.text_secondary),
                );
                ui.add_space(4.0);
            });
            if response.clicked() {
                app.core_mut().active_pass_id = pass.id.clone();
            }
            if response.has_focus() && ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowUp)) {
                app.core_mut().reorder_pass(&pass.id, -1);
            }
            if response.has_focus() && ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowDown)) {
                app.core_mut().reorder_pass(&pass.id, 1);
            }
            response.context_menu(|ui| {
                if ui.button(format!("{}  Rename", icons::PENCIL)).clicked() {
                    ui.data_mut(|d| d.insert_temp(name_id, pass.name.clone()));
                    ui.close();
                }
                if ui.add_enabled(index > 0, egui::Button::new(format!("{}  Move up", icons::ARROW_UP))).clicked() {
                    app.core_mut().reorder_pass(&pass.id, -1);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        index + 1 < passes.len(),
                        egui::Button::new(format!("{}  Move down", icons::ARROW_DOWN)),
                    )
                    .clicked()
                {
                    app.core_mut().reorder_pass(&pass.id, 1);
                    ui.close();
                }
                ui.separator();
                let r = ui.add_enabled(
                    passes.len() > 1,
                    egui::Button::new(RichText::new(format!("{}  Delete pass", icons::TRASH_2)).color(t.danger)),
                );
                if r.clicked() {
                    if pass.pieces.is_empty() {
                        app.core_mut().remove_pass(&pass.id);
                    } else {
                        app.modal = Some(Modal::Confirm(Confirm::RemovePass(pass.id.clone())));
                    }
                    ui.close();
                }
            });
        });
        if let Some(from) = row.1 {
            dropped = Some((*from, index));
        }
    }
    if let Some((from, to)) = dropped {
        let id = passes[from].id.clone();
        app.core_mut().move_pass(&id, to as i32);
    }
    ui.add_space(space::MD);
    widgets::divider(ui);
    ui.add_space(space::SM);
    let active_id = app.core().active_pass().id.clone();
    let name = app.core().active_pass().name.clone();
    ui.label(RichText::new(format!("Colour · {name}")).font(egui::FontId::new(12.5, theme::semibold())));
    ui.add_space(space::XS);
    color_editor(app, ui, &active_id);
}

/// Picker, hex field and document swatches for one pass.
fn color_editor(app: &mut BrickpressApp, ui: &mut Ui, pass_id: &str) {
    let Some(pass_color) = app.core().doc().pass(pass_id).map(|p| p.color.clone()) else { return };
    let mut color = theme::color32(&pass_color);
    ui.spacing_mut().slider_width = 220.0;
    if widgets::color_picker(ui, &mut color) {
        let hex = theme::hex_of(color);
        let id = pass_id.to_string();
        app.core_mut().edit_live(|d| {
            if let Some(p) = d.pass_mut(&id) {
                p.color = hex;
            }
        });
    }
    ui.horizontal(|ui| {
        widgets::field_label(ui, "Hex");
        if let Some(hex) = widgets::hex_field(ui, egui::Id::unique(("hex", pass_id)), &pass_color) {
            app.core_mut().update_pass(pass_id, |p| p.color = hex);
        }
    });
    ui.add_space(space::XS);
    let swatches = app.core().doc().swatches.clone();
    let current = app.core().doc().pass(pass_id).map(|p| p.color.clone()).unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for swatch in &swatches {
            let selected = swatch.eq_ignore_ascii_case(&current);
            if widgets::swatch(ui, theme::color32(swatch), selected, swatch, 22.0).clicked() {
                let s = swatch.clone();
                app.core_mut().update_pass(pass_id, |p| p.color = s);
            }
        }
        let r = widgets::icon_button_sized(ui, icons::PLUS, "Add this colour to the swatches", None, false, 22.0);
        if r.clicked() && !swatches.iter().any(|s| s.eq_ignore_ascii_case(&current)) && swatches.len() < 200 {
            app.core_mut().commit(|d| d.swatches.push(current.clone()));
        }
    });
}

// ---- Print -----------------------------------------------------------------------

fn print(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    if app.core().mode != Mode::Print || app.view() != View::TwoD {
        egui::Frame::new().fill(t.hover).corner_radius(CornerRadius::same(RADIUS + 2)).inner_margin(10).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(icons::INFO)
                            .font(egui::FontId::new(14.0, theme::icon_family()))
                            .color(t.text_secondary),
                    );
                    widgets::secondary_text(ui, "These settings shape the Print view and print exports.");
                });
                if widgets::button(ui, ButtonKind::Secondary, Some(icons::STAMP), "Show print preview").clicked() {
                    let ctx = ui.ctx().clone();
                    app.run(Command::PrintView, &ctx);
                }
            },
        );
        ui.add_space(space::SM);
    }
    widgets::section(ui, "insp-press", "Press", true, |ui| {
        ui.horizontal(|ui| {
            widgets::field_label(ui, "Preset");
            let current = app.core().doc().print_settings;
            let name = print_presets()
                .into_iter()
                .find(|(_, s)| *s == current)
                .map(|(n, _)| n.to_string())
                .or_else(|| {
                    app.core().preferences.print_presets.iter().find(|p| p.settings == current).map(|p| p.name.clone())
                })
                .unwrap_or_else(|| "Custom".into());
            let mut chosen = None;
            egui::ComboBox::from_id_salt("print-preset").selected_text(name.clone()).width(150.0).show_ui(ui, |ui| {
                for (label, settings) in print_presets() {
                    if ui.add(egui::Button::new(label).selected(label == name)).clicked() {
                        chosen = Some(settings);
                    }
                }
                if !app.core().preferences.print_presets.is_empty() {
                    ui.separator();
                    for preset in &app.core().preferences.print_presets {
                        if ui.add(egui::Button::new(&preset.name).selected(preset.name == name)).clicked() {
                            chosen = Some(preset.settings);
                        }
                    }
                }
            });
            if let Some(settings) = chosen {
                app.core_mut().commit(|d| d.print_settings = settings);
            }
            if widgets::icon_button(ui, icons::BOOKMARK_PLUS, "Save these settings as a preset", None, false).clicked()
            {
                app.modal = Some(Modal::SavePrintPreset { name: String::new() });
            }
        });
        ui.add_space(space::XS);
        let groups: [(&str, &[PrintKey]); 4] = [
            ("Ink", &[PrintKey::InkAmount, PrintKey::Pressure, PrintKey::InkCoverage, PrintKey::RollerConsistency]),
            ("Wear", &[PrintKey::PieceWear, PrintKey::EdgeWear, PrintKey::PrintRoughness]),
            ("Texture", &[PrintKey::PaperTooth, PrintKey::TextureScale]),
            ("Registration", &[PrintKey::Misalignment, PrintKey::RegistrationError]),
        ];
        for (title, keys) in groups {
            ui.add_space(space::XS);
            ui.label(RichText::new(title).font(egui::FontId::new(11.5, theme::semibold())).color(t.text_secondary));
            for key in keys {
                let control = PRINT_CONTROLS.iter().find(|c| c.key == *key).expect("control");
                let mut value = app.core().doc().print_settings.get(*key);
                let r = SliderField {
                    label: control.label,
                    help: control.help,
                    value: &mut value,
                    range: control.min..=control.max,
                    default: PRINT_DEFAULTS.get(*key),
                    percent: control.max <= 1.0,
                }
                .show(ui);
                if r.changed() {
                    let key = *key;
                    app.core_mut().edit_live(|d| *d.print_settings.get_mut(key) = value);
                }
            }
        }
        ui.add_space(space::SM);
        ui.horizontal(|ui| {
            if widgets::button(ui, ButtonKind::Secondary, Some(icons::DICES), "New variation")
                .on_hover_text("Reseed every piece and pass: same settings, a different impression.")
                .clicked()
            {
                let ctx = ui.ctx().clone();
                app.run(Command::Reseed, &ctx);
            }
            if widgets::button(ui, ButtonKind::Quiet, Some(icons::ROTATE_CCW), "Defaults").clicked() {
                app.core_mut().commit(|d| d.print_settings = PRINT_DEFAULTS);
            }
        });
    });
    widgets::section(ui, "insp-paper", "Paper", true, |ui| {
        let paper = app.core().doc().paper.clone();
        ui.horizontal(|ui| {
            widgets::field_label(ui, "Stock");
            let mut chosen = None;
            egui::ComboBox::from_id_salt("paper-preset").selected_text(paper.preset.clone()).width(150.0).show_ui(
                ui,
                |ui| {
                    for preset in paper_presets() {
                        let label = format!("● {}", preset.preset);
                        let r = ui.add(
                            egui::Button::new(
                                RichText::new(label).color(theme::color32(&preset.color).gamma_multiply(0.8)),
                            )
                            .selected(preset.preset == paper.preset),
                        );
                        if r.clicked() {
                            chosen = Some(preset);
                        }
                    }
                },
            );
            if let Some(next) = chosen {
                app.core_mut().commit(|d| d.paper = brickpress_core::PaperSettings { seed: d.paper.seed, ..next });
            }
        });
        ui.horizontal(|ui| {
            widgets::field_label(ui, "Colour");
            let mut color = theme::color32(&paper.color);
            let swatch = widgets::swatch(ui, color, false, "Change paper colour", 22.0);
            egui::Popup::from_toggle_button_response(&swatch)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_width(240.0);
                    if widgets::color_picker(ui, &mut color) {
                        let hex = theme::hex_of(color);
                        app.core_mut().edit_live(|d| d.paper.color = hex);
                    }
                    ui.horizontal(|ui| {
                        widgets::field_label(ui, "Hex");
                        if let Some(hex) = widgets::hex_field(ui, egui::Id::unique("paper-hex"), &paper.color) {
                            app.core_mut().commit(|d| d.paper.color = hex);
                        }
                    });
                });
        });
        for (label, help, get) in [
            ("Grain", "Fine mottling of the sheet.", 0usize),
            ("Tooth", "Surface roughness: more tooth breaks up the ink.", 1),
            ("Fibres", "Visible strands in the sheet.", 2),
            ("Brightness", "Lightens or darkens the stock.", 3),
        ] {
            let mut value = [paper.grain, paper.tooth, paper.fibers, paper.brightness][get];
            let default = brickpress_core::settings::paper_preset(&paper.preset, 0)
                .map(|p| [p.grain, p.tooth, p.fibers, p.brightness][get])
                .unwrap_or(value);
            let r = SliderField {
                label,
                help,
                value: &mut value,
                range: if get == 3 { 0.6..=1.2 } else { 0.0..=1.0 },
                default,
                percent: true,
            }
            .show(ui);
            if r.changed() {
                app.core_mut().edit_live(|d| match get {
                    0 => d.paper.grain = value,
                    1 => d.paper.tooth = value,
                    2 => d.paper.fibers = value,
                    _ => d.paper.brightness = value,
                });
            }
        }
    });
}

// ---- Export -------------------------------------------------------------------

fn export(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let printed = app.export.print.unwrap_or(app.core().mode == Mode::Print);
    widgets::field_label(ui, "Format");
    let mut format = app.export.format;
    widgets::segmented(
        ui,
        &mut format,
        &[(ExportFormat::Png, "PNG", "Raster image"), (ExportFormat::Svg, "SVG", "Vector artwork")],
        true,
    );
    app.export.format = format;
    ui.add_space(space::SM);
    widgets::field_label(ui, "Artwork");
    let mut print = printed;
    widgets::segmented(
        ui,
        &mut print,
        &[(false, "Design", "Flat colours with printing gaps"), (true, "Print", "Simulated letterpress impression")],
        true,
    );
    app.export.print = Some(print);
    ui.add_space(space::SM);
    widgets::field_label(ui, "Width");
    let doc = app.tabs[app.active].core.doc();
    let presets = [1024u32, 2048, 4096, 8192];
    let mut choice = if presets.contains(&app.export.width) { app.export.width } else { 0 };
    let before = choice;
    widgets::segmented(
        ui,
        &mut choice,
        &[
            (1024, "1K", "1024 px"),
            (2048, "2K", "2048 px"),
            (4096, "4K", "4096 px"),
            (8192, "8K", "8192 px"),
            (0, "Custom", ""),
        ],
        true,
    );
    if choice != before && choice != 0 {
        app.export.width = choice;
    }
    if choice == 0 || before == 0 {
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut app.export.width).range(64..=12_000).suffix(" px").speed(4.0));
        });
    }
    let dims = export_dimensions(doc, app.export.width);
    match &dims {
        Ok((w, h)) => {
            let mp = (*w as f64 * *h as f64) / 1e6;
            let mm = doc.board.width as f64 * 8.0;
            let dpi = *w as f64 / (mm / 25.4);
            widgets::secondary_text(
                ui,
                &format!(
                    "{w} × {h} px · {mp:.1} MP · {dpi:.0} dpi at actual size ({:.0} × {:.0} mm)",
                    mm,
                    doc.board.height as f64 * 8.0
                ),
            );
        }
        Err(e) => {
            ui.label(RichText::new(e.to_string()).size(12.0).color(t.danger));
        }
    }
    ui.add_space(space::SM);
    widgets::toggle_row(
        ui,
        "Paper background",
        "Off exports transparent pixels around the ink.",
        &mut app.export.paper,
    );
    widgets::toggle_row(ui, "Stud grid", "Draw the grid lines into the image.", &mut app.export.grid);
    if app.export.format == ExportFormat::Svg && print {
        widgets::secondary_text(ui, "Print SVGs keep every piece as a vector, masked by the rendered ink texture.");
    }
    ui.add_space(space::MD);
    let can = dims.is_ok() && app.render_device.is_some();
    let r = ui
        .add_enabled_ui(can, |ui| {
            let width = ui.available_width();
            widgets::button_sized(ui, ButtonKind::Primary, Some(icons::UPLOAD), "Export…", Some(width))
        })
        .inner;
    let _ = CONTROL_HEIGHT;
    if r.clicked() {
        start_export(app, ui.ctx());
    }
    if app.render_device.is_none() {
        widgets::secondary_text(ui, "Exports need a graphics device, which is not available.");
    }
}

fn start_export(app: &mut BrickpressApp, ctx: &egui::Context) {
    let print = app.export.print.unwrap_or(app.core().mode == Mode::Print);
    let mode = if print { ExportMode::Print } else { ExportMode::Design };
    let (ext, filter) = match app.export.format {
        ExportFormat::Png => ("png", "PNG image"),
        ExportFormat::Svg => ("svg", "SVG image"),
    };
    let name = export_filename(&app.core().doc().name, mode, ext);
    let dialog = rfd::FileDialog::new().set_title("Export artwork").add_filter(filter, &[ext]).set_file_name(&name);
    let dialog = match app.tab().file.as_ref().and_then(|f| f.path.parent()) {
        Some(dir) => dialog.set_directory(dir),
        None => dialog,
    };
    let Some(path) = dialog.save_file() else { return };
    let Some((device, queue)) = app.render_device.clone() else { return };
    let doc = app.core().doc().clone();
    let atlas = app.atlas.clone();
    let appearance = app.core().preferences.grid_appearance;
    let options = ExportOptions { mode, paper: app.export.paper, grid: app.export.grid, width: app.export.width };
    let svg = app.export.format == ExportFormat::Svg;
    app.notify(format!("Exporting {}…", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()));
    app.spawn_job(
        move || {
            let renderer = brickpress_render::board2d::Renderer2D::new(
                &device,
                &queue,
                brickpress_render::export::EXPORT_FORMAT,
                atlas,
            );
            let bytes = if svg {
                let masks = if mode == ExportMode::Print {
                    match brickpress_render::svg::print_pass_masks(&device, &queue, &renderer, &doc, &options) {
                        Ok(m) => m,
                        Err(e) => return JobResult::Failed(format!("Export failed: {e}")),
                    }
                } else {
                    Vec::new()
                };
                match brickpress_render::svg::render_svg(&doc, &options, &masks) {
                    Ok(s) => s.into_bytes(),
                    Err(e) => return JobResult::Failed(format!("Export failed: {e}")),
                }
            } else {
                match brickpress_render::export::render_rgba(&device, &queue, &renderer, &doc, appearance, &options)
                    .and_then(|image| brickpress_render::export::encode_png(&image))
                {
                    Ok(b) => b,
                    Err(e) => return JobResult::Failed(format!("Export failed: {e}")),
                }
            };
            match crate::storage::atomic_replace(&path, &bytes) {
                Ok(()) => JobResult::Done(format!(
                    "Exported {}.",
                    path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
                )),
                Err(e) => JobResult::Failed(format!("Could not write the export: {e}")),
            }
        },
        ctx,
    );
}
