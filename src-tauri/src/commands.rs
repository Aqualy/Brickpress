use crate::files::{BridgeError, FileKind, FileStore, Result, SelectedFile};
use std::sync::{Mutex, MutexGuard};
use tauri::{
    ipc::{InvokeBody, Request, Response},
    Manager, State, WebviewWindow,
};
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};

pub fn main_only(window: &WebviewWindow) -> Result<()> {
    if window.label() != "main" {
        return Err(BridgeError::new(
            "forbidden",
            "Only the main editor may use this command.",
        ));
    }
    Ok(())
}
pub fn lock(store: &Mutex<FileStore>) -> Result<MutexGuard<'_, FileStore>> {
    store
        .lock()
        .map_err(|_| BridgeError::new("io", "Native storage is unavailable. Restart Brickpress."))
}
fn raw<'a>(request: &'a Request<'_>) -> Result<&'a [u8]> {
    match request.body() {
        InvokeBody::Raw(bytes) => Ok(bytes),
        _ => Err(BridgeError::new(
            "invalid",
            "Expected a binary IPC payload.",
        )),
    }
}

#[tauri::command]
pub async fn select_file(
    window: WebviewWindow,
    kind: FileKind,
    save: bool,
    default_name: Option<String>,
) -> Result<Option<SelectedFile>> {
    main_only(&window)?;
    #[cfg(feature = "desktop-e2e")]
    if let Some(path) = crate::testing::take_file(&window)? {
        return path
            .map(|path| lock(&window.state::<Mutex<FileStore>>())?.select(path, kind, save))
            .transpose();
    }
    let dialog_window = window.clone();
    let path = tauri::async_runtime::spawn_blocking(move || {
        let (name, extensions) = kind.filter();
        let mut dialog = dialog_window
            .dialog()
            .file()
            .set_parent(&dialog_window)
            .add_filter(name, extensions)
            .set_title(if save {
                "Save — Brickpress"
            } else {
                "Open — Brickpress"
            });
        if let Some(name) = default_name {
            dialog = dialog.set_file_name(name);
        }
        if save {
            dialog.blocking_save_file()
        } else {
            dialog.blocking_pick_file()
        }
    })
    .await
    .map_err(|e| BridgeError::new("dialog", e.to_string()))?;
    match path {
        None => Ok(None),
        Some(path) => {
            let path = path
                .into_path()
                .map_err(|e| BridgeError::new("invalid", e.to_string()))?;
            let selected = lock(&window.state::<Mutex<FileStore>>())?.select(path, kind, save)?;
            Ok(Some(selected))
        }
    }
}
#[tauri::command]
pub async fn read_selected_file(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    token: String,
) -> Result<Response> {
    main_only(&window)?;
    Ok(Response::new(lock(&store)?.read(&token)?))
}
#[tauri::command]
pub async fn write_selected_file(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    request: Request<'_>,
) -> Result<()> {
    main_only(&window)?;
    let token = request
        .headers()
        .get("x-brickpress-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| BridgeError::new("invalid-handle", "Missing selected file token."))?;
    lock(&store)?.write(token, raw(&request)?)
}
#[tauri::command]
pub async fn release_file(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    token: String,
) -> Result<()> {
    main_only(&window)?;
    lock(&store)?.release(&token);
    Ok(())
}
#[tauri::command]
pub async fn read_state(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    key: String,
) -> Result<Option<String>> {
    main_only(&window)?;
    lock(&store)?.read_state(&key)
}
#[tauri::command]
pub async fn write_state(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    key: String,
    value: Option<String>,
) -> Result<()> {
    main_only(&window)?;
    lock(&store)?.write_state(&key, value.as_deref())
}
#[tauri::command]
pub async fn quarantine_state(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    key: String,
) -> Result<()> {
    main_only(&window)?;
    lock(&store)?.quarantine(&key)
}
#[tauri::command]
pub async fn write_trace_asset(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    request: Request<'_>,
) -> Result<String> {
    main_only(&window)?;
    lock(&store)?.write_asset(raw(&request)?)
}
#[tauri::command]
pub async fn read_trace_asset(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    asset_id: String,
) -> Result<Response> {
    main_only(&window)?;
    Ok(Response::new(lock(&store)?.read_asset(&asset_id)?))
}
#[tauri::command]
pub async fn prune_trace_assets(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    keep: Option<String>,
) -> Result<()> {
    main_only(&window)?;
    lock(&store)?.prune_assets(keep.as_deref())
}
#[tauri::command]
pub async fn confirm_action(window: WebviewWindow, message: String) -> Result<bool> {
    main_only(&window)?;
    #[cfg(feature = "desktop-e2e")]
    if let Some(choice) = crate::testing::take_choice(&window)? {
        return Ok(choice == "ok" || choice == "save");
    }
    tauri::async_runtime::spawn_blocking(move || {
        window
            .dialog()
            .message(message)
            .parent(&window)
            .title("Brickpress")
            .buttons(MessageDialogButtons::OkCancel)
            .blocking_show()
    })
    .await
    .map_err(|e| BridgeError::new("dialog", e.to_string()))
}
#[tauri::command]
pub async fn confirm_unsaved(window: WebviewWindow, name: String) -> Result<String> {
    main_only(&window)?;
    #[cfg(feature = "desktop-e2e")]
    if let Some(choice) = crate::testing::take_choice(&window)? {
        return Ok(choice);
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        window
            .dialog()
            .message(format!("Save changes to “{name}” before continuing?"))
            .parent(&window)
            .title("Unsaved document — Brickpress")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::YesNoCancelCustom(
                "Save".into(),
                "Discard".into(),
                "Cancel".into(),
            ))
            .blocking_show_with_result()
    })
    .await
    .map_err(|e| BridgeError::new("dialog", e.to_string()))?;
    Ok(match result {
        MessageDialogResult::Yes => "save",
        MessageDialogResult::No => "discard",
        MessageDialogResult::Custom(label) if label == "Save" => "save",
        MessageDialogResult::Custom(label) if label == "Discard" => "discard",
        _ => "cancel",
    }
    .into())
}
#[tauri::command]
pub async fn confirm_conflict(window: WebviewWindow) -> Result<bool> {
    main_only(&window)?;
    #[cfg(feature = "desktop-e2e")]
    if let Some(choice) = crate::testing::take_choice(&window)? {
        return Ok(choice == "ok" || choice == "save");
    }
    tauri::async_runtime::spawn_blocking(move || {
        window
            .dialog()
            .message("This file changed outside Brickpress. Save your composition to another file?")
            .parent(&window)
            .title("File changed — Brickpress")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Save As".into(),
                "Cancel".into(),
            ))
            .blocking_show()
    })
    .await
    .map_err(|e| BridgeError::new("dialog", e.to_string()))
}
#[tauri::command]
pub async fn set_document_title(window: WebviewWindow, name: String, dirty: bool) -> Result<()> {
    main_only(&window)?;
    let name = name
        .chars()
        .filter(|c| !c.is_control())
        .take(200)
        .collect::<String>();
    window
        .set_title(&format!(
            "{}{} — Brickpress",
            if dirty { "● " } else { "" },
            name
        ))
        .map_err(|e| BridgeError::new("window", e.to_string()))
}
