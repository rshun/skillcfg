use crate::{
    bridge::SafeError,
    directory::{choose_directory, DirectoryPicker},
    DesktopState,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PickerRequest {
    initial: Option<PathBuf>,
}
struct NativePicker {
    window: WebviewWindow,
}
impl DirectoryPicker for NativePicker {
    fn pick(&self, initial: Option<&Path>) -> Result<Option<PathBuf>, SafeError> {
        let mut dialog = self
            .window
            .app_handle()
            .dialog()
            .file()
            .set_parent(&self.window)
            .set_can_create_directories(false)
            .set_title("选择 Skill Hub 目录");
        if let Some(path) = initial {
            dialog = dialog.set_directory(path);
        }
        dialog
            .blocking_pick_folder()
            .map(|p| {
                p.into_path()
                    .map_err(|_| SafeError::new("invalid_directory"))
            })
            .transpose()
    }
}
#[tauri::command]
pub async fn execute(
    window: WebviewWindow,
    state: State<'_, Arc<DesktopState>>,
    command: String,
    request: Value,
) -> Result<Value, SafeError> {
    if command == "choose_directory" {
        let args: PickerRequest =
            serde_json::from_value(request).map_err(|_| SafeError::new("invalid_request"))?;
        let picker = NativePicker { window };
        return tauri::async_runtime::spawn_blocking(move || {
            choose_directory(&picker, args.initial.as_deref()).map(|p| json!(p))
        })
        .await
        .map_err(|_| SafeError::new("picker_unavailable"))?;
    }
    let label = window.label().to_owned();
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.execute(&label, &command, request))
        .await
        .map_err(|_| SafeError::new("session_unavailable"))?
}
