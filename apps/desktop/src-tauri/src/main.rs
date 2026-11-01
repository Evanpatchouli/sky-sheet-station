#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod score_io;

use score_io::{LoadedScore, Snapshot};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::ShellExt;

async fn load_bytes(
    app: &tauri::AppHandle,
    bytes: Vec<u8>,
    source: String,
) -> Result<LoadedScore, String> {
    // A snapshot ensures the CLI validates exactly the bytes the frontend receives,
    // even if the selected file is edited while the CLI is running.
    let snapshot = Snapshot::new(&bytes)?;
    let output = app
        .shell()
        .sidecar("skysheet")
        .map_err(|e| format!("SkySheet CLI 不可用，请重新构建 sidecar：{e}"))?
        .args(score_io::validation_args(&snapshot.path))
        .output()
        .await
        .map_err(|e| format!("无法启动 SkySheet CLI：{e}"))?;
    score_io::check_validation(output.status.code(), &output.stdout, &output.stderr)?;
    score_io::decode_validated(&bytes, source)
}

#[tauri::command]
async fn open_score(app: tauri::AppHandle) -> Result<Option<LoadedScore>, String> {
    let dialog_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("打开 SkySheet 乐谱")
            .add_filter("SkySheet Score", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|e| e.to_string())?;
    let source = path.to_string_lossy().into_owned();
    let bytes = tauri::async_runtime::spawn_blocking(move || score_io::read_file(&path))
        .await
        .map_err(|e| e.to_string())??;
    load_bytes(&app, bytes, source).await.map(Some)
}

#[tauri::command]
async fn load_example(app: tauri::AppHandle) -> Result<LoadedScore, String> {
    load_bytes(
        &app,
        include_bytes!("../../../../docs/examples/sky-15-example.json").to_vec(),
        "内置示例 · docs/examples/sky-15-example.json".into(),
    )
    .await
}

#[tauri::command]
async fn select_audio(app: tauri::AppHandle) -> Result<Option<String>, String> {
    // Selection only. No convert command exists in CLI Alpha.
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("选择音频（转换尚未接通）")
            .add_filter("音频", &["mp3", "wav", "flac"])
            .blocking_pick_file()
            .map(|p| p.to_string())
    })
    .await
    .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_score,
            load_example,
            select_audio
        ])
        .run(tauri::generate_context!())
        .expect("cannot start SkySheet desktop");
}
