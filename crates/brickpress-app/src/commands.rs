//! Every user action, defined once: label, icon, shortcut, availability.
//! Menus, toolbars, context menus, tooltips, the command palette and keyboard
//! handling all read from here, so they cannot drift apart.

use crate::app::{BrickpressApp, View, NO_DOCUMENT};
use crate::icons;
use brickpress_core::{Mode, Tool};
use egui::{Key, KeyboardShortcut, Modifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    New,
    Open,
    CloseTab,
    NextTab,
    PreviousTab,
    OpenPresetAsDocument,
    Save,
    SaveAs,
    Export,
    ImportPresets,
    ExportPresets,
    Quit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Delete,
    SelectAll,
    Deselect,
    SelectTool,
    PlaceTool,
    HandTool,
    Rotate,
    MirrorHorizontal,
    MirrorVertical,
    BringForward,
    SendBackward,
    DesignView,
    PrintView,
    View3D,
    ZoomIn,
    ZoomOut,
    ZoomFit,
    ZoomActual,
    ToggleGrid,
    ToggleLibrary,
    ToggleInspector,
    ResetLayout,
    CommandPalette,
    Shortcuts,
    AddPass,
    Reseed,
    ClearBoard,
    AddTracing,
    RemoveTracing,
    SavePreset,
    AgentConnections,
    About,
    Notices,
}

pub struct Meta {
    pub label: &'static str,
    pub icon: &'static str,
    pub shortcut: Option<KeyboardShortcut>,
    /// Extra words the command palette matches.
    pub keywords: &'static str,
}

const fn key(modifiers: Modifiers, key: Key) -> Option<KeyboardShortcut> {
    Some(KeyboardShortcut::new(modifiers, key))
}
const CMD: Modifiers = Modifiers::COMMAND;
const CMD_SHIFT: Modifiers = Modifiers { shift: true, ..Modifiers::COMMAND };
/// Control on every platform (tab switching is Ctrl+Tab on macOS as well).
const CTRL: Modifiers = Modifiers::CTRL;
const CTRL_SHIFT: Modifiers = Modifiers { shift: true, ..Modifiers::CTRL };
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;

impl Command {
    pub const ALL: [Command; 51] = [
        Command::New,
        Command::Open,
        Command::CloseTab,
        Command::NextTab,
        Command::PreviousTab,
        Command::OpenPresetAsDocument,
        Command::Save,
        Command::SaveAs,
        Command::Export,
        Command::ImportPresets,
        Command::ExportPresets,
        Command::Quit,
        Command::Undo,
        Command::Redo,
        Command::Cut,
        Command::Copy,
        Command::Paste,
        Command::Duplicate,
        Command::Delete,
        Command::SelectAll,
        Command::Deselect,
        Command::SelectTool,
        Command::PlaceTool,
        Command::HandTool,
        Command::Rotate,
        Command::MirrorHorizontal,
        Command::MirrorVertical,
        Command::BringForward,
        Command::SendBackward,
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
        Command::ResetLayout,
        Command::CommandPalette,
        Command::Shortcuts,
        Command::AddPass,
        Command::Reseed,
        Command::ClearBoard,
        Command::AddTracing,
        Command::RemoveTracing,
        Command::SavePreset,
        Command::AgentConnections,
        Command::About,
        Command::Notices,
    ];

    pub fn meta(self) -> Meta {
        let m = |label, icon, shortcut, keywords| Meta { label, icon, shortcut, keywords };
        match self {
            Command::New => m("New project", icons::FILE_PLUS, key(CMD, Key::N), "blank document"),
            Command::Open => m("Open…", icons::FOLDER_OPEN, key(CMD, Key::O), "load project file"),
            Command::CloseTab => m("Close Tab", icons::X, key(CMD, Key::W), "close document project window"),
            Command::NextTab => m("Next Tab", icons::CHEVRON_RIGHT, key(CTRL, Key::Tab), "switch document project"),
            Command::PreviousTab => {
                m("Previous Tab", icons::CHEVRON_LEFT, key(CTRL_SHIFT, Key::Tab), "switch document project")
            }
            Command::OpenPresetAsDocument => {
                m("Open Preset as New Project", icons::EXTERNAL_LINK, None, "composition library document tab")
            }
            Command::Save => m("Save", icons::SAVE, key(CMD, Key::S), "write project"),
            Command::SaveAs => m("Save As…", icons::SAVE, key(CMD_SHIFT, Key::S), "copy duplicate project"),
            Command::Export => m("Export…", icons::UPLOAD, key(CMD, Key::E), "png svg image picture"),
            Command::ImportPresets => m("Import Presets…", icons::DOWNLOAD, None, "library compositions"),
            Command::ExportPresets => m("Export Presets…", icons::UPLOAD, None, "library compositions backup"),
            Command::Quit => m("Quit", icons::LOG_OUT, key(CMD, Key::Q), "exit close"),
            Command::Undo => m("Undo", icons::UNDO_2, key(CMD, Key::Z), "back"),
            Command::Redo => m("Redo", icons::REDO_2, key(CMD_SHIFT, Key::Z), "again"),
            Command::Cut => m("Cut", icons::SCISSORS, key(CMD, Key::X), "clipboard"),
            Command::Copy => m("Copy", icons::COPY, key(CMD, Key::C), "clipboard"),
            Command::Paste => m("Paste", icons::CLIPBOARD_PASTE, key(CMD, Key::V), "clipboard"),
            Command::Duplicate => m("Duplicate", icons::COPY, key(CMD, Key::D), "clone copy"),
            Command::Delete => m("Delete", icons::TRASH_2, key(NONE, Key::Delete), "remove erase"),
            Command::SelectAll => m("Select All", icons::SQUARE_DASHED_MOUSE_POINTER, key(CMD, Key::A), "everything"),
            Command::Deselect => m("Deselect", icons::X, key(NONE, Key::Escape), "clear selection"),
            Command::SelectTool => m("Select", icons::MOUSE_POINTER_2, key(NONE, Key::V), "tool move arrow"),
            Command::PlaceTool => m("Place", icons::STAMP, key(NONE, Key::B), "tool brush paint add"),
            Command::HandTool => m("Hand", icons::HAND, key(NONE, Key::H), "tool pan scroll"),
            Command::Rotate => m("Rotate", icons::ROTATE_CW, key(NONE, Key::R), "turn quarter"),
            Command::MirrorHorizontal => {
                m("Mirror Horizontally", icons::FLIP_HORIZONTAL_2, key(SHIFT, Key::H), "flip reflect")
            }
            Command::MirrorVertical => {
                m("Mirror Vertically", icons::FLIP_VERTICAL_2, key(SHIFT, Key::V), "flip reflect")
            }
            Command::BringForward => {
                m("Bring Forward", icons::BRING_TO_FRONT, key(CMD, Key::CloseBracket), "order above")
            }
            Command::SendBackward => m("Send Backward", icons::SEND_TO_BACK, key(CMD, Key::OpenBracket), "order below"),
            Command::DesignView => m("Design", icons::PEN_TOOL, key(NONE, Key::Num1), "view mode vector"),
            Command::PrintView => {
                m("Print Preview", icons::STAMP, key(NONE, Key::Num2), "view mode letterpress impression")
            }
            Command::View3D => m("3D View", icons::BOX, key(NONE, Key::Num3), "view mode three dimensional orbit"),
            Command::ZoomIn => m("Zoom In", icons::ZOOM_IN, key(CMD, Key::Plus), "magnify"),
            Command::ZoomOut => m("Zoom Out", icons::ZOOM_OUT, key(CMD, Key::Minus), "magnify"),
            Command::ZoomFit => m("Zoom to Fit", icons::SCAN, key(CMD, Key::Num0), "fit board whole"),
            Command::ZoomActual => m("Actual Size", icons::MAXIMIZE, key(CMD, Key::Num1), "100% zoom"),
            Command::ToggleGrid => m("Show Grid", icons::GRID_3X3, key(NONE, Key::G), "studs guide"),
            Command::ToggleLibrary => m("Show Library", icons::PANEL_LEFT, key(CMD, Key::Backslash), "pieces sidebar"),
            Command::ToggleInspector => {
                m("Show Inspector", icons::PANEL_RIGHT, key(CMD_SHIFT, Key::Backslash), "properties sidebar")
            }
            Command::ResetLayout => m("Reset Layout", icons::LAYOUT_GRID, None, "panels sidebars"),
            Command::CommandPalette => m("Command Palette…", icons::COMMAND, key(CMD, Key::K), "search actions run"),
            Command::Shortcuts => m("Keyboard Shortcuts", icons::KEYBOARD, key(NONE, Key::Questionmark), "help keys"),
            Command::AddPass => m("New Ink Pass", icons::PLUS, None, "layer colour"),
            Command::Reseed => m("Reseed Print", icons::DICES, None, "random variation impression"),
            Command::ClearBoard => m("Clear Board…", icons::TRASH_2, None, "remove all pieces"),
            Command::AddTracing => m("Add Tracing Image…", icons::IMAGE_PLUS, None, "guide reference underlay"),
            Command::RemoveTracing => m("Remove Tracing Image", icons::IMAGE, None, "guide reference underlay"),
            Command::SavePreset => m("Save Selection as Preset…", icons::BOOKMARK_PLUS, None, "composition library"),
            Command::AgentConnections => m("Allow Agent Connections", icons::BOT, None, "mcp ai live automation"),
            Command::About => m("About Brickpress", icons::INFO, None, "version"),
            Command::Notices => m("Third-Party Notices", icons::FILE_CODE, None, "licenses licences"),
        }
    }

    /// Single-key shortcuts act only on the canvas, never while typing.
    pub fn is_plain_key(self) -> bool {
        self.meta().shortcut.is_some_and(|s| s.modifiers.is_none() || s.modifiers == SHIFT)
    }

    pub fn shortcut_text(self, ctx: &egui::Context) -> Option<String> {
        let s = self.meta().shortcut?;
        let mac = ctx.os() == egui::os::OperatingSystem::Mac;
        let text = s.format(&egui::ModifierNames::NAMES, mac);
        Some(match s.logical_key {
            Key::Questionmark => "?".into(),
            Key::Delete => {
                if mac {
                    "⌫".into()
                } else {
                    "Delete".into()
                }
            }
            _ => text,
        })
    }

    /// Commands that work without an open document.
    pub fn is_global(self) -> bool {
        matches!(
            self,
            Command::New
                | Command::Open
                | Command::NextTab
                | Command::PreviousTab
                | Command::OpenPresetAsDocument
                | Command::ImportPresets
                | Command::ExportPresets
                | Command::Quit
                | Command::ToggleGrid
                | Command::ToggleLibrary
                | Command::ToggleInspector
                | Command::ResetLayout
                | Command::CommandPalette
                | Command::Shortcuts
                | Command::AgentConnections
                | Command::About
                | Command::Notices
        )
    }

    /// `Ok` when the command can run now, or a short reason why not.
    pub fn availability(self, app: &BrickpressApp) -> Result<(), &'static str> {
        let check = |ok: bool, reason| if ok { Ok(()) } else { Err(reason) };
        if !self.is_global() && !app.has_document() {
            return Err(NO_DOCUMENT);
        }
        let core = app.core();
        let has_selection = !core.selected_pieces().is_empty();
        let in_3d = app.view() == View::ThreeD;
        match self {
            Command::NextTab | Command::PreviousTab => check(app.tabs.len() > 1, "Only one project is open"),
            Command::OpenPresetAsDocument => {
                check(app.preset_target().is_some(), "Choose a preset in the library first")
            }
            Command::Undo => check(core.can_undo(), "Nothing to undo"),
            Command::Redo => check(core.can_redo(), "Nothing to redo"),
            Command::Cut
            | Command::Copy
            | Command::Duplicate
            | Command::Delete
            | Command::BringForward
            | Command::SendBackward => check(has_selection, "Select pieces first"),
            Command::Paste => check(core.has_clipboard(), "Copy pieces first"),
            Command::Deselect => check(
                !core.selected.is_empty() || core.active_preset_id.is_some() || core.tool == Tool::Place,
                "Nothing is selected",
            ),
            Command::SelectAll => check(core.doc().piece_count() > 0, "The board is empty"),
            Command::Rotate | Command::MirrorHorizontal | Command::MirrorVertical => {
                check(has_selection || core.tool == Tool::Place, "Select pieces or choose a piece to place")
            }
            Command::PlaceTool => check(core.mode == Mode::Design || in_3d, "Placing works in Design and 3D"),
            Command::ZoomActual | Command::ZoomIn | Command::ZoomOut | Command::ZoomFit => Ok(()),
            Command::ToggleGrid => check(!in_3d || !app.has_document(), "The grid is part of the 2D views"),
            Command::ExportPresets => check(!core.presets.is_empty(), "Save a preset first"),
            Command::RemoveTracing => check(app.active_tab().is_some_and(|t| t.trace.is_some()), "No tracing image"),
            Command::SavePreset => check(core.doc().piece_count() > 0, "Place some pieces first"),
            Command::ClearBoard => check(core.doc().piece_count() > 0, "The board is already empty"),
            Command::Reseed => check(core.doc().piece_count() > 0, "Place some pieces first"),
            _ => Ok(()),
        }
    }

    /// Whether the command shows as checked (toggles and modes).
    pub fn checked(self, app: &BrickpressApp) -> Option<bool> {
        let core = app.core();
        let open = app.has_document();
        Some(match self {
            Command::SelectTool => open && core.tool == Tool::Select,
            Command::PlaceTool => open && core.tool == Tool::Place,
            Command::HandTool => open && core.tool == Tool::Hand,
            Command::DesignView => open && app.view() == View::TwoD && core.mode == Mode::Design,
            Command::PrintView => open && app.view() == View::TwoD && core.mode == Mode::Print,
            Command::View3D => open && app.view() == View::ThreeD,
            Command::ToggleGrid => app.show_grid,
            Command::ToggleLibrary => app.layout.library,
            Command::ToggleInspector => app.layout.inspector,
            Command::AgentConnections => app.agents.enabled(),
            _ => return None,
        })
    }
}

/// Handles keyboard shortcuts for the whole window.
pub fn handle_shortcuts(app: &mut BrickpressApp, ctx: &egui::Context) {
    let typing = ctx.text_edit_focused();
    let modal = app.modal_open();
    // egui matches shortcuts "logically" (an extra Shift still matches), so
    // the most specific shortcuts must be tried first: Ctrl+Shift+Z before Ctrl+Z.
    let mut ordered: Vec<(Command, KeyboardShortcut)> =
        Command::ALL.iter().filter_map(|c| c.meta().shortcut.map(|s| (*c, s))).collect();
    let weight = |m: Modifiers| m.shift as u8 + m.alt as u8 + (m.command || m.ctrl) as u8;
    ordered.sort_by_key(|(_, s)| std::cmp::Reverse(weight(s.modifiers)));
    for (command, shortcut) in ordered {
        if (typing || modal)
            && (command.is_plain_key()
                || matches!(
                    command,
                    Command::Undo | Command::Redo | Command::Cut | Command::Copy | Command::Paste | Command::SelectAll
                ))
        {
            continue;
        }
        if modal && command != Command::Quit {
            continue;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
            app.run(command, ctx);
        }
    }
    // eframe turns Ctrl+C/X/V into clipboard events; outside text fields they
    // act on pieces.
    if !typing && !modal {
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Copy => app.run(Command::Copy, ctx),
                egui::Event::Cut => app.run(Command::Cut, ctx),
                egui::Event::Paste(_) => app.run(Command::Paste, ctx),
                _ => {}
            }
        }
    }
    // Alternatives that 1.x also accepted.
    if !typing && !modal {
        if ctx.input_mut(|i| i.consume_key(CMD, Key::Y)) {
            app.run(Command::Redo, ctx);
        }
        if ctx.input_mut(|i| i.consume_key(NONE, Key::Backspace)) {
            app.run(Command::Delete, ctx);
        }
        if ctx.input_mut(|i| i.consume_key(CMD, Key::Equals)) {
            app.run(Command::ZoomIn, ctx);
        }
        if ctx.input_mut(|i| i.consume_key(NONE, Key::Slash)) {
            app.run(Command::Shortcuts, ctx);
        }
    }
    // Ctrl+Page Down / Page Up switch tabs too, as in browsers and Photoshop.
    if !modal {
        if ctx.input_mut(|i| i.consume_key(CTRL, Key::PageDown)) {
            app.run(Command::NextTab, ctx);
        }
        if ctx.input_mut(|i| i.consume_key(CTRL, Key::PageUp)) {
            app.run(Command::PreviousTab, ctx);
        }
    }
}
