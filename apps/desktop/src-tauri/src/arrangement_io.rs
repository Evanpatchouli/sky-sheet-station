//! Transport only: the packaged CLI owns arrangement and file output.
use crate::score_io::{ImportError, LoadedScore, Snapshot};
use serde::Serialize;
use serde_json::Value;
use std::{ffi::OsString, path::Path};

pub struct ArrangementSource {
    pub snapshot: Snapshot,
    pub document_id: String,
}
#[derive(Serialize)]
pub struct LoadedArrangement {
    pub loaded: LoadedScore,
    pub report: Value,
}

pub fn arrangement_args(input: &Path, output: &Path, transpose: Option<i16>) -> Vec<OsString> {
    let mut args = vec![
        "arrange".into(),
        "--json".into(),
        "-o".into(),
        output.as_os_str().into(),
    ];
    if let Some(shift) = transpose {
        args.extend(["--transpose".into(), shift.to_string().into()]);
    } else {
        args.push("--auto".into());
    }
    args.extend(["--".into(), input.as_os_str().into()]);
    args
}
pub fn check_arrangement(
    code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
    expected: &Path,
    document_id: &str,
) -> Result<Value, ImportError> {
    let value: Value = serde_json::from_slice(stdout)
        .map_err(|e| format!("编曲 CLI 响应无效：{e} {}", String::from_utf8_lossy(stderr)))?;
    if value["output_version"] != 1
        || value["command"] != "arrange"
        || code.is_none()
        || code.map(i64::from) != value["exit_code"].as_i64()
    {
        return Err("编曲 CLI 响应版本、命令或退出码不匹配".to_string().into());
    }
    if code != Some(0) || value["ok"] != true || !value["error"].is_null() {
        let mut report = value["error"]["arrangement_report"].clone();
        crate::analysis_io::exact_display_integers(&mut report);
        return Err(ImportError {
            message: format!(
                "编曲失败：{}。此前乐谱已保留。",
                value["error"]["message"]
                    .as_str()
                    .unwrap_or("请检查输入与诊断")
            ),
            report: (report.is_object() && report["stage"] != "preparation_failed")
                .then_some(report),
        });
    }
    let mut report = value["result"]["report"].clone();
    if value["result"]["output_path"].as_str() != Some(expected.to_string_lossy().as_ref())
        || report["arrangement_report_version"] != 1
        || report["source_document_id"] != document_id
        || report["score_generated"] != true
        || report["score_validation"] != true
        || report["time_preserved"] != true
        || report["pitch_mapping_valid"] != true
        || !report["note_mappings"].is_array()
        || !report["candidates"].is_array()
    {
        return Err("编曲报告与输入、输出或验证结果不一致；未切换播放器。"
            .to_string()
            .into());
    }
    crate::analysis_io::exact_display_integers(&mut report);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_sidecar_arranges_ode_and_loads_through_existing_score_reader() {
        let raw = include_bytes!("../../../../audios/ode-to-joy.mid");
        let snapshot = Snapshot::midi(raw, std::ffi::OsStr::new("欢乐颂 & source.mid")).unwrap();
        let output = snapshot.converted_path();
        let cli = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/skysheet-x86_64-pc-windows-msvc.exe");
        let analysis = std::process::Command::new(&cli)
            .args(crate::analysis_io::analysis_args(&snapshot.path))
            .output()
            .unwrap();
        let source = crate::analysis_io::check_analysis(
            analysis.status.code(),
            &analysis.stdout,
            &analysis.stderr,
            "original".into(),
        )
        .unwrap();
        let id = source.report["source_document_id"].as_str().unwrap();
        let result = std::process::Command::new(&cli)
            .args(arrangement_args(&snapshot.path, &output, None))
            .output()
            .unwrap();
        let report = check_arrangement(
            result.status.code(),
            &result.stdout,
            &result.stderr,
            &output,
            id,
        )
        .unwrap();
        assert_eq!(report["kept"], 30);
        assert_eq!(report["dropped"], 22);
        let validation = std::process::Command::new(&cli)
            .args(crate::score_io::validation_args(&output))
            .output()
            .unwrap();
        crate::score_io::check_validation(
            validation.status.code(),
            &validation.stdout,
            &validation.stderr,
        )
        .unwrap();
        let loaded =
            crate::score_io::decode_validated(&std::fs::read(&output).unwrap(), "arranged".into())
                .unwrap();
        assert_eq!(loaded.score.tracks[1].notes.len(), 30);
        assert_eq!(loaded.midi_keys.len(), 15);
        assert_eq!(std::fs::read(&snapshot.path).unwrap(), raw);
        assert!(
            check_arrangement(Some(0), &result.stdout, b"", &output, "different source").is_err()
        );
        assert!(
            check_arrangement(Some(0), &result.stdout, b"", Path::new("wrong.json"), id).is_err()
        );
        let dir = snapshot.path.parent().unwrap().to_path_buf();
        drop(snapshot);
        assert!(!dir.exists());
    }

    #[test]
    fn failed_fixed_arrangement_retains_report_and_does_not_produce_score() {
        let snapshot = Snapshot::midi(
            include_bytes!("../../../../audios/ode-to-joy.mid"),
            std::ffi::OsStr::new("source.mid"),
        )
        .unwrap();
        let output = snapshot.converted_path();
        let cli = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/skysheet-x86_64-pc-windows-msvc.exe");
        let result = std::process::Command::new(cli)
            .args(arrangement_args(&snapshot.path, &output, Some(127)))
            .output()
            .unwrap();
        let err = check_arrangement(
            result.status.code(),
            &result.stdout,
            &result.stderr,
            &output,
            "unused for failures",
        )
        .unwrap_err();
        assert_eq!(err.report.unwrap()["dropped"], 52);
        assert!(!output.exists());
        assert!(check_arrangement(None, b"{}", b"", &output, "").is_err());
        let dir = snapshot.path.parent().unwrap().to_path_buf();
        drop(snapshot);
        assert!(!dir.exists());
    }
}
