//! Modal dialogs and the command palette. Each dialog has one primary action,
//! a Cancel that is always safe, Escape to close and Enter to confirm.

use crate::app::{BrickpressApp, Confirm, Modal, Pending};
use crate::commands::Command;
use crate::icons;
use crate::theme::{self, space, tokens, RADIUS};
use crate::widgets::{self, ButtonKind};
use egui::{vec2, Align, CornerRadius, Key, Layout, RichText, Sense, Ui};

const NOTICES: &str = include_str!("../assets/THIRD_PARTY_NOTICES.txt");

fn dialog<R>(ctx: &egui::Context, id: &str, width: f32, add: impl FnOnce(&mut Ui) -> R) -> (R, bool) {
    let t = tokens(ctx);
    let response = egui::Modal::new(egui::Id::unique(id))
        .backdrop_color(egui::Color32::from_black_alpha(if ctx.theme() == egui::Theme::Dark { 120 } else { 70 }))
        .frame(
            egui::Frame::new()
                .fill(t.surface)
                .stroke(egui::Stroke::new(1.0, t.divider))
                .corner_radius(CornerRadius::same(12))
                .shadow(ctx.global_style().visuals.window_shadow)
                .inner_margin(20),
        )
        .show(ctx, |ui| {
            ui.set_width(width);
            add(ui)
        });
    let close = response.should_close();
    (response.inner, close)
}

fn title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(egui::FontId::new(15.0, theme::semibold())));
    ui.add_space(space::SM);
}

fn body(ui: &mut Ui, text: &str) {
    let t = tokens(ui.ctx());
    ui.add(egui::Label::new(RichText::new(text).color(t.text_secondary)).wrap());
    ui.add_space(space::LG);
}

/// Right-aligned buttons; returns the index clicked.
fn actions(ui: &mut Ui, buttons: &[(ButtonKind, &str)]) -> Option<usize> {
    let mut clicked = None;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        for (i, (kind, label)) in buttons.iter().enumerate().rev() {
            if widgets::button(ui, *kind, None, label).clicked() {
                clicked = Some(i);
            }
        }
    });
    clicked
}

pub fn show(app: &mut BrickpressApp, ctx: &egui::Context) {
    let Some(modal) = app.modal.clone() else { return };
    let enter = ctx.input(|i| i.key_pressed(Key::Enter));
    match modal {
        Modal::Unsaved(pending) => {
            let name = app.document_title();
            let text = match pending {
                Pending::CloseTab(_) => "Your changes will be lost if you don’t save them before closing.".to_string(),
                Pending::Quit => {
                    let more = match app.other_dirty_tabs() {
                        0 => String::new(),
                        1 => " You’ll be asked about 1 more project next.".into(),
                        n => format!(" You’ll be asked about {n} more projects next."),
                    };
                    format!("Your changes will be lost if you don’t save them before quitting.{more}")
                }
            };
            let (choice, close) = dialog(ctx, "unsaved", 400.0, |ui| {
                title(ui, &format!("Save changes to “{name}”?"));
                body(ui, &text);
                actions(
                    ui,
                    &[
                        (ButtonKind::Quiet, "Don’t Save"),
                        (ButtonKind::Secondary, "Cancel"),
                        (ButtonKind::Primary, "Save…"),
                    ],
                )
            });
            match choice.or(if enter { Some(2) } else { None }) {
                Some(0) => app.resolve_unsaved(pending, false),
                Some(2) => app.resolve_unsaved(pending, true),
                Some(_) => app.modal = None,
                None if close => app.modal = None,
                None => {}
            }
        }
        Modal::Conflict => {
            let (choice, close) = dialog(ctx, "conflict", 420.0, |ui| {
                title(ui, "The file changed on disk");
                body(ui, "Another program changed or removed this project since you opened it. Save a copy to keep both versions, or cancel and review the other change first.");
                actions(ui, &[(ButtonKind::Secondary, "Cancel"), (ButtonKind::Primary, "Save a Copy…")])
            });
            match choice {
                Some(1) => {
                    app.modal = None;
                    app.save(true);
                }
                Some(_) => app.modal = None,
                None if close => app.modal = None,
                None => {}
            }
        }
        Modal::Confirm(what) => {
            let (heading, text, label) = match &what {
                Confirm::ClearBoard => (
                    "Clear the board?".to_string(),
                    format!("All {} pieces will be removed. You can undo this.", app.core().doc().piece_count()),
                    "Clear Board",
                ),
                Confirm::RemovePass(id) => {
                    let pass = app.core().doc().pass(id);
                    (
                        format!("Delete “{}”?", pass.map(|p| p.name.as_str()).unwrap_or("this pass")),
                        format!(
                            "The pass and its {} pieces will be removed. You can undo this.",
                            pass.map(|p| p.pieces.len()).unwrap_or(0)
                        ),
                        "Delete Pass",
                    )
                }
            };
            let (choice, close) = dialog(ctx, "confirm", 380.0, |ui| {
                title(ui, &heading);
                body(ui, &text);
                actions(ui, &[(ButtonKind::Secondary, "Cancel"), (ButtonKind::Danger, label)])
            });
            match choice {
                Some(1) => {
                    app.modal = None;
                    match what {
                        Confirm::ClearBoard => {
                            app.core_mut().clear();
                        }
                        Confirm::RemovePass(id) => {
                            app.core_mut().remove_pass(&id);
                        }
                    }
                }
                Some(_) => app.modal = None,
                None if close => app.modal = None,
                None => {}
            }
        }
        Modal::SavePreset { mut name, selection_only } => {
            let (choice, close) = dialog(ctx, "save-preset", 380.0, |ui| {
                title(ui, "Save as preset");
                body(
                    ui,
                    if selection_only {
                        "Saves the selected pieces, with their inks, to your preset library on this device."
                    } else {
                        "Saves every visible piece, with its inks, to your preset library on this device."
                    },
                );
                widgets::field_label(ui, "Name");
                let r = ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(f32::INFINITY)
                        .char_limit(80)
                        .hint_text("For example: Flower motif"),
                );
                if !r.has_focus() && name.is_empty() {
                    r.request_focus();
                }
                ui.add_space(space::LG);
                let valid = !name.trim().is_empty();
                let mut c = actions(ui, &[(ButtonKind::Secondary, "Cancel"), (ButtonKind::Primary, "Save Preset")]);
                if enter && valid {
                    c = Some(1);
                }
                if c == Some(1) && !valid {
                    c = None;
                }
                c
            });
            match choice {
                Some(1) => {
                    app.modal = None;
                    if app.core_mut().save_preset(&name, selection_only) {
                        app.library_tab = crate::app::LibraryTab::Presets;
                        app.layout.library = true;
                    }
                }
                Some(_) => app.modal = None,
                None if close => app.modal = None,
                None => app.modal = Some(Modal::SavePreset { name, selection_only }),
            }
        }
        Modal::RenamePreset { id, mut name } => {
            let (choice, close) = dialog(ctx, "rename-preset", 360.0, |ui| {
                title(ui, "Rename preset");
                let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY).char_limit(80));
                r.request_focus();
                ui.add_space(space::LG);
                let c = actions(ui, &[(ButtonKind::Secondary, "Cancel"), (ButtonKind::Primary, "Rename")]);
                if enter {
                    Some(1)
                } else {
                    c
                }
            });
            match choice {
                Some(1) => {
                    app.modal = None;
                    app.core_mut().rename_preset(&id, &name);
                }
                Some(_) => app.modal = None,
                None if close => app.modal = None,
                None => app.modal = Some(Modal::RenamePreset { id, name }),
            }
        }
        Modal::SavePrintPreset { mut name } => {
            let (choice, close) = dialog(ctx, "save-print-preset", 360.0, |ui| {
                title(ui, "Save print settings");
                body(ui, "Saves the current press settings on this device. Saving under an existing name updates it.");
                let r = ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(f32::INFINITY)
                        .char_limit(80)
                        .hint_text("Preset name"),
                );
                if name.is_empty() {
                    r.request_focus();
                }
                ui.add_space(space::LG);
                let c = actions(ui, &[(ButtonKind::Secondary, "Cancel"), (ButtonKind::Primary, "Save")]);
                if enter && !name.trim().is_empty() {
                    Some(1)
                } else {
                    c
                }
            });
            match choice {
                Some(1) => {
                    app.modal = None;
                    app.core_mut().save_print_preset(&name);
                }
                Some(_) => app.modal = None,
                None if close => app.modal = None,
                None => app.modal = Some(Modal::SavePrintPreset { name }),
            }
        }
        Modal::Palette { query, index } => palette(app, ctx, query, index),
        Modal::Shortcuts => {
            let (_, close) = dialog(ctx, "shortcuts", 560.0, |ui| {
                title(ui, "Keyboard shortcuts");
                egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                    let groups: [(&str, &[Command]); 4] = [
                        (
                            "File",
                            &[
                                Command::New,
                                Command::Open,
                                Command::Save,
                                Command::SaveAs,
                                Command::Export,
                                Command::Quit,
                            ],
                        ),
                        (
                            "Edit",
                            &[
                                Command::Undo,
                                Command::Redo,
                                Command::Cut,
                                Command::Copy,
                                Command::Paste,
                                Command::Duplicate,
                                Command::Delete,
                                Command::SelectAll,
                                Command::Deselect,
                            ],
                        ),
                        (
                            "Tools",
                            &[
                                Command::SelectTool,
                                Command::PlaceTool,
                                Command::HandTool,
                                Command::Rotate,
                                Command::MirrorHorizontal,
                                Command::MirrorVertical,
                                Command::BringForward,
                                Command::SendBackward,
                            ],
                        ),
                        (
                            "View",
                            &[
                                Command::DesignView,
                                Command::PrintView,
                                Command::View3D,
                                Command::ZoomIn,
                                Command::ZoomOut,
                                Command::ZoomFit,
                                Command::ZoomActual,
                                Command::ToggleGrid,
                                Command::ToggleLibrary,
                                Command::ToggleInspector,
                                Command::CommandPalette,
                            ],
                        ),
                    ];
                    egui::Grid::new("shortcut-grid").num_columns(2).spacing(vec2(24.0, 6.0)).show(ui, |ui| {
                        for (group, commands) in groups {
                            ui.label(RichText::new(group).font(egui::FontId::new(12.0, theme::semibold())));
                            ui.end_row();
                            for c in commands {
                                ui.label(c.meta().label);
                                if let Some(s) = c.shortcut_text(ui.ctx()) {
                                    widgets::kbd(ui, &s);
                                }
                                ui.end_row();
                            }
                        }
                        ui.label(RichText::new("Board").font(egui::FontId::new(12.0, theme::semibold())));
                        ui.end_row();
                        for (what, keys) in [
                            ("Move selection (Shift: 4 studs)", "Arrows"),
                            ("Pan", "Space + drag, scroll"),
                            ("Zoom at the pointer", "Ctrl + scroll, pinch"),
                            ("Place at the keyboard cursor", "Enter"),
                            ("Pick the piece under the cursor", "I, middle-click"),
                            ("Select a whole ink pass", "Double-click"),
                            ("Cancel a stroke", "Esc, right-click"),
                        ] {
                            ui.label(what);
                            widgets::kbd(ui, keys);
                            ui.end_row();
                        }
                    });
                });
                ui.add_space(space::MD);
                actions(ui, &[(ButtonKind::Primary, "Done")])
            });
            if close || ctx.input(|i| i.key_pressed(Key::Enter)) {
                app.modal = None;
            }
        }
        Modal::About => {
            let (choice, close) = dialog(ctx, "about", 380.0, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Brickpress").font(egui::FontId::new(20.0, theme::semibold())));
                    ui.label(
                        RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                            .color(tokens(ui.ctx()).text_secondary),
                    );
                    ui.add_space(space::SM);
                    ui.add(
                        egui::Label::new("A studio for brick compositions and simulated letterpress prints.").wrap(),
                    );
                    ui.add_space(space::SM);
                    let gpu = app.render_device.is_some();
                    ui.label(
                        RichText::new(if gpu { "GPU rendering: active" } else { "GPU rendering: unavailable" })
                            .size(12.0)
                            .color(tokens(ui.ctx()).text_secondary),
                    );
                });
                ui.add_space(space::LG);
                actions(ui, &[(ButtonKind::Secondary, "Third-Party Notices"), (ButtonKind::Primary, "Close")])
            });
            match choice {
                Some(0) => app.modal = Some(Modal::Notices),
                Some(_) => app.modal = None,
                None if close => app.modal = None,
                None => {}
            }
        }
        Modal::Notices => {
            let (choice, close) = dialog(ctx, "notices", 620.0, |ui| {
                title(ui, "Third-party notices");
                egui::ScrollArea::vertical().max_height(440.0).show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(NOTICES).font(egui::FontId::monospace(11.0))).wrap());
                });
                ui.add_space(space::MD);
                actions(ui, &[(ButtonKind::Primary, "Close")])
            });
            if choice.is_some() || close {
                app.modal = None;
            }
        }
    }
}

fn palette(app: &mut BrickpressApp, ctx: &egui::Context, mut query: String, mut index: usize) {
    let t = tokens(ctx);
    let q = query.to_lowercase();
    let matches: Vec<Command> = Command::ALL
        .iter()
        .copied()
        .filter(|c| c.availability(app).is_ok() || !q.is_empty())
        .filter(|c| {
            let m = c.meta();
            let hay = format!("{} {}", m.label, m.keywords).to_lowercase();
            q.split_whitespace().all(|w| hay.contains(w))
        })
        .filter(|c| *c != Command::CommandPalette)
        .collect();
    let (down, up, enter) =
        ctx.input(|i| (i.key_pressed(Key::ArrowDown), i.key_pressed(Key::ArrowUp), i.key_pressed(Key::Enter)));
    if down {
        index = (index + 1).min(matches.len().saturating_sub(1));
    }
    if up {
        index = index.saturating_sub(1);
    }
    index = index.min(matches.len().saturating_sub(1));
    let mut run: Option<Command> = None;
    let area = egui::Modal::default_area(egui::Id::unique("palette")).anchor(egui::Align2::CENTER_TOP, vec2(0.0, 90.0));
    let response = egui::Modal::new(egui::Id::unique("palette"))
        .area(area)
        .backdrop_color(egui::Color32::from_black_alpha(50))
        .frame(
            egui::Frame::new()
                .fill(t.surface)
                .stroke(egui::Stroke::new(1.0, t.divider))
                .corner_radius(CornerRadius::same(12))
                .shadow(ctx.global_style().visuals.window_shadow)
                .inner_margin(8),
        )
        .show(ctx, |ui| {
            ui.set_width(520.0);
            let r = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .hint_text(format!("{}  Type a command", icons::SEARCH))
                    .desired_width(f32::INFINITY)
                    .frame(egui::Frame::NONE.inner_margin(8)),
            );
            r.request_focus();
            if r.changed() {
                index = 0;
            }
            widgets::divider(ui);
            ui.add_space(4.0);
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                if matches.is_empty() {
                    widgets::empty_state(
                        ui,
                        icons::COMMAND,
                        "No matching commands",
                        "Try another word, such as export or rotate.",
                    );
                }
                for (i, command) in matches.iter().enumerate() {
                    let meta = command.meta();
                    let available = command.availability(app);
                    let (rect, row) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                    if i == index {
                        ui.painter().rect_filled(rect, CornerRadius::same(RADIUS), t.selected);
                        row.scroll_to_me(None);
                    } else if row.hovered() {
                        ui.painter().rect_filled(rect, CornerRadius::same(RADIUS), t.hover);
                    }
                    let color = if available.is_ok() { t.text } else { t.text_disabled };
                    ui.painter().text(
                        rect.left_center() + vec2(12.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        meta.icon,
                        egui::FontId::new(15.0, theme::icon_family()),
                        color,
                    );
                    ui.painter().text(
                        rect.left_center() + vec2(38.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        meta.label,
                        egui::FontId::proportional(13.0),
                        color,
                    );
                    let right = match (&available, command.shortcut_text(ui.ctx())) {
                        (Err(reason), _) => Some(reason.to_string()),
                        (Ok(()), s) => s,
                    };
                    if let Some(text) = right {
                        ui.painter().text(
                            rect.right_center() - vec2(12.0, 0.0),
                            egui::Align2::RIGHT_CENTER,
                            text,
                            egui::FontId::proportional(11.5),
                            t.text_secondary,
                        );
                    }
                    if row.clicked() && available.is_ok() {
                        run = Some(*command);
                    }
                }
            });
        });
    if enter {
        if let Some(c) = matches.get(index).filter(|c| c.availability(app).is_ok()) {
            run = Some(*c);
        }
    }
    if let Some(command) = run {
        app.modal = None;
        app.run(command, ctx);
    } else if response.should_close() {
        app.modal = None;
    } else {
        app.modal = Some(Modal::Palette { query, index });
    }
}
