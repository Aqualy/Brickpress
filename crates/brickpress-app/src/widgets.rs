//! The component kit. Every screen is built from these, so states (hover,
//! pressed, selected, focus, disabled), sizes and spacing stay consistent.

use crate::theme::{self, space, tokens, CONTROL_HEIGHT, RADIUS};
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Layout, Rect, Response, Role, Sense, Stroke, StrokeKind,
    Ui, WidgetInfo,
};

fn focus_ring(ui: &Ui, response: &Response, rect: Rect, radius: u8) {
    if response.has_focus() && ui.is_enabled() {
        let t = tokens(ui.ctx());
        ui.painter().rect_stroke(
            rect.expand(1.5),
            CornerRadius::same(radius + 2),
            Stroke::new(2.0, t.focus),
            StrokeKind::Outside,
        );
    }
}

/// Hover text with an optional keyboard shortcut, shown as "Name  Ctrl+S".
pub fn tooltip(response: Response, label: &str, shortcut: Option<&str>) -> Response {
    let label = label.to_string();
    let shortcut = shortcut.map(str::to_string);
    response.on_hover_ui(move |ui| {
        ui.horizontal(|ui| {
            ui.label(label.as_str());
            if let Some(s) = &shortcut {
                ui.add_space(space::SM);
                kbd(ui, s);
            }
        });
    })
}

/// A keycap.
pub fn kbd(ui: &mut Ui, keys: &str) {
    let t = tokens(ui.ctx());
    let galley = ui.painter().layout_no_wrap(keys.to_string(), FontId::monospace(11.0), t.text_secondary);
    let size = galley.size() + vec2(10.0, 4.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(rect, CornerRadius::same(4), t.hover, Stroke::new(1.0, t.divider), StrokeKind::Inside);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, t.text_secondary);
}

/// A square, frameless icon button (toolbars, rows). `selected` shows it as
/// the active tool or toggled on.
pub fn icon_button(ui: &mut Ui, icon: &str, label: &str, shortcut: Option<&str>, selected: bool) -> Response {
    icon_button_sized(ui, icon, label, shortcut, selected, CONTROL_HEIGHT)
}

pub fn icon_button_sized(
    ui: &mut Ui,
    icon: &str,
    label: &str,
    shortcut: Option<&str>,
    selected: bool,
    size: f32,
) -> Response {
    let t = tokens(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::selected(Role::Button, enabled, selected, label));
    if ui.is_rect_visible(rect) {
        let fill = if selected {
            t.selected
        } else if response.is_pointer_button_down_on() {
            t.pressed
        } else if response.hovered() {
            t.hover
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, CornerRadius::same(RADIUS), fill);
        let color = if !enabled {
            t.text_disabled
        } else if selected {
            t.accent_text
        } else {
            t.text
        };
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, icon, FontId::new(16.0, theme::icon_family()), color);
        focus_ring(ui, &response, rect, RADIUS);
    }
    tooltip(response, label, shortcut)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Quiet,
    Danger,
}

/// A text button, optionally with a leading icon.
pub fn button(ui: &mut Ui, kind: ButtonKind, icon: Option<&str>, label: &str) -> Response {
    button_sized(ui, kind, icon, label, None)
}

/// A text button; `width` stretches it (e.g. full-width primary actions).
pub fn button_sized(ui: &mut Ui, kind: ButtonKind, icon: Option<&str>, label: &str, width: Option<f32>) -> Response {
    let t = tokens(ui.ctx());
    let font = FontId::new(13.0, theme::medium());
    let galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), t.text);
    let icon_w = if icon.is_some() { 16.0 + 6.0 } else { 0.0 };
    let width = width.unwrap_or((galley.size().x + icon_w + 24.0).max(CONTROL_HEIGHT));
    let (rect, response) = ui.allocate_exact_size(vec2(width, CONTROL_HEIGHT), Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let down = response.is_pointer_button_down_on();
        let hovered = response.hovered();
        let (fill, stroke, text) = match kind {
            ButtonKind::Primary => {
                let fill = if !enabled {
                    t.accent.gamma_multiply(0.45)
                } else if down {
                    t.accent.gamma_multiply(0.82)
                } else if hovered {
                    t.accent.gamma_multiply(0.92)
                } else {
                    t.accent
                };
                (fill, Stroke::NONE, t.on_accent)
            }
            ButtonKind::Secondary => (
                if down {
                    t.pressed
                } else if hovered {
                    t.hover
                } else {
                    t.surface
                },
                Stroke::new(1.0, if hovered { t.boundary } else { t.boundary.gamma_multiply(0.75) }),
                if enabled { t.text } else { t.text_disabled },
            ),
            ButtonKind::Quiet => (
                if down {
                    t.pressed
                } else if hovered {
                    t.hover
                } else {
                    Color32::TRANSPARENT
                },
                Stroke::NONE,
                if enabled { t.text } else { t.text_disabled },
            ),
            ButtonKind::Danger => (
                if down {
                    t.pressed
                } else if hovered {
                    t.hover
                } else {
                    t.surface
                },
                Stroke::new(1.0, t.danger.gamma_multiply(0.8)),
                if enabled { t.danger } else { t.text_disabled },
            ),
        };
        ui.painter().rect(rect, CornerRadius::same(RADIUS), fill, stroke, StrokeKind::Inside);
        let content_w = galley.size().x + icon_w;
        let mut x = rect.center().x - content_w / 2.0;
        if let Some(icon) = icon {
            ui.painter().text(
                pos2(x + 8.0, rect.center().y),
                Align2::CENTER_CENTER,
                icon,
                FontId::new(15.0, theme::icon_family()),
                text,
            );
            x += icon_w;
        }
        let galley = ui.painter().layout_no_wrap(label.to_string(), font, text);
        ui.painter().galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, text);
        focus_ring(ui, &response, rect, RADIUS);
    }
    response
}

/// A segmented control: mutually exclusive options in one pill.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str, &str)],
    full_width: bool,
) -> Response {
    let t = tokens(ui.ctx());
    let font = FontId::new(12.5, theme::medium());
    let widths: Vec<f32> = options
        .iter()
        .map(|(_, label, _)| ui.painter().layout_no_wrap(label.to_string(), font.clone(), t.text).size().x + 20.0)
        .collect();
    let natural: f32 = widths.iter().sum::<f32>() + 4.0;
    let width = if full_width { ui.available_width() } else { natural };
    let (outer, mut response) = ui.allocate_exact_size(vec2(width, CONTROL_HEIGHT), Sense::hover());
    ui.painter().rect_filled(outer, CornerRadius::same(RADIUS + 1), t.hover);
    let scale = (width - 4.0) / (natural - 4.0);
    let mut x = outer.left() + 2.0;
    for (i, (option, label, help)) in options.iter().enumerate() {
        let w = widths[i] * scale;
        let rect = Rect::from_min_size(pos2(x, outer.top() + 2.0), vec2(w, outer.height() - 4.0));
        x += w;
        let id = response.id.with(i);
        let r = ui.interact(rect, id, Sense::click());
        let selected = *value == *option;
        let enabled = ui.is_enabled();
        r.widget_info(|| WidgetInfo::selected(Role::RadioButton, enabled, selected, label));
        if r.clicked() && !selected {
            *value = *option;
            response.mark_changed();
        }
        if selected {
            ui.painter().rect(
                rect,
                CornerRadius::same(RADIUS),
                t.surface,
                Stroke::new(1.0, t.divider),
                StrokeKind::Inside,
            );
        } else if r.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::same(RADIUS), t.pressed.gamma_multiply(0.6));
        }
        let color = if !enabled {
            t.text_disabled
        } else if selected {
            t.text
        } else {
            t.text_secondary
        };
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, *label, font.clone(), color);
        focus_ring(ui, &r, rect, RADIUS);
        if !help.is_empty() {
            r.on_hover_text(*help);
        }
    }
    response
}

/// Underlined navigation tabs.
pub fn tabs<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> Response {
    let t = tokens(ui.ctx());
    let font = FontId::new(12.5, theme::medium());
    let (outer, response) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
    ui.painter().line_segment([outer.left_bottom(), outer.right_bottom()], Stroke::new(1.0, t.divider));
    let w = outer.width() / options.len() as f32;
    for (i, (option, label)) in options.iter().enumerate() {
        let rect = Rect::from_min_size(pos2(outer.left() + w * i as f32, outer.top()), vec2(w, outer.height()));
        let r = ui.interact(rect, response.id.with(i), Sense::click());
        let selected = *value == *option;
        r.widget_info(|| WidgetInfo::selected(Role::Tab, true, selected, label));
        if r.clicked() {
            *value = *option;
        }
        let color = if selected || r.hovered() { t.text } else { t.text_secondary };
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, *label, font.clone(), color);
        if selected {
            let bar = Rect::from_min_max(
                pos2(rect.left() + 10.0, rect.bottom() - 2.0),
                pos2(rect.right() - 10.0, rect.bottom()),
            );
            ui.painter().rect_filled(bar, CornerRadius::same(1), t.accent_text);
        }
        focus_ring(ui, &r, rect.shrink(3.0), RADIUS);
    }
    response
}

pub struct DocumentTabResponse {
    pub response: Response,
    /// The close button was clicked.
    pub close: bool,
}

/// A document tab (the strip above the canvas): the name, a dot while it has
/// unsaved changes, and a close button on hover and on the active tab. The
/// active tab takes the workspace colour so it joins the canvas below it.
pub fn document_tab(
    ui: &mut Ui,
    id: egui::Id,
    label: &str,
    dirty: bool,
    active: bool,
    hint: &str,
) -> DocumentTabResponse {
    const PAD: f32 = 14.0;
    const CLOSE: f32 = 20.0;
    const CLOSE_PAD: f32 = 6.0;
    let t = tokens(ui.ctx());
    let font = FontId::new(12.5, theme::medium());
    let natural = ui.painter().layout_no_wrap(label.to_string(), font.clone(), t.text).size().x;
    let dot = if dirty { 12.0 } else { 0.0 };
    let width = (PAD + natural + dot + 10.0 + CLOSE + CLOSE_PAD).clamp(128.0, 240.0);
    let height = ui.available_height().max(24.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let response = ui.interact(rect, id, Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::selected(Role::Tab, enabled, active, label));
    let hovered = ui.rect_contains_pointer(rect);
    let close_rect =
        Rect::from_center_size(pos2(rect.right() - CLOSE_PAD - CLOSE / 2.0, rect.center().y), vec2(CLOSE, CLOSE));
    let close_response = (hovered || active).then(|| {
        let r = ui.interact(close_rect, id.with("close"), Sense::click());
        r.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, format!("Close {label}")));
        r
    });
    let close = close_response.as_ref().is_some_and(|r| r.clicked());
    if ui.is_rect_visible(rect) {
        let r = RADIUS + 1;
        let top = CornerRadius { nw: r, ne: r, sw: 0, se: 0 };
        if active {
            // A card in the workspace colour, outlined on three sides, that
            // covers the strip's bottom line so it runs into the canvas.
            let outer = rect.with_max_y(rect.bottom() + 1.0);
            ui.painter().rect_filled(outer, top, t.divider);
            let inner_r = CornerRadius { nw: r - 1, ne: r - 1, sw: 0, se: 0 };
            ui.painter().rect_filled(
                Rect::from_min_max(outer.min + vec2(1.0, 1.0), outer.max - vec2(1.0, 0.0)),
                inner_r,
                t.workspace,
            );
        } else if hovered {
            ui.painter().rect_filled(rect.shrink2(vec2(0.0, 0.0)).with_max_y(rect.bottom() - 1.0), top, t.hover);
        }
        let color = if !enabled {
            t.text_disabled
        } else if active || hovered {
            t.text
        } else {
            t.text_secondary
        };
        let max_text = (close_rect.left() - 10.0 - dot - (rect.left() + PAD)).max(8.0);
        let font = if active { FontId::new(12.5, theme::semibold()) } else { font };
        let mut job = egui::text::LayoutJob::simple_singleline(label.to_string(), font, color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(max_text);
        let galley = ui.painter().layout_job(job);
        let text_pos = pos2(rect.left() + PAD, rect.center().y - galley.size().y / 2.0 + 0.5);
        let text_right = text_pos.x + galley.size().x;
        ui.painter().galley(text_pos, galley, color);
        if dirty {
            ui.painter().circle_filled(
                pos2(text_right + 8.0, rect.center().y + 0.5),
                3.0,
                if active { t.text_secondary } else { t.text_disabled },
            );
        }
        if let Some(r) = &close_response {
            let fill = if r.is_pointer_button_down_on() {
                t.pressed
            } else if r.hovered() {
                if active {
                    t.hover
                } else {
                    t.pressed
                }
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(close_rect, CornerRadius::same(RADIUS), fill);
            let color = if r.hovered() { t.text } else { t.text_secondary };
            ui.painter().text(
                close_rect.center(),
                Align2::CENTER_CENTER,
                crate::icons::X,
                FontId::new(14.0, theme::icon_family()),
                color,
            );
            focus_ring(ui, r, close_rect, 4);
        }
        focus_ring(ui, &response, rect.shrink(2.0), RADIUS);
    }
    if let Some(r) = close_response {
        r.on_hover_text("Close");
    }
    let response = if hint.is_empty() { response } else { response.on_hover_text(hint) };
    DocumentTabResponse { response, close }
}

/// A collapsible inspector section that remembers whether it is open.
pub fn section<R>(ui: &mut Ui, id: &str, title: &str, default_open: bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    let t = tokens(ui.ctx());
    let id = ui.make_persistent_id(id);
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
    let open = state.is_open();
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, open, title));
    if response.clicked() {
        state.toggle(ui);
    }
    if response.hovered() {
        ui.painter().rect_filled(rect.expand2(vec2(4.0, 0.0)), CornerRadius::same(RADIUS), t.hover);
    }
    ui.painter().text(
        pos2(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        title,
        FontId::new(12.5, theme::semibold()),
        t.text,
    );
    let chevron = if open { crate::icons::CHEVRON_DOWN } else { crate::icons::CHEVRON_RIGHT };
    ui.painter().text(
        pos2(rect.right() - 6.0, rect.center().y),
        Align2::RIGHT_CENTER,
        chevron,
        FontId::new(14.0, theme::icon_family()),
        t.text_secondary,
    );
    focus_ring(ui, &response, rect, RADIUS);
    state.store(ui.ctx());
    let result = state.show_body_unindented(ui, |ui| {
        let r = add(ui);
        ui.add_space(space::SM);
        r
    });
    ui.add_space(2.0);
    divider(ui);
    result.map(|r| r.inner)
}

pub fn divider(ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment([rect.left_center(), rect.right_center()], Stroke::new(1.0, t.divider));
}

/// Small secondary label above or beside a control.
pub fn field_label(ui: &mut Ui, text: &str) -> Response {
    let t = tokens(ui.ctx());
    ui.label(egui::RichText::new(text).size(12.0).color(t.text_secondary))
}

pub fn secondary_text(ui: &mut Ui, text: &str) -> Response {
    let t = tokens(ui.ctx());
    ui.add(egui::Label::new(egui::RichText::new(text).size(12.0).color(t.text_secondary)).wrap())
}

/// A labelled row: label on the left, control right-aligned.
pub fn row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.set_min_height(CONTROL_HEIGHT);
        field_label(ui, label);
        ui.with_layout(Layout::right_to_left(Align::Center), add).inner
    })
    .inner
}

/// A switch.
pub fn toggle(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let t = tokens(ui.ctx());
    let (rect, mut response) = ui.allocate_exact_size(vec2(32.0, 18.0), Sense::click());
    let enabled = ui.is_enabled();
    let value = *on;
    response.widget_info(|| WidgetInfo::selected(Role::Switch, enabled, value, label));
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
    let track = if *on { t.accent } else { t.boundary.gamma_multiply(0.6) };
    ui.painter().rect_filled(rect, CornerRadius::same(9), if enabled { track } else { track.gamma_multiply(0.5) });
    let x = egui::lerp((rect.left() + 9.0)..=(rect.right() - 9.0), how_on);
    ui.painter().circle_filled(pos2(x, rect.center().y), 7.0, Color32::WHITE);
    focus_ring(ui, &response, rect, 9);
    response
}

/// A labelled switch row.
pub fn toggle_row(ui: &mut Ui, label: &str, help: &str, on: &mut bool) -> Response {
    ui.horizontal(|ui| {
        ui.set_min_height(CONTROL_HEIGHT);
        let label_response = field_label(ui, label);
        let r = ui.with_layout(Layout::right_to_left(Align::Center), |ui| toggle(ui, on, label)).inner;
        if !help.is_empty() {
            label_response.on_hover_text(help);
        }
        r
    })
    .inner
}

/// A slider with its label and value on one line and an exact numeric field.
/// Double-click the value to reset it.
pub struct SliderField<'a> {
    pub label: &'a str,
    pub help: &'a str,
    pub value: &'a mut f64,
    pub range: std::ops::RangeInclusive<f64>,
    pub default: f64,
    pub percent: bool,
}

impl SliderField<'_> {
    pub fn show(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let SliderField { label, help, value, range, default, percent } = self;
        let mut changed = false;
        let response = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.horizontal(|ui| {
                    let l = field_label(ui, label);
                    if !help.is_empty() {
                        l.on_hover_text(help);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let speed = (range.end() - range.start()) / 300.0;
                        let mut drag = egui::DragValue::new(value).range(range.clone()).speed(speed);
                        drag = if percent {
                            drag.custom_formatter(|v, _| format!("{:.0}%", v * 100.0)).custom_parser(|s| {
                                s.trim_end_matches('%').trim().parse::<f64>().ok().map(|v| v / 100.0)
                            })
                        } else {
                            drag.max_decimals(2)
                        };
                        let r = ui.add_sized(vec2(56.0, 22.0), drag);
                        let r = r.on_hover_text("Drag or type a value. Double-click to reset.");
                        if r.double_clicked() {
                            *value = default;
                            changed = true;
                        }
                        changed |= r.changed();
                    });
                });
                let width = ui.available_width();
                ui.spacing_mut().slider_width = width;
                // A slimmer handle than egui's default, sized for dense panels.
                ui.spacing_mut().interact_size.y = 16.0;
                ui.spacing_mut().slider_rail_height = 4.0;
                let slider = egui::Slider::new(value, range).show_value(false);
                let r = ui.add(slider);
                changed |= r.changed();
                r
            })
            .inner;
        let _ = t;
        let mut response = response;
        if changed {
            response.mark_changed();
        }
        response
    }
}

/// A colour swatch button.
pub fn swatch(ui: &mut Ui, color: Color32, selected: bool, label: &str, size: f32) -> Response {
    let t = tokens(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, selected, label));
    let r = (size / 2.0).round() as u8;
    ui.painter().rect(
        rect,
        CornerRadius::same(r),
        color,
        Stroke::new(1.0, Color32::from_black_alpha(40)),
        StrokeKind::Inside,
    );
    if selected {
        ui.painter().rect_stroke(
            rect.expand(2.5),
            CornerRadius::same(r + 3),
            Stroke::new(2.0, t.accent_text),
            StrokeKind::Inside,
        );
    } else if response.hovered() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(r + 2),
            Stroke::new(1.0, t.boundary),
            StrokeKind::Inside,
        );
    }
    focus_ring(ui, &response, rect, r);
    response.on_hover_text(label)
}

/// A centred message for empty lists.
pub fn empty_state(ui: &mut Ui, icon: &str, title: &str, body: &str) {
    let t = tokens(ui.ctx());
    ui.vertical_centered(|ui| {
        ui.add_space(space::XL);
        ui.label(egui::RichText::new(icon).font(FontId::new(26.0, theme::icon_family())).color(t.text_disabled));
        ui.add_space(space::SM);
        ui.label(egui::RichText::new(title).font(FontId::new(13.0, theme::semibold())).color(t.text));
        ui.add_space(2.0);
        ui.add(egui::Label::new(egui::RichText::new(body).size(12.0).color(t.text_secondary)).wrap());
        ui.add_space(space::LG);
    });
}

/// A full-width, left-aligned list row: icon, label and quieter detail text
/// (recent files and similar lists).
pub fn list_row(ui: &mut Ui, icon: &str, label: &str, detail: &str, width: f32) -> Response {
    let t = tokens(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(width, CONTROL_HEIGHT + 2.0), Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let fill = if response.is_pointer_button_down_on() {
            t.pressed
        } else if response.hovered() {
            t.hover
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, CornerRadius::same(RADIUS), fill);
        let painter = ui.painter_at(rect.shrink2(vec2(space::SM, 0.0)));
        let y = rect.center().y;
        let x = rect.left() + space::SM;
        painter.text(
            pos2(x + 8.0, y),
            Align2::CENTER_CENTER,
            icon,
            FontId::new(15.0, theme::icon_family()),
            t.text_secondary,
        );
        let name = painter.layout_no_wrap(label.to_string(), FontId::new(13.0, theme::medium()), t.text);
        let name_w = name.size().x;
        painter.galley(pos2(x + 24.0, y - name.size().y / 2.0), name, t.text);
        if !detail.is_empty() {
            painter.text(
                pos2(x + 24.0 + name_w + space::SM, y),
                Align2::LEFT_CENTER,
                detail,
                FontId::new(12.0, egui::FontFamily::Proportional),
                t.text_secondary,
            );
        }
        focus_ring(ui, &response, rect, RADIUS);
    }
    response
}

/// A hex colour field. Returns a valid `#rrggbb` when edited.
pub fn hex_field(ui: &mut Ui, id: egui::Id, current: &str) -> Option<String> {
    let mut text = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| current.to_uppercase());
    let response = ui
        .add(egui::TextEdit::singleline(&mut text).desired_width(76.0).font(egui::TextStyle::Monospace).char_limit(7));
    let mut out = None;
    if response.changed() {
        let candidate = if text.starts_with('#') { text.clone() } else { format!("#{text}") };
        if brickpress_core::valid_color(&candidate) {
            out = Some(candidate.to_lowercase());
        }
        ui.data_mut(|d| d.insert_temp(id, text.clone()));
    }
    if !response.has_focus() && !response.changed() {
        ui.data_mut(|d| d.remove::<String>(id));
    }
    let valid = brickpress_core::valid_color(if text.starts_with('#') { &text } else { "" });
    if response.has_focus() && !valid {
        let t = tokens(ui.ctx());
        ui.painter().rect_stroke(
            response.rect,
            CornerRadius::same(RADIUS),
            Stroke::new(1.0, t.danger),
            StrokeKind::Inside,
        );
    }
    out
}

/// Heading at the top of a panel.
pub fn panel_heading(ui: &mut Ui, title: &str) {
    let t = tokens(ui.ctx());
    ui.add_space(space::MD);
    ui.label(egui::RichText::new(title).font(FontId::new(13.0, theme::semibold())).color(t.text));
    ui.add_space(space::XS);
}

/// A saturation/value square and hue strip. Returns true while the colour
/// changes. (egui's own picker includes controls this app does not need.)
pub fn color_picker(ui: &mut Ui, color: &mut Color32) -> bool {
    use egui::epaint::{Hsva, Mesh};
    let t = tokens(ui.ctx());
    let id = ui.make_persistent_id("color-picker-hsv");
    // Keep hue while saturation or value is zero, where RGB loses it.
    let stored: Option<(Color32, Hsva)> = ui.data(|d| d.get_temp(id));
    let mut hsva = match stored {
        Some((c, h)) if c == *color => h,
        _ => Hsva::from(*color),
    };
    let width = ui.available_width().min(260.0);
    let mut changed = false;

    let (square, response) = ui.allocate_exact_size(vec2(width, 150.0), Sense::click_and_drag());
    response.widget_info(|| WidgetInfo::labeled(Role::ColorWell, true, "Saturation and brightness"));
    if let Some(p) = response.interact_pointer_pos() {
        hsva.s = ((p.x - square.left()) / square.width()).clamp(0.0, 1.0);
        hsva.v = 1.0 - ((p.y - square.top()) / square.height()).clamp(0.0, 1.0);
        changed = true;
    }
    let mut mesh = Mesh::default();
    let steps = 24;
    for yi in 0..=steps {
        for xi in 0..=steps {
            let (s, v) = (xi as f32 / steps as f32, 1.0 - yi as f32 / steps as f32);
            let c: Color32 = Hsva { h: hsva.h, s, v, a: 1.0 }.into();
            mesh.colored_vertex(
                pos2(square.left() + s * square.width(), square.top() + (1.0 - v) * square.height()),
                c,
            );
        }
    }
    for yi in 0..steps {
        for xi in 0..steps {
            let i = yi * (steps + 1) + xi;
            mesh.add_triangle(i, i + 1, i + steps + 1);
            mesh.add_triangle(i + 1, i + steps + 2, i + steps + 1);
        }
    }
    ui.painter().add(mesh);
    ui.painter().rect_stroke(square, CornerRadius::same(4), Stroke::new(1.0, t.divider), StrokeKind::Inside);
    let marker = pos2(square.left() + hsva.s * square.width(), square.top() + (1.0 - hsva.v) * square.height());
    ui.painter().circle_stroke(marker, 6.0, Stroke::new(2.0, Color32::WHITE));
    ui.painter().circle_stroke(marker, 7.5, Stroke::new(1.0, Color32::from_black_alpha(120)));

    ui.add_space(6.0);
    let (strip, hue_response) = ui.allocate_exact_size(vec2(width, 14.0), Sense::click_and_drag());
    hue_response.widget_info(|| WidgetInfo::slider(true, hsva.h as f64, "Hue"));
    if let Some(p) = hue_response.interact_pointer_pos() {
        hsva.h = ((p.x - strip.left()) / strip.width()).clamp(0.0, 0.9999);
        changed = true;
    }
    if hue_response.has_focus() {
        let (l, r) = ui.input(|i| (i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight)));
        if l || r {
            hsva.h = (hsva.h + if r { 0.01 } else { -0.01 }).rem_euclid(1.0);
            changed = true;
        }
    }
    let mut mesh = Mesh::default();
    let segments = 36;
    for i in 0..=segments {
        let h = i as f32 / segments as f32;
        let c: Color32 = Hsva { h, s: 1.0, v: 1.0, a: 1.0 }.into();
        let x = strip.left() + h * strip.width();
        mesh.colored_vertex(pos2(x, strip.top()), c);
        mesh.colored_vertex(pos2(x, strip.bottom()), c);
    }
    for i in 0..segments as u32 {
        let k = i * 2;
        mesh.add_triangle(k, k + 1, k + 2);
        mesh.add_triangle(k + 1, k + 3, k + 2);
    }
    ui.painter().add(mesh);
    ui.painter().rect_stroke(strip, CornerRadius::same(4), Stroke::new(1.0, t.divider), StrokeKind::Inside);
    let x = strip.left() + hsva.h * strip.width();
    let knob = Rect::from_center_size(pos2(x, strip.center().y), vec2(6.0, 18.0));
    ui.painter().rect(
        knob,
        CornerRadius::same(3),
        Color32::WHITE,
        Stroke::new(1.0, Color32::from_black_alpha(120)),
        StrokeKind::Outside,
    );
    focus_ring(ui, &hue_response, strip, 4);

    if changed {
        *color = hsva.into();
        let c = *color;
        ui.data_mut(|d| d.insert_temp(id, (c, hsva)));
    }
    changed
}
