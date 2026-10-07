fn main() {
    let commands = &[
        "select_file",
        "read_selected_file",
        "write_selected_file",
        "release_file",
        "read_state",
        "write_state",
        "quarantine_state",
        "write_trace_asset",
        "read_trace_asset",
        "prune_trace_assets",
        "confirm_action",
        "confirm_unsaved",
        "confirm_conflict",
        "set_document_title",
        "test_reset",
        "test_fixture",
        "test_queue_file",
        "test_queue_choice",
        "test_read_file",
        "test_read_bytes",
        "test_corrupt_file",
        "test_request_close",
        "test_request_quit",
        "test_pending_dialogs",
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(commands)),
    )
    .expect("build Tauri application permissions");
}
