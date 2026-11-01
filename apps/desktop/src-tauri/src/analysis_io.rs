//! CLI transport and display projection only; musical analysis stays in Rust CLI.
use serde::Serialize;
use serde_json::Value;
use std::{ffi::OsString, path::Path};

#[derive(Debug, Serialize)]
pub struct LoadedAnalysis {
    pub source: String,
    pub report: Value,
}
pub fn analysis_args(path: &Path) -> Vec<OsString> {
    vec![
        "analyze".into(),
        "--json".into(),
        "--".into(),
        path.as_os_str().into(),
    ]
}
// JSON sent over WebView IPC must not round original u64 tick values. This is
// a display projection, not a second music model or alteration of CLI JSON.
fn exact_display_integers(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for v in values {
                exact_display_integers(v);
            }
        }
        Value::Object(values) => {
            for v in values.values_mut() {
                exact_display_integers(v);
            }
        }
        Value::Number(n) if n.as_u64().is_some_and(|v| v > 9_007_199_254_740_991) => {
            *value = Value::String(n.to_string())
        }
        _ => {}
    }
}
pub fn check_analysis(
    code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
    source: String,
) -> Result<LoadedAnalysis, crate::score_io::ImportError> {
    let value: Value = serde_json::from_slice(stdout).map_err(|e| {
        format!(
            "分析 CLI 返回无效 JSON：{e}；{}",
            String::from_utf8_lossy(stderr)
        )
    })?;
    if value["output_version"] != 1
        || value["command"] != "analyze"
        || code.map(i64::from) != value["exit_code"].as_i64()
    {
        return Err("分析 CLI 版本、命令或退出码不匹配，请重新构建 sidecar。"
            .to_string()
            .into());
    }
    if code != Some(0) || value["ok"] != true || !value["error"].is_null() {
        let report = value["error"]["report"].clone();
        return Err(crate::score_io::ImportError {
            message: format!(
                "MIDI 分析失败：{}",
                value["error"]["message"]
                    .as_str()
                    .unwrap_or("请检查 MIDI 文件及诊断")
            ),
            report: (!report.is_null()).then_some(report),
        });
    }
    let mut report = value["result"]["report"].clone();
    if report["report_version"] != 2
        || !report["source_document_id"].is_string()
        || !report["structure_analysis"]["tracks"].is_array()
        || !report["track_ranking"].is_array()
        || !report["tempo_analysis"]["effective_map"].is_array()
        || !report["diagnostics"].is_array()
    {
        return Err("不支持的音乐分析报告，请重新构建 sidecar。"
            .to_string()
            .into());
    }
    exact_display_integers(&mut report);
    Ok(LoadedAnalysis { source, report })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score_io::Snapshot;
    #[test]
    fn packaged_cli_analyzes_real_midi_without_producing_score() {
        let bytes = include_bytes!("../../../../audios/ode-to-joy.mid");
        let snapshot = Snapshot::midi(bytes, std::ffi::OsStr::new("欢乐颂 & source.mid")).unwrap();
        let cli = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/skysheet-x86_64-pc-windows-msvc.exe");
        let result = std::process::Command::new(cli)
            .args(analysis_args(&snapshot.path))
            .output()
            .unwrap();
        let loaded = check_analysis(
            result.status.code(),
            &result.stdout,
            &result.stderr,
            "original source".into(),
        )
        .unwrap();
        assert_eq!(loaded.report["note_statistics"]["total_notes"], 52);
        assert_eq!(
            loaded.report["structure_analysis"]["tracks"][2]["note_count"],
            22
        );
        assert_eq!(loaded.report["track_ranking"][0]["track"], 1);
        assert_eq!(std::fs::read(&snapshot.path).unwrap(), bytes);
        assert!(!snapshot.converted_path().exists());
        let path = snapshot.path.clone();
        drop(snapshot);
        assert!(!path.exists());
        assert!(check_analysis(Some(1), &result.stdout, b"", String::new()).is_err());
        assert!(check_analysis(None, b"{}", b"", String::new()).is_err());
        let mut stale: Value = serde_json::from_slice(&result.stdout).unwrap();
        stale["result"]["report"]["report_version"] = 1.into();
        assert!(
            check_analysis(
                Some(0),
                &serde_json::to_vec(&stale).unwrap(),
                b"",
                String::new()
            )
            .is_err()
        );
    }
    #[test]
    fn failed_analysis_and_exact_tick_projection() {
        let bytes=serde_json::to_vec(&serde_json::json!({"output_version":1,"command":"analyze","exit_code":1,"ok":false,"error":{"message":"missing note off","report":{"diagnostics":[{"code":"missing_note_off"}]}},"result":null})).unwrap();
        let error = check_analysis(Some(1), &bytes, b"", String::new()).unwrap_err();
        assert!(error.message.contains("missing note off"));
        assert!(error.report.is_some());
        let mut value = serde_json::json!({"start_tick":u64::MAX,"notes":[{"end_tick":9007199254740992_u64}],"ppq":480,"score":0.8});
        exact_display_integers(&mut value);
        assert_eq!(value["start_tick"], "18446744073709551615");
        assert_eq!(value["notes"][0]["end_tick"], "9007199254740992");
        assert_eq!(value["ppq"], 480);
        assert_eq!(value["score"], 0.8);
    }
}
