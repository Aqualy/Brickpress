//! The status bar: cursor position, selection, counts, mode hints, agent
//! connection and save state.

use crate::app::{BrickpressApp, View};
use crate::commands::Command;
use crate::icons;
use crate::theme::{self, space, tokens};
use brickpress_core::{Symmetry, Tool};
use egui::{Align, Layout, RichText, Ui};

pub fn show(app: &mut BrickpressApp, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    egui::Panel::bottom("statusbar")
        .exact_size(26.0)
        .frame(
            egui::Frame::new()
                .fill(t.panel)
                .inner_margin(egui::Margin::symmetric(12, 0))
                .stroke(egui::Stroke::new(1.0, t.divider)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = space::LG;
                let small = |text: String| RichText::new(text).size(11.5).color(t.text_secondary);
                let mono =
                    |text: String| RichText::new(text).font(egui::FontId::monospace(11.5)).color(t.text_secondary);
                if let Some(tab) = app.active_tab() {
                    let doc = tab.core.doc();
                    ui.label(mono(format!("{} × {} studs", doc.board.width, doc.board.height)));
                    if let Some((x, y)) = tab.canvas.cursor_cell {
                        ui.label(mono(format!("x {x}  y {y}")));
                    }
                    let count = doc.piece_count();
                    ui.label(small(format!("{count} piece{}", if count == 1 { "" } else { "s" })));
                    let selected = tab.core.selected.len();
                    if selected > 0 {
                        ui.label(RichText::new(format!("{selected} selected")).size(11.5).color(t.accent_text));
                    }
                    if doc.options.symmetry != Symmetry::Off {
                        ui.label(small(format!(
                            "{} Symmetry: {}",
                            icons::FLIP_HORIZONTAL_2,
                            doc.options.symmetry.label()
                        )));
                    }
                    if let Some(problem) = &tab.canvas.placement_problem {
                        if tab.core.tool == Tool::Place {
                            ui.label(
                                RichText::new(format!("{} {problem}", icons::TRIANGLE_ALERT))
                                    .size(11.5)
                                    .color(t.warning),
                            );
                        }
                    }
                } else {
                    ui.label(small("No project open".into()));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let zoom = app.active_tab().map(|tab| match tab.view {
                        View::TwoD => tab.canvas.zoom_percent(),
                        View::ThreeD => (tab.camera.scale * 100.0).round() as i32,
                    });
                    if let Some(zoom) = zoom {
                        let r = ui.add(egui::Label::new(mono(format!("{zoom}%"))).sense(egui::Sense::click()));
                        if r.on_hover_text("Click to zoom to fit").clicked() {
                            app.zoom_fit();
                        }
                    }
                    if app.agents.enabled() {
                        let n = app.agents.connections();
                        let color = if app.agents.active() {
                            t.accent_text
                        } else if n > 0 {
                            t.success
                        } else {
                            t.text_secondary
                        };
                        let label = match n {
                            0 => "Agents: waiting".to_string(),
                            1 => "Agent connected".to_string(),
                            n => format!("{n} agents connected"),
                        };
                        let r = ui.add(
                            egui::Label::new(RichText::new(format!("{} {label}", icons::BOT)).size(11.5).color(color))
                                .sense(egui::Sense::click()),
                        );
                        if r.on_hover_text("Agents connect with `brickpress mcp --live`. Click to turn off.").clicked()
                        {
                            let ctx = ui.ctx().clone();
                            app.run(Command::AgentConnections, &ctx);
                        }
                        if let Some(e) = &app.agents.error {
                            ui.label(RichText::new(e).size(11.5).color(t.danger));
                        }
                    }
                    if app.active_tab().and_then(|tab| tab.trace.as_ref()).is_some_and(|t| t.meta.visible) {
                        ui.label(small(format!("{} Tracing guide", icons::IMAGE)));
                    }
                    let _ = theme::CONTROL_HEIGHT;
                });
            });
        });
}
