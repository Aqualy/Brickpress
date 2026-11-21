//! End-to-end UI tests: the real app, rendered with wgpu, driven through
//! AccessKit labels. Set BRICKPRESS_ARTIFACTS to keep screenshots.

use brickpress_app::app::{BrickpressApp, InspectorTab, LibraryTab, Modal, Pending, View};
use brickpress_app::commands::Command;
use egui::accesskit::Role;
use egui::{Key, Modifiers, Theme};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;

/// GPU devices are created one test at a time.
static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn harness(theme: Theme, data: &std::path::Path) -> Harness<'static, BrickpressApp> {
    let data = data.to_path_buf();
    Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_theme(theme)
        .wgpu()
        .build_eframe(move |cc| BrickpressApp::with_data_dir(cc, None, data))
}

fn save(h: &mut Harness<'_, BrickpressApp>, name: &str) {
    if let Some(dir) = std::env::var_os("BRICKPRESS_ARTIFACTS") {
        std::fs::create_dir_all(&dir).unwrap();
        h.run_steps(2);
        if let Ok(image) = h.render() {
            image.save(std::path::Path::new(&dir).join(format!("{name}.png"))).unwrap();
        }
    }
}

fn run(h: &mut Harness<'_, BrickpressApp>, command: Command) {
    let ctx = h.ctx.clone();
    h.state_mut().run(command, &ctx);
    h.run_steps(3);
}

#[test]
fn first_run_shows_the_starter_and_every_panel() {
    let _gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(Theme::Light, dir.path());
    h.run_steps(4);
    for label in ["File", "Edit", "Arrange", "View", "Help", "Export", "Pieces", "Presets", "Design", "Inks", "Print"] {
        assert!(h.query_all_by_label(label).next().is_some(), "missing {label}");
    }
    assert!(h.state().core().doc().piece_count() > 30, "the starter composition is shown on first run");
    save(&mut h, "light-design");
    for tab in [InspectorTab::Inks, InspectorTab::Print, InspectorTab::Export] {
        h.state_mut().inspector_tab = tab;
        h.run_steps(3);
        save(&mut h, &format!("light-tab-{tab:?}").to_lowercase());
    }
}

#[test]
fn dark_theme_print_and_3d_views_render() {
    let _gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(Theme::Dark, dir.path());
    h.run_steps(4);
    save(&mut h, "dark-design");
    run(&mut h, Command::PrintView);
    assert_eq!(h.state().core().mode, brickpress_core::Mode::Print);
    save(&mut h, "dark-print");
    run(&mut h, Command::View3D);
    assert_eq!(h.state().view(), View::ThreeD);
    save(&mut h, "dark-3d");
}

#[test]
fn place_a_piece_from_the_library_and_undo_it() {
    let _gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(Theme::Light, dir.path());
    h.run_steps(3);
    run(&mut h, Command::New);
    assert_eq!(h.state().core().doc().piece_count(), 0);
    h.get_all_by_label("Tile 1×1").next().unwrap().click();
    h.run_steps(2);
    assert_eq!(h.state().core().tool, brickpress_core::Tool::Place);
    // Click the middle of the board.
    let board = h.get_by_label_contains("by 16 studs").rect().center();
    h.hover_at(board);
    h.run_steps(2);
    save(&mut h, "placing-ghost");
    h.drag_at(board);
    h.drop_at(board);
    h.run_steps(3);
    assert_eq!(h.state().core().doc().piece_count(), 1, "one click places one tile");
    assert!(h.state().core().dirty);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert_eq!(h.state().core().doc().piece_count(), 0, "Ctrl+Z undoes the placement");
}

/// A clean project file named `name` in `dir`.
fn project(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(format!("{name}.brickpress.json"));
    let mut doc = brickpress_core::persistence::document::create_document(false);
    doc.name = name.into();
    std::fs::write(&path, brickpress_core::persistence::document::serialize_document(&doc)).unwrap();
    path
}

fn tab<'a>(h: &'a Harness<'_, BrickpressApp>, name: &'a str) -> egui_kittest::Node<'a> {
    h.get_by_role_and_label(Role::Tab, name)
}

#[test]
fn documents_open_in_tabs_and_switch() {
    let _gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut h = harness(Theme::Light, dir.path());
    h.run_steps(3);
    h.state_mut().open_path(project(files.path(), "alpha"));
    h.run_steps(3);
    assert_eq!(h.state().tabs.len(), 1, "the untouched starter makes way for the opened file");
    h.state_mut().open_path(project(files.path(), "beta"));
    h.run_steps(3);
    assert_eq!((h.state().tabs.len(), h.state().active), (2, 1));
    // An edit in beta stays in beta.
    h.state_mut().core_mut().reseed();
    h.run_steps(3);
    assert!(h.state().tabs[1].dirty() && !h.state().tabs[0].dirty());
    run(&mut h, Command::New);
    assert_eq!((h.state().tabs.len(), h.state().active), (3, 2), "New always adds a tab");
    assert_eq!(h.state().core().doc().piece_count(), 0);
    save(&mut h, "tabs-light");

    tab(&h, "alpha.brickpress.json").click();
    h.run_steps(3);
    assert_eq!(h.state().active, 0);
    assert_eq!(h.state().core().doc().name, "alpha");
    h.key_press_modifiers(Modifiers::CTRL, Key::Tab);
    h.run_steps(2);
    assert_eq!(h.state().active, 1, "Ctrl+Tab goes to the next tab");
    h.key_press_modifiers(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab);
    h.run_steps(2);
    assert_eq!(h.state().active, 0, "Ctrl+Shift+Tab goes back");
    h.key_press_modifiers(Modifiers::CTRL, Key::PageUp);
    h.run_steps(2);
    assert_eq!(h.state().active, 2, "and wraps around");
    // Opening a file that is already open activates its tab.
    h.state_mut().open_path(files.path().join("beta.brickpress.json"));
    h.run_steps(2);
    assert_eq!((h.state().tabs.len(), h.state().active), (3, 1));
}

#[test]
fn closing_a_tab_with_unsaved_changes_asks_first() {
    let _gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(Theme::Light, dir.path());
    h.run_steps(3);
    h.state_mut().core_mut().reseed();
    h.run_steps(3);
    assert!(h.state().dirty());
    run(&mut h, Command::New);
    tab(&h, "A study in bloom").click();
    h.run_steps(3);
    run(&mut h, Command::CloseTab);
    assert!(matches!(h.state().modal, Some(Modal::Unsaved(Pending::CloseTab(_)))));
    save(&mut h, "unsaved-dialog");
    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert!(h.state().modal.is_none());
    assert_eq!(h.state().tabs.len(), 2, "cancel keeps the tab");
    assert!(h.state().core().doc().piece_count() > 0, "and its document");
    h.get_by_label("Close A study in bloom").click();
    h.run_steps(3);
    assert!(matches!(h.state().modal, Some(Modal::Unsaved(_))), "the close button asks too");
    h.get_by_label("Don’t Save").click();
    h.run_steps(3);
    assert_eq!(h.state().tabs.len(), 1);
    assert_eq!(h.state().core().doc().piece_count(), 0, "the other tab is shown");
    // A clean tab closes without asking, leaving the empty state.
    run(&mut h, Command::CloseTab);
    assert!(!h.state().has_document());
    assert!(h.query_all_by_label("No project open").next().is_some());
    assert!(Command::Undo.availability(h.state()).is_err(), "document commands are disabled");
    let files = tempfile::tempdir().unwrap();
    h.state_mut().recent = vec![project(files.path(), "Poster"), project(files.path(), "Coasters")];
    h.run_steps(2);
    save(&mut h, "empty-state");
    h.get_by_label("Poster.brickpress.json").click();
    h.run_steps(3);
    assert_eq!(h.state().core().doc().name, "Poster", "recent projects open from the empty state");
    run(&mut h, Command::CloseTab);
    h.get_by_label("New Project").click();
    h.run_steps(3);
    assert_eq!(h.state().tabs.len(), 1);
}

#[test]
fn a_preset_opens_as_a_new_project() {
    let _gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(Theme::Dark, dir.path());
    h.run_steps(3);
    h.state_mut().core_mut().reseed();
    assert!(h.state_mut().core_mut().save_preset("Garden", false));
    h.state_mut().library_tab = LibraryTab::Presets;
    h.run_steps(3);
    let (w, h_studs) = {
        let preset = &h.state().core().presets[0];
        (preset.width, preset.height)
    };
    h.get_by_label("Open as new project").click();
    h.run_steps(3);
    assert_eq!((h.state().tabs.len(), h.state().active), (2, 1), "a tab of its own");
    let doc = h.state().core().doc().clone();
    assert_eq!(doc.name, "Garden");
    assert_eq!((doc.board.width, doc.board.height), (w + 4, h_studs + 4));
    assert_eq!(doc.piece_count(), h.state().tabs[0].core.doc().piece_count());
    assert!(h.state().dirty() && h.state().tabs[1].file.is_none(), "unsaved and never saved");
    assert_eq!(h.state().core().presets.len(), 1, "the library follows the active tab");
    save(&mut h, "tabs-dark-preset");
    let starter = tab(&h, "A study in bloom").rect().center();
    h.hover_at(starter);
    h.run_steps(2);
    save(&mut h, "tabs-dark-hover");
    h.get_by_label("Close A study in bloom").click();
    h.run_steps(3);
    assert!(matches!(h.state().modal, Some(Modal::Unsaved(_))), "an inactive tab's close button asks too");
    assert_eq!(h.state().active, 0, "and shows the tab it asks about");
}

#[test]
fn the_command_palette_finds_and_runs_commands() {
    let _gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(Theme::Dark, dir.path());
    h.run_steps(3);
    h.key_press_modifiers(Modifiers::COMMAND, Key::K);
    h.run_steps(3);
    assert!(matches!(h.state().modal, Some(Modal::Palette { .. })));
    h.event(egui::Event::Text("export".into()));
    h.run_steps(3);
    save(&mut h, "palette");
    h.key_press(Key::Enter);
    h.run_steps(3);
    assert!(h.state().modal.is_none());
    assert_eq!(h.state().inspector_tab, InspectorTab::Export);
}

#[test]
fn keyboard_tools_and_selection() {
    let _gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(Theme::Light, dir.path());
    h.run_steps(3);
    h.key_press(Key::B);
    h.run_steps(2);
    assert_eq!(h.state().core().tool, brickpress_core::Tool::Place);
    h.key_press(Key::V);
    h.run_steps(2);
    assert_eq!(h.state().core().tool, brickpress_core::Tool::Select);
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run_steps(2);
    let count = h.state().core().doc().piece_count();
    assert_eq!(h.state().core().selected.len(), count);
    save(&mut h, "select-all");
    h.key_press(Key::Escape);
    h.run_steps(2);
    assert!(h.state().core().selected.is_empty());
    h.key_press(Key::Num2);
    h.run_steps(2);
    assert_eq!(h.state().core().mode, brickpress_core::Mode::Print);
}

#[test]
fn a_live_agent_edits_the_open_window_and_the_person_can_undo_it() {
    let _gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    // A private socket so a real Brickpress window is never involved.
    std::env::set_var("BRICKPRESS_SOCKET", format!("brickpress-test-{}.sock", std::process::id()));
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(Theme::Light, dir.path());
    h.run_steps(2);
    run(&mut h, Command::New);
    run(&mut h, Command::AgentConnections);
    assert!(h.state().agents.enabled());
    let agent = std::thread::spawn(|| {
        let mut client = (0..50)
            .find_map(|_| {
                std::thread::sleep(std::time::Duration::from_millis(50));
                brickpress_mcp::live::LiveClient::connect().ok()
            })
            .expect("connect to the running window");
        let op: brickpress_core::ops::Op = serde_json::from_value(serde_json::json!({
            "op": "placePieces", "pieces": [{"piece": "3070", "x": 2, "y": 2}, {"piece": "3069", "x": 4, "y": 2}]
        }))
        .unwrap();
        let placed = client.call(op).unwrap();
        let info = client.call(brickpress_core::ops::Op::Info).unwrap();
        (placed, info)
    });
    let started = std::time::Instant::now();
    while !agent.is_finished() {
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "the agent did not finish");
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let (placed, info) = agent.join().unwrap();
    assert_eq!(placed["placed"], 2);
    assert_eq!(info["pieces"], 2);
    h.run_steps(2);
    save(&mut h, "live-agent");
    assert_eq!(h.state().core().doc().piece_count(), 2, "the window shows the agent's pieces");
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(2);
    assert_eq!(h.state().core().doc().piece_count(), 0, "one Ctrl+Z undoes the agent's step");
    assert!(h.state().tabs[0].core.doc().piece_count() > 30, "agents edit only the active tab");
    run(&mut h, Command::AgentConnections);
    assert!(!h.state().agents.enabled());
}
