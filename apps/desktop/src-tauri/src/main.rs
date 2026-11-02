#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analysis_io;
mod arrangement_io;
mod audio_io;
mod score_io;
mod source_io;

use score_io::{LoadedScore, Snapshot};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
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
async fn save_score(app: tauri::AppHandle, json: String) -> Result<Option<String>, String> {
    if json.len() > 32 * 1024 * 1024 {
        return Err("此桌面版本最多保存 32 MiB 乐谱。".into());
    }
    // Keep the exact frontend bytes through the existing raw reader / CLI gate.
    // No serde_json::Value conversion may round tempo before validation.
    load_bytes(&app, json.as_bytes().to_vec(), "编辑乐谱".into()).await?;
    let picker = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_title("另存乐谱（请选择新文件）")
            .set_file_name("sky-score.json")
            .add_filter("SkySheet Score", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        score_io::write_new_score(&path, json.as_bytes())?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
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
async fn analyze_midi(
    app: tauri::AppHandle,
    source_token: Option<String>,
) -> Result<Option<analysis_io::LoadedAnalysis>, score_io::ImportError> {
    let generation = app.state::<AtomicU64>().fetch_add(1, Ordering::SeqCst) + 1;
    let path = if let Some(token) = source_token {
        source_io::selected_path(&app, &token, "midi")?
    } else {
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
        selected.into_path().map_err(|e| e.to_string())?
    };
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
    if app.state::<AtomicU64>().load(Ordering::SeqCst) != generation {
        return Err("分析请求已过期".to_string().into());
    }
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
    options: arrangement_io::Options,
) -> Result<Option<arrangement_io::LoadedArrangement>, score_io::ImportError> {
    let generation = app.state::<AtomicU64>().fetch_add(1, Ordering::SeqCst) + 1;
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
    let output_path = snapshot.converted_path();
    let output = app
        .shell()
        .sidecar("skysheet")
        .map_err(|e| e.to_string())?
        .args(arrangement_io::arrangement_args(
            &snapshot.path,
            &output_path,
            transpose,
            &options,
        ))
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let mut report = arrangement_io::check_arrangement(
        output.status.code(),
        &output.stdout,
        &output.stderr,
        &output_path,
        &source_document_id,
    )?;
    let loaded = load_bytes(
        &app,
        score_io::read_file(&output_path)?,
        "编曲预览（未保存）".into(),
    )
    .await?;
    arrangement_io::verify_score(&report, &loaded)?;
    if app.state::<AtomicU64>().load(Ordering::SeqCst) != generation {
        return Err("编曲请求已过期".to_string().into());
    }
    let token = output_path.to_string_lossy().into_owned();
    report["score_file"] = serde_json::Value::Null;
    report["report_file"] = serde_json::Value::Null;
    *app.state::<Mutex<Option<arrangement_io::PendingArrangement>>>()
        .lock()
        .map_err(|e| e.to_string())? = Some(arrangement_io::PendingArrangement {
        snapshot,
        token: token.clone(),
    });
    Ok(Some(arrangement_io::LoadedArrangement {
        token,
        loaded,
        report,
    }))
}

#[tauri::command]
async fn save_arrangement(
    app: tauri::AppHandle,
    token: String,
) -> Result<Option<serde_json::Value>, String> {
    // Capture the validated preview before opening the dialog; token prevents
    // a stale frontend result from saving a different arrangement.
    let (score, report) = {
        let state = app.state::<Mutex<Option<arrangement_io::PendingArrangement>>>();
        let guard = state.lock().map_err(|e| e.to_string())?;
        let pending = guard
            .as_ref()
            .filter(|p| p.token == token)
            .ok_or("编曲预览已过期，请重新生成")?;
        let output = pending.snapshot.converted_path();
        let score = score_io::read_file(&output)?;
        let raw = std::fs::read(format!("{}.arrangement.json", output.display()))
            .map_err(|e| e.to_string())?;
        (
            score,
            serde_json::from_slice(&raw).map_err(|e| e.to_string())?,
        )
    };
    let picker = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_title("保存 Score 和独立编曲报告（不覆盖）")
            .set_file_name("sky-arrangement.json")
            .add_filter("SkySheet Score", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || arrangement_io::save_pair(&path, &score, report))
        .await
        .map_err(|e| e.to_string())?
        .map(Some)
}

#[tauri::command]
fn cancel_arrangement(app: tauri::AppHandle) {
    // Invalidate completion without terminating a CLI that owns temporary files.
    app.state::<AtomicU64>().fetch_add(1, Ordering::SeqCst);
}

fn main() {
    tauri::Builder::default()
        .manage(source_io::Selection::default())
        .manage(audio_io::Jobs::default())
        .manage(Mutex::new(None::<audio_io::PendingAudio>))
        .manage(Mutex::new(None::<arrangement_io::ArrangementSource>))
        .manage(Mutex::new(None::<arrangement_io::PendingArrangement>))
        .manage(AtomicU64::new(0))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_score,
            save_score,
            source_io::select_source,
            open_midi,
            analyze_midi,
            arrange_midi,
            save_arrangement,
            cancel_arrangement,
            load_example,
            audio_io::transcribe_audio,
            audio_io::activate_transcription,
            audio_io::cancel_audio,
            audio_io::save_transcription
        ])
        .run(tauri::generate_context!())
        .expect("cannot start SkySheet desktop");
}
