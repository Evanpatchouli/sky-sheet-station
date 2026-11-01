#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analysis_io;
mod arrangement_io;
mod score_io;

use score_io::{LoadedScore, Snapshot};
use std::sync::Mutex;
use tauri::Manager;
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
async fn open_midi(app: tauri::AppHandle) -> Result<Option<LoadedScore>, score_io::ImportError> {
    let picker = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_title("打开 MIDI")
            .add_filter("Standard MIDI File", &["mid", "midi"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|e| e.to_string())?;
    let source = path.to_string_lossy().into_owned();
    let snapshot = tauri::async_runtime::spawn_blocking(move || {
        let bytes = score_io::read_file(&path)?;
        score_io::Snapshot::midi(&bytes, path.file_name().ok_or("MIDI 文件名无效")?)
    })
    .await
    .map_err(|e| e.to_string())??;
    let output_path = snapshot.converted_path();
    let output = app
        .shell()
        .sidecar("skysheet")
        .map_err(|e| e.to_string())?
        .args(score_io::import_args(&snapshot.path, &output_path))
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let report = score_io::check_import(
        output.status.code(),
        &output.stdout,
        &output.stderr,
        &output_path,
    )?;
    let bytes = score_io::read_file(&output_path)?;
    let mut loaded = load_bytes(&app, bytes, source).await?;
    loaded.import_report = Some(report);
    Ok(Some(loaded))
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

#[tauri::command]
async fn analyze_midi(
    app: tauri::AppHandle,
) -> Result<Option<analysis_io::LoadedAnalysis>, score_io::ImportError> {
    let picker = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_title("分析 MIDI")
            .add_filter("Standard MIDI File", &["mid", "midi"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|e| e.to_string())?;
    let source = path.to_string_lossy().into_owned();
    let snapshot = tauri::async_runtime::spawn_blocking(move || {
        let bytes = score_io::read_file(&path)?;
        Snapshot::midi(&bytes, path.file_name().ok_or("MIDI 文件名无效")?)
    })
    .await
    .map_err(|e| e.to_string())??;
    let output = app
        .shell()
        .sidecar("skysheet")
        .map_err(|e| e.to_string())?
        .args(analysis_io::analysis_args(&snapshot.path))
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let loaded =
        analysis_io::check_analysis(output.status.code(), &output.stdout, &output.stderr, source)?;
    let document_id = loaded.report["source_document_id"]
        .as_str()
        .ok_or_else(|| "分析缺少源标识".to_string())?
        .to_string();
    *app.state::<Mutex<Option<arrangement_io::ArrangementSource>>>()
        .lock()
        .map_err(|e| e.to_string())? = Some(arrangement_io::ArrangementSource {
        snapshot,
        document_id,
    });
    Ok(Some(loaded))
}

#[tauri::command]
async fn arrange_midi(
    app: tauri::AppHandle,
    source_document_id: String,
    transpose: Option<i16>,
) -> Result<Option<arrangement_io::LoadedArrangement>, score_io::ImportError> {
    if transpose.is_some_and(|v| !(-127..=127).contains(&v)) {
        return Err("移调必须是 -127..127 的整数".to_string().into());
    }
    // Copy the analyzed snapshot, not a potentially changed original file.
    // Release the mutex before any await or picker interaction.
    let snapshot = {
        let state = app.state::<Mutex<Option<arrangement_io::ArrangementSource>>>();
        let guard = state.lock().map_err(|e| e.to_string())?;
        let source = guard.as_ref().ok_or_else(|| "请先分析 MIDI".to_string())?;
        if source.document_id != source_document_id {
            return Err("分析来源已变化，请重新查看分析结果".to_string().into());
        }
        Snapshot::midi(
            &score_io::read_file(&source.snapshot.path)?,
            source
                .snapshot
                .path
                .file_name()
                .ok_or_else(|| "无效来源".to_string())?,
        )?
    };
    let picker = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_title("保存 Sky 编曲（不会覆盖已有文件）")
            .set_file_name("sky-arrangement.json")
            .add_filter("SkySheet Score", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let output_path = selected.into_path().map_err(|e| e.to_string())?;
    let output = app
        .shell()
        .sidecar("skysheet")
        .map_err(|e| e.to_string())?
        .args(arrangement_io::arrangement_args(
            &snapshot.path,
            &output_path,
            transpose,
        ))
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let report = arrangement_io::check_arrangement(
        output.status.code(),
        &output.stdout,
        &output.stderr,
        &output_path,
        &source_document_id,
    )?;
    let loaded = load_bytes(
        &app,
        score_io::read_file(&output_path)?,
        output_path.to_string_lossy().into_owned(),
    )
    .await?;
    Ok(Some(arrangement_io::LoadedArrangement { loaded, report }))
}

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(None::<arrangement_io::ArrangementSource>))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_score,
            open_midi,
            analyze_midi,
            arrange_midi,
            load_example,
            select_audio
        ])
        .run(tauri::generate_context!())
        .expect("cannot start SkySheet desktop");
}
