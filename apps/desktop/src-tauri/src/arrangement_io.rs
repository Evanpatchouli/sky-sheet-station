//! Transport only: the packaged CLI owns arrangement and file output.
use crate::score_io::{ImportError, LoadedScore, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{ffi::OsString, path::Path};

pub struct ArrangementSource {
    pub snapshot: Snapshot,
    pub document_id: String,
}
#[derive(Serialize)]
pub struct LoadedArrangement {
    pub token: String,
    pub loaded: LoadedScore,
    pub report: Value,
}

pub struct PendingArrangement {
    pub snapshot: Snapshot,
    pub token: String,
}

#[derive(Deserialize)]
pub struct Options {
    pub mode: String,
    pub allow_octave_shifts: bool,
    pub melody_priority: u8,
    pub bass_priority: u8,
    pub accompaniment_priority: u8,
    pub maximum_polyphony: Option<usize>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            mode: "melody-first".into(),
            allow_octave_shifts: true,
            melody_priority: 3,
            bass_priority: 2,
            accompaniment_priority: 1,
            maximum_polyphony: None,
        }
    }
}

pub fn arrangement_args(
    input: &Path,
    output: &Path,
    transpose: Option<i16>,
    options: &Options,
) -> Vec<OsString> {
    let mut args = vec![
        "arrange".into(),
        "--json".into(),
        "-o".into(),
        output.as_os_str().into(),
    ];
    args.extend([
        "--mode".into(),
        options.mode.clone().into(),
        "--melody-priority".into(),
        options.melody_priority.to_string().into(),
        "--bass-priority".into(),
        options.bass_priority.to_string().into(),
        "--accompaniment-priority".into(),
        options.accompaniment_priority.to_string().into(),
    ]);
    if !options.allow_octave_shifts {
        args.push("--no-octave-shifts".into());
    }
    if let Some(max) = options.maximum_polyphony {
        args.extend(["--max-polyphony".into(), max.to_string().into()]);
    }
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
    if stdout.len() > 64 * 1024 * 1024 {
        return Err("编曲响应超过桌面 64 MiB 资源边界".to_string().into());
    }
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
        || report["arrangement_report_version"] != 2
        || report["source_document_id"] != document_id
        || report["score_generated"] != true
        || report["score_validation"] != true
        || report["time_preserved"] != true
        || report["pitch_mapping_valid"] != true
        || !report["note_mappings"].is_array()
        || !report["candidates"].is_array()
        || report["playability"]["arrangement_constraints_satisfied"] != true
    {
        return Err("编曲报告与输入、输出或验证结果不一致；未切换播放器。"
            .to_string()
            .into());
    }
    crate::analysis_io::exact_display_integers(&mut report);
    Ok(report)
}

pub fn verify_score(report: &Value, loaded: &LoadedScore) -> Result<(), String> {
    let mappings = report["note_mappings"]
        .as_array()
        .ok_or("missing mappings")?;
    let notes: std::collections::BTreeMap<_, _> = loaded
        .score
        .tracks
        .iter()
        .enumerate()
        .flat_map(|(t, track)| track.notes.iter().map(move |n| (n.id.as_str(), (t, n))))
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    let mut kept = 0;
    for m in mappings {
        let id = m["source_note_id"]
            .as_str()
            .ok_or("invalid note identity")?;
        if !seen.insert(id) {
            return Err("duplicate mapping".into());
        }
        if m["mapped"] != true {
            if notes.contains_key(id) {
                return Err("dropped note exists in Score".into());
            }
            continue;
        }
        kept += 1;
        let (t, n) = notes.get(id).ok_or("mapped note missing from Score")?;
        if m["source_track"].as_u64() != Some(*t as u64)
            || m["target_midi"].as_u64() != Some(u64::from(n.pitch.midi_number()))
            || m["projected_start_tick"].as_u64() != Some(n.start_tick)
            || m["projected_duration_ticks"].as_u64() != Some(n.duration_ticks)
            || m["key_index"].as_u64().map(|k| k + 1)
                != loaded.score.key_number_for(&n.pitch).map(|k| k as u64)
        {
            return Err(format!("Score/report mismatch for {id}"));
        }
    }
    if kept != notes.len()
        || report["kept"].as_u64() != Some(kept as u64)
        || report["original_notes"].as_u64() != Some(mappings.len() as u64)
    {
        return Err("Score/report count mismatch".into());
    }
    Ok(())
}

pub fn save_pair(output: &Path, score: &[u8], mut report: Value) -> Result<Value, String> {
    use std::io::Write;
    let report_path = std::path::PathBuf::from(format!("{}.arrangement.json", output.display()));
    report["score_file"] = output.to_string_lossy().as_ref().into();
    report["report_file"] = report_path.to_string_lossy().as_ref().into();
    let bytes = serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?;
    let mut a = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    let b = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&report_path);
    let mut b = match b {
        Ok(b) => b,
        Err(e) => {
            drop(a);
            let cleanup = std::fs::remove_file(output);
            return Err(format!("{e}; cleanup: {cleanup:?}"));
        }
    };
    let write = (|| -> std::io::Result<()> {
        a.write_all(score)?;
        b.write_all(&bytes)?;
        a.sync_all()?;
        b.sync_all()
    })();
    drop(a);
    drop(b);
    if let Err(e) = write {
        let x = std::fs::remove_file(output);
        let y = std::fs::remove_file(&report_path);
        return Err(format!("{e}; cleanup: {x:?}, {y:?}"));
    }
    crate::analysis_io::exact_display_integers(&mut report);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_preserves_bytes_and_refuses_collisions_without_partial_score() {
        let snapshot = Snapshot::new(b"fixture").unwrap();
        let output = snapshot.path.parent().unwrap().join("saved.json");
        let bytes = include_bytes!("../../../../docs/examples/sky-15-example.json");
        let report = save_pair(
            &output,
            bytes,
            serde_json::json!({"arrangement_report_version":2}),
        )
        .unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), bytes);
        assert_eq!(report["score_file"], output.to_string_lossy().as_ref());
        assert!(save_pair(&output, b"overwrite", serde_json::json!({})).is_err());
        let other = snapshot.path.parent().unwrap().join("blocked.json");
        let collision = format!("{}.arrangement.json", other.display());
        std::fs::write(&collision, b"user report").unwrap();
        assert!(save_pair(&other, bytes, serde_json::json!({})).is_err());
        assert!(!other.exists());
        assert_eq!(std::fs::read(&collision).unwrap(), b"user report");
        std::fs::remove_file(output).unwrap();
        std::fs::remove_file(report["report_file"].as_str().unwrap()).unwrap();
        std::fs::remove_file(collision).unwrap();
    }
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
            .args(arrangement_args(
                &snapshot.path,
                &output,
                None,
                &Options::default(),
            ))
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
        assert_eq!(report["kept"], 52);
        assert_eq!(report["dropped"], 0);
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
        verify_score(&report, &loaded).unwrap();
        let mut changed = report.clone();
        changed["note_mappings"][0]["projected_start_tick"] = serde_json::json!(999);
        assert!(verify_score(&changed, &loaded).is_err());
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
            .args(arrangement_args(
                &snapshot.path,
                &output,
                Some(127),
                &Options::default(),
            ))
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
