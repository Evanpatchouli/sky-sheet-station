//! Audio transport only. Decoding, model inference and MIDI writing live in CLI.
use crate::{
    analysis_io, arrangement_io,
    score_io::{self, Snapshot},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    ffi::OsString,
    io::{Read, Write},
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tauri::{Manager, ipc::Channel};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::{
    ShellExt,
    process::{CommandChild, CommandEvent},
};

struct ActiveJob {
    id: String,
    cancelled: bool,
    child: Option<CommandChild>,
}
#[derive(Default)]
pub struct Jobs(Mutex<Option<ActiveJob>>);
pub struct PendingAudio {
    snapshot: Snapshot,
    report: Vec<u8>,
    document_id: String,
}
#[derive(Deserialize)]
pub struct Options {
    onset_threshold: f64,
    frame_threshold: f64,
    minimum_note_ms: f64,
}
#[derive(Serialize)]
pub struct LoadedAudio {
    token: String,
    source: String,
    report: Value,
    analysis: analysis_io::LoadedAnalysis,
}

struct JobGuard {
    app: tauri::AppHandle,
    id: String,
}
impl Drop for JobGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.app.state::<Jobs>().0.lock()
            && state.as_ref().is_some_and(|job| job.id == self.id)
        {
            // kill_on_drop protects app/window shutdown and unexpected errors.
            state.take();
        }
    }
}
fn active(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<Jobs>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    if guard
        .as_ref()
        .is_some_and(|job| job.id == id && !job.cancelled)
    {
        Ok(())
    } else {
        Err("音频转录已取消".into())
    }
}

#[tauri::command]
pub async fn cancel_audio(app: tauri::AppHandle, request_id: String) -> Result<(), String> {
    let child = {
        let state = app.state::<Jobs>();
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        if let Some(job) = guard.as_mut().filter(|job| job.id == request_id) {
            job.cancelled = true;
            job.child.take()
        } else {
            None
        }
    };
    if let Some(child) = child {
        tauri::async_runtime::spawn_blocking(move || child.kill())
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn args(input: &Path, output: &Path, options: &Options) -> Vec<OsString> {
    vec![
        "transcribe".into(),
        "--json".into(),
        "--progress".into(),
        "-o".into(),
        output.into(),
        "--onset-threshold".into(),
        options.onset_threshold.to_string().into(),
        "--frame-threshold".into(),
        options.frame_threshold.to_string().into(),
        "--minimum-note-ms".into(),
        options.minimum_note_ms.to_string().into(),
        "--".into(),
        input.into(),
    ]
}

async fn run(
    app: &tauri::AppHandle,
    id: &str,
    args: Vec<OsString>,
    channel: &Channel<Value>,
) -> Result<(Option<i32>, Vec<u8>), String> {
    active(app, id)?;
    let (mut events, child) = app
        .shell()
        .sidecar("skysheet")
        .map_err(|e| e.to_string())?
        .args(args)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| e.to_string())?;
    {
        let state = app.state::<Jobs>();
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        let job = guard
            .as_mut()
            .filter(|job| job.id == id && !job.cancelled)
            .ok_or("音频转录已取消")?;
        job.child = Some(child);
    }
    let mut stdout = Vec::new();
    let mut failure = None;
    let mut code = None;
    while let Some(event) = events.recv().await {
        match event {
            CommandEvent::Stdout(bytes) => {
                if stdout.len() + bytes.len() > 64 * 1024 * 1024 {
                    failure = Some("转录响应超过 64 MiB".to_string());
                    cancel_audio(app.clone(), id.into()).await?;
                } else {
                    stdout.extend(bytes);
                    stdout.push(b'\n');
                }
            }
            CommandEvent::Stderr(bytes) => {
                if let Ok(progress) = serde_json::from_slice::<Value>(&bytes)
                    && progress["stage"].is_string()
                    && progress["completed"].is_u64()
                {
                    let _ = channel.send(progress);
                }
            }
            CommandEvent::Error(e) => {
                failure = Some(e);
                cancel_audio(app.clone(), id.into()).await?;
            }
            CommandEvent::Terminated(status) => {
                code = status.code;
                break;
            }
            _ => {}
        }
    }
    // All writes are finished before the snapshot can be dropped.
    if let Ok(mut guard) = app.state::<Jobs>().0.lock()
        && let Some(job) = guard.as_mut().filter(|job| job.id == id)
    {
        job.child.take();
    }
    if let Some(error) = failure {
        return Err(error);
    }
    active(app, id)?;
    Ok((code, stdout))
}

fn check_transcription(code: Option<i32>, bytes: &[u8], output: &Path) -> Result<Value, String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("转录 CLI 返回无效 JSON：{e}"))?;
    if value["output_version"] != 1
        || value["command"] != "transcribe"
        || code.is_none()
        || code.map(i64::from) != value["exit_code"].as_i64()
    {
        return Err("转录 CLI 版本、命令或退出码不匹配".into());
    }
    if code != Some(0) || value["ok"] != true || !value["error"].is_null() {
        return Err(format!(
            "转录失败：{}",
            value["error"]["message"]
                .as_str()
                .unwrap_or("请检查音频文件")
        ));
    }
    if value["result"]["output_path"].as_str() != Some(output.to_string_lossy().as_ref())
        || value["result"]["report"]["report_version"] != 1
        || !value["result"]["report"]["notes"].is_array()
    {
        return Err("转录结果与输出文件不一致".into());
    }
    Ok(value["result"].clone())
}

#[tauri::command]
pub async fn transcribe_audio(
    app: tauri::AppHandle,
    request_id: String,
    options: Options,
    on_progress: Channel<Value>,
) -> Result<Option<LoadedAudio>, String> {
    {
        let state = app.state::<Jobs>();
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("已有音频任务正在结束，请稍后重试".into());
        }
        *guard = Some(ActiveJob {
            id: request_id.clone(),
            cancelled: false,
            child: None,
        });
    }
    let _guard = JobGuard {
        app: app.clone(),
        id: request_id.clone(),
    };
    let generation = app.state::<AtomicU64>().fetch_add(1, Ordering::SeqCst) + 1;
    let picker = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_title("选择要转录的音频 · 最长 10 分钟 / 128 MiB")
            .add_filter("音频", &["mp3", "wav", "flac"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    active(&app, &request_id)?;
    let path = selected.into_path().map_err(|e| e.to_string())?;
    let source = path.to_string_lossy().into_owned();
    let _ = on_progress.send(serde_json::json!({"stage":"import","completed":0,"total":null}));
    let snapshot = tauri::async_runtime::spawn_blocking(move || {
        let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("请选择音频文件".into());
        }
        let mut bytes = Vec::new();
        file.take(128 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.is_empty() || bytes.len() > 128 * 1024 * 1024 {
            return Err("音频文件须为 1 字节至 128 MiB".into());
        }
        Snapshot::named(&bytes, path.file_name().ok_or("文件名无效")?)
    })
    .await
    .map_err(|e| e.to_string())??;
    let output = snapshot.path.parent().unwrap().join("transcription.mid");
    let (code, stdout) = run(
        &app,
        &request_id,
        args(&snapshot.path, &output, &options),
        &on_progress,
    )
    .await?;
    let result = check_transcription(code, &stdout, &output)?;
    let _ = on_progress.send(serde_json::json!({"stage":"analyze","completed":0,"total":null}));
    let midi = score_io::read_file(&output)?;
    let midi_snapshot = Snapshot::midi(&midi, std::ffi::OsStr::new("transcribed.mid"))?;
    let (code, stdout) = run(
        &app,
        &request_id,
        analysis_io::analysis_args(&midi_snapshot.path),
        &on_progress,
    )
    .await?;
    let analysis =
        analysis_io::check_analysis(code, &stdout, &[], source.clone()).map_err(|e| e.message)?;
    active(&app, &request_id)?;
    if app.state::<AtomicU64>().load(Ordering::SeqCst) != generation {
        return Err("音频结果已过期".into());
    }
    let document_id = analysis.report["source_document_id"]
        .as_str()
        .ok_or("分析缺少来源标识")?
        .to_string();
    let pending = Snapshot::midi(&midi, std::ffi::OsStr::new("transcribed.mid"))?;
    let token = pending.path.to_string_lossy().into_owned();
    let report_bytes = std::fs::read(format!("{}.transcription.json", output.display()))
        .map_err(|e| e.to_string())?;
    *app.state::<Mutex<Option<arrangement_io::ArrangementSource>>>()
        .lock()
        .map_err(|e| e.to_string())? = Some(arrangement_io::ArrangementSource {
        snapshot: midi_snapshot,
        document_id: document_id.clone(),
    });
    *app.state::<Mutex<Option<PendingAudio>>>()
        .lock()
        .map_err(|e| e.to_string())? = Some(PendingAudio {
        snapshot: pending,
        report: report_bytes,
        document_id,
    });
    Ok(Some(LoadedAudio {
        token,
        source,
        report: result["report"].clone(),
        analysis,
    }))
}

#[tauri::command]
pub fn activate_transcription(app: tauri::AppHandle, token: String) -> Result<(), String> {
    let source = {
        let state = app.state::<Mutex<Option<PendingAudio>>>();
        let guard = state.lock().map_err(|e| e.to_string())?;
        let pending = guard
            .as_ref()
            .filter(|p| p.snapshot.path.to_string_lossy() == token)
            .ok_or("转录结果已过期，请重新转录")?;
        arrangement_io::ArrangementSource {
            snapshot: Snapshot::midi(
                &score_io::read_file(&pending.snapshot.path)?,
                std::ffi::OsStr::new("transcribed.mid"),
            )?,
            document_id: pending.document_id.clone(),
        }
    };
    app.state::<AtomicU64>().fetch_add(1, Ordering::SeqCst);
    *app.state::<Mutex<Option<arrangement_io::ArrangementSource>>>()
        .lock()
        .map_err(|e| e.to_string())? = Some(source);
    Ok(())
}

fn save_pair(path: &Path, midi: &[u8], report: &[u8]) -> Result<(), String> {
    let report_path = format!("{}.transcription.json", path.display());
    let mut a = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("不覆盖已有文件：{e}"))?;
    let mut b = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&report_path)
    {
        Ok(b) => b,
        Err(e) => {
            drop(a);
            let _ = std::fs::remove_file(path);
            return Err(e.to_string());
        }
    };
    let written = a
        .write_all(midi)
        .and_then(|_| a.sync_all())
        .and_then(|_| b.write_all(report))
        .and_then(|_| b.sync_all());
    drop(a);
    drop(b);
    if let Err(e) = written {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(report_path);
        return Err(e.to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn save_transcription(
    app: tauri::AppHandle,
    token: String,
) -> Result<Option<String>, String> {
    let (midi, report) = {
        let state = app.state::<Mutex<Option<PendingAudio>>>();
        let guard = state.lock().map_err(|e| e.to_string())?;
        let pending = guard
            .as_ref()
            .filter(|p| p.snapshot.path.to_string_lossy() == token)
            .ok_or("转录结果已过期，请重新转录")?;
        (
            score_io::read_file(&pending.snapshot.path)?,
            pending.report.clone(),
        )
    };
    let picker = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_title("保存转录 MIDI 和报告（不覆盖）")
            .set_file_name("transcribed.mid")
            .add_filter("MIDI", &["mid"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        save_pair(&path, &midi, &report)?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn response_identity_exit_status_and_file_are_checked() {
        let output = Path::new("result.mid");
        let mut value = serde_json::json!({"output_version":1,"command":"transcribe","exit_code":0,"ok":true,"error":null,
            "result":{"output_path":"result.mid","report":{"report_version":1,"notes":[]}}});
        assert!(check_transcription(Some(0), &serde_json::to_vec(&value).unwrap(), output).is_ok());
        assert!(check_transcription(None, &serde_json::to_vec(&value).unwrap(), output).is_err());
        value["command"] = "arrange".into();
        assert!(
            check_transcription(Some(0), &serde_json::to_vec(&value).unwrap(), output).is_err()
        );
        value["command"] = "transcribe".into();
        value["result"]["output_path"] = "other.mid".into();
        assert!(
            check_transcription(Some(0), &serde_json::to_vec(&value).unwrap(), output).is_err()
        );
    }
    #[test]
    fn saves_exact_midi_and_report_without_overwrite_or_partial_pair() {
        let snapshot = Snapshot::new(b"source").unwrap();
        let output = snapshot.path.parent().unwrap().join("transcription.mid");
        let report = output.with_file_name("transcription.mid.transcription.json");
        std::fs::write(&report, b"owned report").unwrap();
        assert!(save_pair(&output, b"midi", b"report").is_err());
        assert!(!output.exists());
        assert_eq!(std::fs::read(&report).unwrap(), b"owned report");
        std::fs::remove_file(&report).unwrap();
        save_pair(&output, b"midi", b"report").unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), b"midi");
        assert!(save_pair(&output, b"replace", b"replace").is_err());
        assert_eq!(std::fs::read(&output).unwrap(), b"midi");
    }
}
