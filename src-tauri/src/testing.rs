// Compiled only into explicitly enabled debug test builds. Never ship this API.
use crate::{
    commands::{lock, main_only},
    files::{BridgeError, FileStore, Result},
};
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{ipc::Response, Manager, State, WebviewWindow};

#[derive(Default)]
pub struct TestDialogs {
    files: VecDeque<Option<PathBuf>>,
    choices: VecDeque<String>,
}
pub fn take_file(window: &WebviewWindow) -> Result<Option<Option<PathBuf>>> {
    Ok(window
        .state::<Mutex<TestDialogs>>()
        .lock()
        .map_err(|_| BridgeError::new("test", "Test dialog lock"))?
        .files
        .pop_front())
}
pub fn take_choice(window: &WebviewWindow) -> Result<Option<String>> {
    Ok(window
        .state::<Mutex<TestDialogs>>()
        .lock()
        .map_err(|_| BridgeError::new("test", "Test dialog lock"))?
        .choices
        .pop_front())
}
fn fixture_path(store: &FileStore, name: &str) -> Result<PathBuf> {
    if name.is_empty()
        || name.contains(['/', '\\', ':'])
        || name == "."
        || name == ".."
        || name.chars().any(char::is_control)
    {
        return Err(BridgeError::new(
            "test",
            "Fixture must be a plain basename.",
        ));
    }
    let directory = store.root.join("test-files");
    fs::create_dir_all(&directory)?;
    Ok(directory.join(name))
}
#[tauri::command]
pub fn test_reset(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    dialogs: State<'_, Mutex<TestDialogs>>,
) -> Result<()> {
    main_only(&window)?;
    let mut store = lock(&store)?;
    store.reset_handles();
    for key in ["recovery", "presets", "preferences", "trace-metadata"] {
        store.write_state(key, None)?;
    }
    store.prune_assets(None)?;
    *dialogs
        .lock()
        .map_err(|_| BridgeError::new("test", "Test dialog lock"))? = TestDialogs::default();
    Ok(())
}
#[tauri::command]
pub fn test_fixture(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    name: String,
    data: String,
) -> Result<String> {
    main_only(&window)?;
    let path = fixture_path(&*lock(&store)?, &name)?;
    fs::write(&path, data)?;
    Ok(path.to_string_lossy().into_owned())
}
#[tauri::command]
pub fn test_queue_file(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    dialogs: State<'_, Mutex<TestDialogs>>,
    path: Option<String>,
) -> Result<()> {
    main_only(&window)?;
    let path = path
        .map(|path| {
            let store = lock(&store)?;
            let path = PathBuf::from(path);
            let name = path
                .file_name()
                .and_then(|v| v.to_str())
                .ok_or_else(|| BridgeError::new("test", "Invalid path"))?;
            let expected = fixture_path(&store, name)?;
            if path != expected || path.parent() != Some(Path::new(&store.root.join("test-files")))
            {
                return Err(BridgeError::new(
                    "test",
                    "Picker fixture is outside the isolated test directory.",
                ));
            }
            Ok(path)
        })
        .transpose()?;
    dialogs
        .lock()
        .map_err(|_| BridgeError::new("test", "Test dialog lock"))?
        .files
        .push_back(path);
    Ok(())
}
#[tauri::command]
pub fn test_queue_choice(
    window: WebviewWindow,
    dialogs: State<'_, Mutex<TestDialogs>>,
    choice: String,
) -> Result<()> {
    main_only(&window)?;
    if !["save", "discard", "cancel", "ok", "no"].contains(&choice.as_str()) {
        return Err(BridgeError::new("test", "Invalid test choice."));
    }
    dialogs
        .lock()
        .map_err(|_| BridgeError::new("test", "Test dialog lock"))?
        .choices
        .push_back(choice);
    Ok(())
}
#[tauri::command]
pub fn test_read_file(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    name: String,
) -> Result<String> {
    main_only(&window)?;
    Ok(fs::read_to_string(fixture_path(&*lock(&store)?, &name)?)?)
}
#[tauri::command]
pub fn test_read_bytes(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    name: String,
) -> Result<Response> {
    main_only(&window)?;
    Ok(Response::new(fs::read(fixture_path(
        &*lock(&store)?,
        &name,
    )?)?))
}
#[tauri::command]
pub fn test_corrupt_file(
    window: WebviewWindow,
    store: State<'_, Mutex<FileStore>>,
    name: String,
    data: String,
) -> Result<()> {
    main_only(&window)?;
    fs::write(fixture_path(&*lock(&store)?, &name)?, data)?;
    Ok(())
}

#[tauri::command]
pub fn test_request_close(window: WebviewWindow) -> Result<()> {
    main_only(&window)?;
    window
        .close()
        .map_err(|e| BridgeError::new("window", e.to_string()))
}

#[tauri::command]
pub fn test_request_quit(window: WebviewWindow) -> Result<()> {
    main_only(&window)?;
    window.app_handle().exit(0);
    Ok(())
}

#[tauri::command]
pub fn test_pending_dialogs(
    window: WebviewWindow,
    dialogs: State<'_, Mutex<TestDialogs>>,
) -> Result<(usize, usize)> {
    main_only(&window)?;
    let dialogs = dialogs
        .lock()
        .map_err(|_| BridgeError::new("test", "Test dialog lock"))?;
    Ok((dialogs.files.len(), dialogs.choices.len()))
}
