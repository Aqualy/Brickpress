//! The window's top bar: menus, history, the document name and state, the
//! view switch and the primary Export action.

use crate::app::{BrickpressApp, InspectorTab, View};
use crate::commands::Command;
use crate::icons;
use crate::theme::{self, space, tokens, TOOLBAR_HEIGHT};
use crate::widgets::{self, ButtonKind};
use brickpress_core::Mode;
use egui::{Align, Layout, RichText, Ui};

/// One menu entry from the command registry: label, shortcut, check mark,
/// and a tooltip explaining why it is disabled.
pub fn menu_item(ui: &mut Ui, app: &mut BrickpressApp, command: Command) {
    let meta = command.meta();
    let availability = command.availability(app);
    let mut button = egui::Button::new(format!("{}  {}", meta.icon, meta.label));
    if let Some(text) = command.shortcut_text(ui.ctx()) {
        button = button.shortcut_text(text);
    }
    if let Some(checked) = command.checked(app) {
        button = button.selected(checked);
    }
    let response = ui.add_enabled(availability.is_ok(), button);
    let response = match availability {
        Err(reason) => response.on_disabled_hover_text(reason),
        Ok(()) => response,
    };
    if response.clicked() {
        let ctx = ui.ctx().clone();
        app.run(command, &ctx);
        ui.close();
    }
}

fn menus(ui: &mut Ui, app: &mut BrickpressApp) {
    egui::MenuBar::new().ui(ui, |ui| {
        // Roomy, evenly spaced menu titles with a full-height hover target.
        ui.spacing_mut().button_padding = egui::vec2(10.0, 5.0);
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.menu_button("File", |ui| {
            menu_item(ui, app, Command::New);
            menu_item(ui, app, Command::Open);
            ui.menu_button(format!("{}  Open Recent", icons::HISTORY), |ui| {
                if app.recent.is_empty() {
                    ui.add_enabled(false, egui::Button::new("No recent projects"));
                }
                for path in app.recent.clone() {
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    if ui.button(name).on_hover_text(path.display().to_string()).clicked() {
                        app.open_path(path);
                        ui.close();
                    }
                }
            });
            menu_item(ui, app, Command::OpenPresetAsDocument);
            ui.separator();
            menu_item(ui, app, Command::Save);
            menu_item(ui, app, Command::SaveAs);
            menu_item(ui, app, Command::CloseTab);
            ui.separator();
            menu_item(ui, app, Command::Export);
            menu_item(ui, app, Command::ImportPresets);
            menu_item(ui, app, Command::ExportPresets);
            ui.separator();
            menu_item(ui, app, Command::Quit);
        });
        ui.menu_button("Edit", |ui| {
            menu_item(ui, app, Command::Undo);
            menu_item(ui, app, Command::Redo);
            ui.separator();
            menu_item(ui, app, Command::Cut);
            menu_item(ui, app, Command::Copy);
            menu_item(ui, app, Command::Paste);
            menu_item(ui, app, Command::Duplicate);
            menu_item(ui, app, Command::Delete);
            ui.separator();
            menu_item(ui, app, Command::SelectAll);
            menu_item(ui, app, Command::Deselect);
        });
        ui.menu_button("Arrange", |ui| {
            menu_item(ui, app, Command::Rotate);
            menu_item(ui, app, Command::MirrorHorizontal);
            menu_item(ui, app, Command::MirrorVertical);
            ui.separator();
            menu_item(ui, app, Command::BringForward);
            menu_item(ui, app, Command::SendBackward);
            ui.separator();
            menu_item(ui, app, Command::AddPass);
            menu_item(ui, app, Command::SavePreset);
            menu_item(ui, app, Command::Reseed);
            ui.separator();
            menu_item(ui, app, Command::AddTracing);
            menu_item(ui, app, Command::RemoveTracing);
            ui.separator();
            menu_item(ui, app, Command::ClearBoard);
        });
        ui.menu_button("View", |ui| {
            menu_item(ui, app, Command::DesignView);
            menu_item(ui, app, Command::PrintView);
            menu_item(ui, app, Command::View3D);
            ui.separator();
            menu_item(ui, app, Command::ZoomIn);
            menu_item(ui, app, Command::ZoomOut);
            menu_item(ui, app, Command::ZoomFit);
            menu_item(ui, app, Command::ZoomActual);
            ui.separator();
            menu_item(ui, app, Command::ToggleGrid);
            menu_item(ui, app, Command::ToggleLibrary);
            menu_item(ui, app, Command::ToggleInspector);
            menu_item(ui, app, Command::ResetLayout);
            ui.separator();
            menu_item(ui, app, Command::NextTab);
            menu_item(ui, app, Command::PreviousTab);
            ui.separator();
            ui.menu_button(format!("{}  Appearance", icons::SUN), |ui| {
                let current = ui.ctx().options(|o| o.theme_preference);
                for (pref, label) in [
                    (egui::ThemePreference::System, "Match System"),
                    (egui::ThemePreference::Light, "Light"),
                    (egui::ThemePreference::Dark, "Dark"),
                ] {
                    if ui.add(egui::Button::new(label).selected(current == pref)).clicked() {
                        ui.ctx().set_theme(pref);
                        ui.close();
                    }
                }
            });
        });
        ui.menu_button("Help", |ui| {
            menu_item(ui, app, Command::CommandPalette);
            menu_item(ui, app, Command::Shortcuts);
            ui.separator();
            menu_item(ui, app, Command::AgentConnections);
            ui.separator();
            menu_item(ui, app, Command::Notices);
            menu_item(ui, app, Command::About);
        });
        ui.add_space(space::SM);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 18.0), egui::Sense::hover());
        ui.painter()
            .line_segment([rect.center_top(), rect.center_bottom()], egui::Stroke::new(1.0, tokens(ui.ctx()).divider));
        ui.add_space(space::XS);
        for command in [Command::Undo, Command::Redo] {
            let meta = command.meta();
            let shortcut = command.shortcut_text(ui.ctx());
            let ok = command.availability(app).is_ok();
            let r = ui
                .add_enabled_ui(ok, |ui| widgets::icon_button(ui, meta.icon, meta.label, shortcut.as_deref(), false))
                .inner;
            if r.clicked() {
                let ctx = ui.ctx().clone();
                app.run(command, &ctx);
            }
        }
    });
}

pub fn show(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    egui::Panel::top("topbar")
        .exact_size(TOOLBAR_HEIGHT)
        .frame(
            egui::Frame::new()
                .fill(t.panel)
                .inner_margin(egui::Margin::symmetric(8, 0))
                .stroke(egui::Stroke::new(1.0, t.divider)),
        )
        .show(ui, |ui| {
            let full = ui.max_rect();
            let center_width = 360.0f32.min(full.width() * 0.34);
            let zone = |ui: &mut Ui, rect: egui::Rect, layout: Layout, add: &mut dyn FnMut(&mut Ui)| {
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect).layout(layout), |ui| add(ui));
            };
            // Left: menus and history.
            let left = egui::Rect::from_min_max(full.min, egui::pos2(full.center().x - center_width / 2.0, full.max.y));
            zone(ui, left, Layout::left_to_right(Align::Center), &mut |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                menus(ui, app);
            });
            // Centre: document name and state.
            let center = egui::Rect::from_center_size(full.center(), egui::vec2(center_width, full.height()));
            zone(ui, center, Layout::left_to_right(Align::Center).with_main_align(Align::Center), &mut |ui| {
                title(app, ui)
            });
            // Right: view switch and Export.
            let right =
                egui::Rect::from_min_max(egui::pos2(full.center().x + center_width / 2.0, full.min.y), full.max);
            zone(ui, right, Layout::right_to_left(Align::Center), &mut |ui| {
                ui.spacing_mut().item_spacing.x = space::SM;
                let open = app.has_document();
                let export = ui
                    .add_enabled_ui(open, |ui| widgets::button(ui, ButtonKind::Primary, Some(icons::UPLOAD), "Export"))
                    .inner;
                let export =
                    widgets::tooltip(export, "Export PNG or SVG", Command::Export.shortcut_text(ui.ctx()).as_deref());
                if export.clicked() {
                    app.layout.inspector = true;
                    app.inspector_tab = InspectorTab::Export;
                }
                ui.add_enabled_ui(open, |ui| view_switch(app, ui));
            });
        });
}

fn title(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let Some(tab) = app.active_tab() else {
        ui.label(RichText::new("Brickpress").font(egui::FontId::new(13.0, theme::semibold())).color(t.text_secondary));
        return;
    };
    let title = tab.title();
    let state = if app.busy > 0 {
        "Working…"
    } else if tab.dirty() {
        "Edited"
    } else if tab.file.is_some() {
        "Saved"
    } else {
        "Not saved yet"
    };
    ui.spacing_mut().item_spacing.x = 6.0;
    let name_id = ui.make_persistent_id("doc-name");
    match ui.data(|d| d.get_temp::<String>(name_id)) {
        Some(mut text) => {
            let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(200.0).char_limit(160));
            r.request_focus();
            if r.lost_focus() {
                if !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    app.core_mut().rename(&text);
                }
                ui.data_mut(|d| d.remove::<String>(name_id));
            } else {
                ui.data_mut(|d| d.insert_temp(name_id, text));
            }
        }
        None => {
            let r = ui
                .add(
                    egui::Label::new(
                        RichText::new(&title).font(egui::FontId::new(13.0, theme::semibold())).color(t.text),
                    )
                    .truncate()
                    .sense(egui::Sense::click()),
                )
                .on_hover_text("Click to rename the project");
            if r.clicked() {
                ui.data_mut(|d| d.insert_temp(name_id, app.core().doc().name.clone()));
            }
            ui.label(RichText::new("·").color(t.text_disabled));
            ui.label(RichText::new(state).size(12.0).color(t.text_secondary));
        }
    }
}

fn view_switch(app: &mut BrickpressApp, ui: &mut Ui) {
    #[derive(Clone, Copy, PartialEq)]
    enum V {
        Design,
        Print,
        ThreeD,
    }
    let mut v = match (app.view(), app.core().mode) {
        (View::ThreeD, _) => V::ThreeD,
        (_, Mode::Print) => V::Print,
        _ => V::Design,
    };
    let before = v;
    widgets::segmented(
        ui,
        &mut v,
        &[
            (V::Design, "Design", "Compose pieces · 1"),
            (V::Print, "Print", "Letterpress preview · 2"),
            (V::ThreeD, "3D", "Orbit the build · 3"),
        ],
        false,
    );
    if v != before {
        let ctx = ui.ctx().clone();
        app.run(
            match v {
                V::Design => Command::DesignView,
                V::Print => Command::PrintView,
                V::ThreeD => Command::View3D,
            },
            &ctx,
        );
    }
}
