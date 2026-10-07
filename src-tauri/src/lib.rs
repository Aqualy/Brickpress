mod commands;
mod files;
#[cfg(feature = "desktop-e2e")]
mod testing;

use std::sync::Mutex;
use tauri::Manager;

#[cfg(all(feature = "desktop-e2e", not(debug_assertions)))]
compile_error!("desktop-e2e is debug-only and must never be included in an installer");

pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(not(feature = "desktop-e2e"))]
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED,
                )
                .build(),
        );
    #[cfg(feature = "desktop-e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());

    let builder = builder.plugin(tauri_plugin_dialog::init()).setup(|app| {
        #[cfg(not(feature = "desktop-e2e"))]
        let root = app.path().app_data_dir()?;
        #[cfg(feature = "desktop-e2e")]
        let root = std::env::var_os("BRICKPRESS_TEST_DATA_DIR")
            .map(std::path::PathBuf::from)
            .ok_or("desktop-e2e requires an isolated BRICKPRESS_TEST_DATA_DIR")?;
        app.manage(Mutex::new(
            files::FileStore::new(root.clone())
                .unwrap_or_else(|_| files::FileStore::uninitialized(root.clone())),
        ));
        #[cfg(feature = "desktop-e2e")]
        app.manage(Mutex::new(testing::TestDialogs::default()));
        let window = tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?;
        #[cfg(feature = "desktop-e2e")]
        let window = window.data_directory(root.join("webview"));
        window.build()?;
        Ok(())
    });

    #[cfg(not(feature = "desktop-e2e"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::select_file,
        commands::read_selected_file,
        commands::write_selected_file,
        commands::release_file,
        commands::read_state,
        commands::write_state,
        commands::quarantine_state,
        commands::write_trace_asset,
        commands::read_trace_asset,
        commands::prune_trace_assets,
        commands::confirm_action,
        commands::confirm_unsaved,
        commands::confirm_conflict,
        commands::set_document_title
    ]);
    #[cfg(feature = "desktop-e2e")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::select_file,
        commands::read_selected_file,
        commands::write_selected_file,
        commands::release_file,
        commands::read_state,
        commands::write_state,
        commands::quarantine_state,
        commands::write_trace_asset,
        commands::read_trace_asset,
        commands::prune_trace_assets,
        commands::confirm_action,
        commands::confirm_unsaved,
        commands::confirm_conflict,
        commands::set_document_title,
        testing::test_reset,
        testing::test_fixture,
        testing::test_queue_file,
        testing::test_queue_choice,
        testing::test_read_file,
        testing::test_read_bytes,
        testing::test_corrupt_file,
        testing::test_request_close,
        testing::test_request_quit,
        testing::test_pending_dialogs
    ]);
    builder
        .build(tauri::generate_context!())
        .expect("building Brickpress")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if let Some(window) = app.get_webview_window("main") {
                    // Native application Quit (including Cmd+Q) must use the
                    // same asynchronous protection as the window close button.
                    // The controller destroys the window only after approval
                    // and flushing; a later exit with no main window is safe.
                    api.prevent_exit();
                    let _ = window.close();
                }
            }
        });
}
