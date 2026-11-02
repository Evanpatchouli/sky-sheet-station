//! Native selection authority for the one-dialog generation workflow.
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
pub struct Selection(Mutex<Option<PathBuf>>);

#[derive(Serialize)]
pub struct SelectedSource {
    token: String,
    name: String,
    kind: &'static str,
}

fn kind(path: &std::path::Path) -> Result<&'static str, String> {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "mid" | "midi" => Ok("midi"),
        "mp3" | "wav" | "flac" => Ok("audio"),
        _ => Err("请选择 MIDI、MP3、WAV 或 FLAC 文件。".into()),
    }
}

pub fn selected_path(
    app: &tauri::AppHandle,
    token: &str,
    expected: &str,
) -> Result<PathBuf, String> {
    let state = app.state::<Selection>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let path = guard
        .as_ref()
        .filter(|path| path.to_string_lossy() == token)
        .ok_or("选择的文件已变化，请重新选择。")?;
    if kind(path)? != expected {
        return Err("选择的文件类型不匹配。".into());
    }
    Ok(path.clone())
}

#[tauri::command]
pub async fn select_source(app: tauri::AppHandle) -> Result<Option<SelectedSource>, String> {
    let picker = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_title("导入音频或 MIDI")
            .add_filter("音乐文件", &["mid", "midi", "mp3", "wav", "flac"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(file) = selected else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|e| e.to_string())?;
    let result = SelectedSource {
        token: path.to_string_lossy().into_owned(),
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        kind: kind(&path)?,
    };
    *app.state::<Selection>()
        .0
        .lock()
        .map_err(|e| e.to_string())? = Some(path);
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_only_supported_native_selections() {
        for (name, expected) in [
            ("音频.WAV", "audio"),
            ("a.FLAC", "audio"),
            ("a.MP3", "audio"),
            ("a.MIDI", "midi"),
            ("a.mid", "midi"),
        ] {
            assert_eq!(kind(std::path::Path::new(name)).unwrap(), expected);
        }
        assert!(kind(std::path::Path::new("a.json")).is_err());
    }
}
