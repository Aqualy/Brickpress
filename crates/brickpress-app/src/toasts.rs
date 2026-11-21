//! Non-blocking notifications. They never take focus, pause while hovered,
//! and can be dismissed. Errors stay longer than confirmations.

use crate::icons;
use crate::theme::{self, tokens};
use egui::{vec2, Align2, CornerRadius, FontId, Id, Stroke, StrokeKind};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

struct Toast {
    id: u64,
    kind: ToastKind,
    text: String,
    shown: Instant,
    paused: Duration,
}

#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
    next: u64,
}

impl Toasts {
    pub fn push(&mut self, kind: ToastKind, text: impl Into<String>) {
        let text = text.into();
        // Repeated messages refresh instead of stacking.
        if let Some(existing) = self.items.iter_mut().find(|t| t.text == text) {
            existing.shown = Instant::now();
            existing.paused = Duration::ZERO;
            return;
        }
        self.next += 1;
        self.items.push(Toast { id: self.next, kind, text, shown: Instant::now(), paused: Duration::ZERO });
        if self.items.len() > 4 {
            self.items.remove(0);
        }
    }

    pub fn latest(&self) -> Option<&str> {
        self.items.last().map(|t| t.text.as_str())
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if self.items.is_empty() {
            return;
        }
        let t = tokens(ctx);
        let screen = ctx.content_rect();
        // Above the canvas tool dock and the status bar.
        let mut y = screen.bottom() - 92.0;
        let dt = Duration::from_secs_f32(ctx.input(|i| i.stable_dt).min(0.1));
        let mut dismiss = Vec::new();
        for toast in self.items.iter_mut().rev() {
            let lifetime = match toast.kind {
                ToastKind::Error => Duration::from_secs(8),
                _ => Duration::from_secs(4),
            };
            let area = egui::Area::new(Id::unique(("toast", toast.id)))
                .order(egui::Order::Tooltip)
                .anchor(Align2::CENTER_BOTTOM, vec2(0.0, y - screen.bottom()))
                .interactable(true);
            let response = area.show(ctx, |ui| {
                egui::Frame::new()
                    .fill(t.surface)
                    .stroke(Stroke::new(1.0, t.divider))
                    .corner_radius(CornerRadius::same(8))
                    .shadow(ui.visuals().popup_shadow)
                    .inner_margin(egui::Margin { left: 12, right: 6, top: 6, bottom: 6 })
                    .show(ui, |ui| {
                        ui.set_max_width(520.0);
                        ui.horizontal(|ui| {
                            let (icon, color) = match toast.kind {
                                ToastKind::Info => (icons::INFO, t.text_secondary),
                                ToastKind::Success => (icons::CIRCLE_CHECK, t.success),
                                ToastKind::Error => (icons::TRIANGLE_ALERT, t.danger),
                            };
                            ui.label(
                                egui::RichText::new(icon).font(FontId::new(15.0, theme::icon_family())).color(color),
                            );
                            ui.add(egui::Label::new(egui::RichText::new(&toast.text).color(t.text)).wrap());
                            if crate::widgets::icon_button_sized(ui, icons::X, "Dismiss", None, false, 24.0).clicked() {
                                dismiss.push(toast.id);
                            }
                        });
                    });
            });
            let r = response.response;
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Status, true, &toast.text));
            if r.contains_pointer() {
                toast.paused += dt;
            }
            if toast.shown.elapsed().saturating_sub(toast.paused) > lifetime {
                dismiss.push(toast.id);
            }
            let _ = StrokeKind::Inside;
            y -= r.rect.height() + 8.0;
        }
        self.items.retain(|t| !dismiss.contains(&t.id));
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}
