//! The left panel: the piece library and composition presets.

use crate::app::{BrickpressApp, LibraryTab, Modal};
use crate::commands::Command;
use crate::icons;
use crate::theme::{self, space, tokens, RADIUS};
use crate::widgets::{self, ButtonKind};
use brickpress_core::{catalog, Category, Mode, Piece, Tool};
use egui::{pos2, vec2, Align2, CornerRadius, FontId, Rect, RichText, Sense, Stroke, StrokeKind, Ui};

pub fn show(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    egui::Panel::left("library")
        .resizable(true)
        .default_size(272.0)
        .min_size(220.0)
        .max_size(420.0)
        .frame(
            egui::Frame::new()
                .fill(t.panel)
                .inner_margin(egui::Margin::symmetric(12, 0))
                .stroke(egui::Stroke::new(1.0, t.divider)),
        )
        .show(ui, |ui| {
            widgets::tabs(
                ui,
                &mut app.library_tab,
                &[(LibraryTab::Pieces, "Pieces"), (LibraryTab::Presets, "Presets")],
            );
            ui.add_space(space::SM);
            match app.library_tab {
                LibraryTab::Pieces => pieces(app, ui),
                LibraryTab::Presets => presets(app, ui),
            }
        });
}

fn matches(piece: &Piece, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let size = format!("{}x{}", piece.footprint.width_studs, piece.footprint.height_studs);
    let rsize = format!("{}x{}", piece.footprint.height_studs, piece.footprint.width_studs);
    let haystack = format!("{} {} {} {size} {rsize} {}", piece.name, piece.id, piece.category.label(), piece.notes)
        .to_lowercase()
        .replace('×', "x");
    query.to_lowercase().replace('×', "x").split_whitespace().all(|word| haystack.contains(word))
}

fn pieces(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let search = ui.add(
        egui::TextEdit::singleline(&mut app.library_search)
            .hint_text(format!("{}  Search pieces", icons::SEARCH))
            .desired_width(f32::INFINITY),
    );
    search.widget_info(|| egui::WidgetInfo::labeled(egui::Role::SearchInput, true, "Search pieces"));
    if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
        search.request_focus();
    }
    ui.add_space(space::SM);
    // Category chips.
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        let chip = |ui: &mut Ui, label: &str, selected: bool| -> bool {
            let r = ui.add(
                egui::Button::new(RichText::new(label).size(12.0))
                    .selected(selected)
                    .corner_radius(CornerRadius::same(12))
                    .min_size(vec2(0.0, 24.0)),
            );
            r.clicked()
        };
        if chip(ui, "All", app.library_category.is_none()) {
            app.library_category = None;
        }
        for category in Category::ALL {
            if chip(ui, category.label(), app.library_category == Some(category)) {
                app.library_category = if app.library_category == Some(category) { None } else { Some(category) };
            }
        }
    });
    ui.add_space(space::SM);
    let query = app.library_search.trim().to_string();
    let physical = app.core().doc().options.physical;
    let ink = theme::color32(&app.core().active_pass().color);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let all = &catalog().pieces;
        let filtered: Vec<usize> = (0..all.len())
            .filter(|&i| app.library_category.is_none_or(|c| all[i].category == c) && matches(&all[i], &query))
            .collect();
        if filtered.is_empty() {
            widgets::empty_state(ui, icons::SEARCH, "No pieces found", "Try a size such as 2x4, or another category.");
            return;
        }
        let groups: Vec<(String, Vec<usize>)> = if query.is_empty() && app.library_category.is_none() {
            let mut groups = vec![(
                "Essentials".to_string(),
                filtered.iter().copied().filter(|&i| all[i].default_toolbar).collect(),
            )];
            for category in Category::ALL {
                groups.push((
                    category.label().into(),
                    filtered.iter().copied().filter(|&i| all[i].category == category).collect(),
                ));
            }
            groups
        } else {
            vec![(format!("{} result{}", filtered.len(), if filtered.len() == 1 { "" } else { "s" }), filtered)]
        };
        for (title, items) in groups {
            if items.is_empty() {
                continue;
            }
            ui.add_space(space::XS);
            ui.label(RichText::new(title).font(FontId::new(11.5, theme::semibold())).color(t.text_secondary));
            ui.add_space(space::XS);
            let columns = ((ui.available_width() + 6.0) / 66.0).floor().max(2.0) as usize;
            let size = (ui.available_width() - 6.0 * (columns - 1) as f32) / columns as f32;
            for chunk in items.chunks(columns) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for &i in chunk {
                        piece_tile(app, ui, i, size, ink, physical);
                    }
                });
                ui.add_space(6.0 - ui.spacing().item_spacing.y);
            }
            ui.add_space(space::SM);
        }
    });
}

fn piece_tile(app: &mut BrickpressApp, ui: &mut Ui, index: usize, size: f32, ink: egui::Color32, physical: bool) {
    let t = tokens(ui.ctx());
    let piece = &catalog().pieces[index];
    let unavailable = physical && !piece.is_physical();
    let selected = app.core().tool == Tool::Place
        && app.core().active_preset_id.is_none()
        && app.core().active_piece_id == piece.id;
    let (rect, response) = ui.allocate_exact_size(vec2(size, size + 18.0), Sense::click());
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, !unavailable, selected, &piece.name));
    let tile = Rect::from_min_size(rect.min, vec2(size, size));
    let fill = if selected {
        t.selected
    } else if response.hovered() {
        t.hover
    } else {
        t.surface
    };
    let stroke = if selected { Stroke::new(1.5, t.accent_text) } else { Stroke::new(1.0, t.divider) };
    ui.painter().rect(tile, CornerRadius::same(RADIUS + 2), fill, stroke, StrokeKind::Inside);
    if let Some(texture) = app.thumbnails.get(index) {
        let tint = if unavailable { t.text_disabled } else { ink };
        let image_rect = tile.shrink(size * 0.14);
        ui.painter().image(texture.id(), image_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
    }
    if piece.surface_height_plates != 1.0 {
        ui.painter().text(
            tile.right_top() + vec2(-5.0, 4.0),
            Align2::RIGHT_TOP,
            "⅔",
            FontId::new(10.0, theme::semibold()),
            t.warning,
        );
    }
    let label = format!("{}×{}", piece.footprint.width_studs, piece.footprint.height_studs);
    ui.painter().text(
        pos2(tile.center().x, tile.bottom() + 9.0),
        Align2::CENTER_CENTER,
        label,
        FontId::monospace(10.5),
        if unavailable { t.text_disabled } else { t.text_secondary },
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            tile.expand(1.5),
            CornerRadius::same(RADIUS + 4),
            Stroke::new(2.0, t.focus),
            StrokeKind::Outside,
        );
    }
    let response = response.on_hover_ui(|ui| {
        ui.label(RichText::new(&piece.name).font(FontId::new(13.0, theme::semibold())));
        ui.label(
            RichText::new(format!(
                "{} · {}×{} studs · #{}",
                piece.category.label(),
                piece.footprint.width_studs,
                piece.footprint.height_studs,
                piece.id
            ))
            .size(12.0)
            .color(t.text_secondary),
        );
        if piece.surface_height_plates != 1.0 {
            ui.label(
                RichText::new("Not standard height: it cannot print with other pieces on a real press.")
                    .size(12.0)
                    .color(t.warning),
            );
        }
        if unavailable {
            ui.label(RichText::new("Unavailable while physical mode is on.").size(12.0).color(t.text_secondary));
        }
    });
    if response.clicked() && !unavailable {
        if app.core().mode != Mode::Design {
            app.core_mut().set_mode(Mode::Design);
        }
        if app.view() == crate::app::View::TwoD || app.view() == crate::app::View::ThreeD {
            app.core_mut().choose_piece(&piece.id);
        }
    }
}

fn presets(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    ui.horizontal(|ui| {
        let can_save = Command::SavePreset.availability(app).is_ok();
        let label =
            if app.core().selected_pieces().is_empty() { "Save board as preset" } else { "Save selection as preset" };
        let r = ui
            .add_enabled_ui(can_save, |ui| {
                widgets::button(ui, ButtonKind::Secondary, Some(icons::BOOKMARK_PLUS), label)
            })
            .inner;
        if r.clicked() {
            let ctx = ui.ctx().clone();
            app.run(Command::SavePreset, &ctx);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button(icons::ELLIPSIS, |ui| {
                crate::topbar::menu_item(ui, app, Command::ImportPresets);
                crate::topbar::menu_item(ui, app, Command::ExportPresets);
            })
            .response
            .on_hover_text("Import or export presets");
        });
    });
    ui.add_space(space::SM);
    if app.core().presets.is_empty() {
        widgets::empty_state(
            ui,
            icons::BOOKMARK_PLUS,
            "No presets yet",
            "Select pieces and save them as a preset to reuse a motif. Presets are stored on this device.",
        );
        return;
    }
    let generation = app.core().presets_generation;
    let ctx = ui.ctx().clone();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for preset in app.core().presets.clone() {
            let selected = app.core().active_preset_id.as_deref() == Some(&preset.id);
            let texture = {
                let entry =
                    app.preset_thumbnails.get(&preset.id).filter(|(g, _)| *g == generation).map(|(_, tex)| tex.clone());
                entry.unwrap_or_else(|| {
                    let pieces: Vec<_> = preset
                        .passes
                        .iter()
                        .flat_map(|pass| {
                            let rgb = brickpress_core::parse_color(&pass.color).unwrap_or([0, 0, 0]);
                            pass.pieces.iter().map(move |p| (p.clone(), rgb))
                        })
                        .collect();
                    let (w, h, rgba) = brickpress_render::scene::preview_rgba(
                        &app.atlas,
                        &pieces,
                        preset.width as f64,
                        preset.height as f64,
                        112,
                    );
                    let image = egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], &rgba);
                    let tex = ctx.load_texture(format!("preset-{}", preset.id), image, egui::TextureOptions::LINEAR);
                    app.preset_thumbnails.insert(preset.id.clone(), (generation, tex.clone()));
                    tex
                })
            };
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0), Sense::click());
            response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected, &preset.name));
            let fill = if selected {
                t.selected
            } else if response.hovered() {
                t.hover
            } else {
                egui::Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, CornerRadius::same(RADIUS + 2), fill);
            let thumb = Rect::from_min_size(rect.min + vec2(6.0, 6.0), vec2(52.0, 52.0));
            ui.painter().rect(
                thumb,
                CornerRadius::same(RADIUS),
                egui::Color32::from_rgb(0xF6, 0xF4, 0xEC),
                Stroke::new(1.0, t.divider),
                StrokeKind::Inside,
            );
            ui.painter().image(
                texture.id(),
                thumb.shrink(3.0),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            // Text stops short of the open button, so the two never overlap.
            let text_w = (rect.width() - 68.0 - 40.0).max(20.0);
            let line = |text: String, font: FontId, color: egui::Color32| {
                let mut job = egui::text::LayoutJob::simple_singleline(text, font, color);
                job.wrap = egui::text::TextWrapping::truncate_at_width(text_w);
                ui.painter().layout_job(job)
            };
            let name = line(preset.name.clone(), FontId::new(13.0, theme::medium()), t.text);
            ui.painter().galley(rect.min + vec2(68.0, 22.0 - name.size().y / 2.0), name, t.text);
            let meta = line(
                format!("{}×{} studs · {} pieces", preset.width, preset.height, preset.piece_count()),
                FontId::new(11.5, egui::FontFamily::Proportional),
                t.text_secondary,
            );
            ui.painter().galley(rect.min + vec2(68.0, 42.0 - meta.size().y / 2.0), meta, t.text_secondary);
            if response.has_focus() {
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(RADIUS + 2),
                    Stroke::new(2.0, t.focus),
                    StrokeKind::Inside,
                );
            }
            // Secondary action: open the preset as a project of its own. It
            // shows on hover, selection or keyboard focus to keep the list calm.
            let open_rect = Rect::from_center_size(rect.right_center() - vec2(8.0 + 14.0, 0.0), vec2(28.0, 28.0));
            let open_id = ui.scope_id().with(("preset-open", &preset.id));
            let show_open = selected
                || ui.rect_contains_pointer(rect)
                || response.has_focus()
                || ui.data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
            let open = ui
                .scope_builder(egui::UiBuilder::new().max_rect(open_rect).id_salt(open_id), |ui| {
                    if !show_open {
                        // Transparent but still focusable and clickable.
                        ui.multiply_opacity(0.0);
                    }
                    let meta = Command::OpenPresetAsDocument.meta();
                    widgets::icon_button_sized(ui, meta.icon, "Open as new project", None, false, 28.0)
                })
                .inner;
            // Keep the button visible while it has keyboard focus.
            if ui.data(|d| d.get_temp::<bool>(open_id).unwrap_or(false)) != open.has_focus() {
                ui.data_mut(|d| d.insert_temp(open_id, open.has_focus()));
                ui.ctx().request_repaint();
            }
            let has_document = app.has_document();
            let hint = if has_document {
                "Click to place. Right-click for more."
            } else {
                "Click to open as a new project. Right-click for more."
            };
            let response = response.on_hover_text(hint);
            let mut open_document = open.clicked();
            if response.clicked() {
                app.library_preset = Some(preset.id.clone());
                if has_document {
                    if app.core().mode != Mode::Design {
                        app.core_mut().set_mode(Mode::Design);
                    }
                    app.core_mut().choose_preset(&preset.id);
                } else {
                    open_document = true;
                }
            }
            response.context_menu(|ui| {
                app.library_preset = Some(preset.id.clone());
                let place = ui.add_enabled(has_document, egui::Button::new(format!("{}  Place", icons::STAMP)));
                if place.on_disabled_hover_text(crate::app::NO_DOCUMENT).clicked() {
                    app.core_mut().choose_preset(&preset.id);
                    ui.close();
                }
                crate::topbar::menu_item(ui, app, Command::OpenPresetAsDocument);
                if ui.button(format!("{}  Rename…", icons::PENCIL)).clicked() {
                    app.modal = Some(Modal::RenamePreset { id: preset.id.clone(), name: preset.name.clone() });
                    ui.close();
                }
                ui.separator();
                if ui.button(RichText::new(format!("{}  Delete", icons::TRASH_2)).color(t.danger)).clicked() {
                    app.core_mut().delete_preset(&preset.id);
                    app.notify(format!("Deleted “{}”.", preset.name));
                    ui.close();
                }
            });
            if open_document {
                app.library_preset = Some(preset.id.clone());
                app.run(Command::OpenPresetAsDocument, &ctx);
            }
            ui.add_space(2.0);
        }
    });
}
