use crate::{ArrangementDiagnostic, Diagnostic, INTERNAL, INVALID_SCORE, IO_ERROR, args::Args};
use serde_json::{Value, json};
use skysheet_arrange::TransposeMode;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

fn report_path(args: &Args, output: &Path) -> PathBuf {
    args.report.clone().unwrap_or_else(|| {
        let mut name = output.as_os_str().to_os_string();
        name.push(".arrangement.json");
        PathBuf::from(name)
    })
}

fn check_destinations(output: &Path, report: &Path) -> Result<(), Diagnostic> {
    let resolve = |path: &Path| -> Result<PathBuf, Diagnostic> {
        if fs::symlink_metadata(path).is_ok() {
            return Err(Diagnostic::new(
                "file_write",
                format!(
                    "{} already exists; refusing overwrite or input alias",
                    path.display()
                ),
                IO_ERROR,
            ));
        }
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let parent = fs::canonicalize(parent).map_err(|e| {
            Diagnostic::new("file_write", format!("{}: {e}", parent.display()), IO_ERROR)
        })?;
        let name = path.file_name().ok_or_else(|| {
            Diagnostic::new("file_write", "output requires a file name", IO_ERROR)
        })?;
        Ok(parent.join(name))
    };
    let a = resolve(output)?;
    let b = resolve(report)?;
    let aliases = a == b
        || (cfg!(windows)
            && a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase());
    if aliases {
        return Err(Diagnostic::new(
            "file_write",
            "Score and report destinations must be distinct",
            IO_ERROR,
        ));
    }
    Ok(())
}

// Both byte buffers are fully encoded/validated before reserving either path.
// create_new rejects existing files, symlinks, hardlinks and input aliases.
fn write_pair(output: &Path, score: &[u8], report: &Path, diagnostic: &[u8]) -> Result<(), String> {
    let mut a = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| {
            format!(
                "{}: {e}; existing files are never overwritten",
                output.display()
            )
        })?;
    let mut b = match OpenOptions::new().write(true).create_new(true).open(report) {
        Ok(file) => file,
        Err(e) => {
            drop(a);
            let cleanup = fs::remove_file(output);
            return Err(format!(
                "{}: {e}; Score cleanup: {cleanup:?}",
                report.display()
            ));
        }
    };
    let written = a
        .write_all(score)
        .and_then(|_| a.sync_all())
        .and_then(|_| b.write_all(diagnostic))
        .and_then(|_| b.sync_all());
    drop(a);
    drop(b);
    if let Err(e) = written {
        let ca = fs::remove_file(output);
        let cb = fs::remove_file(report);
        return Err(format!(
            "write failed: {e}; Score cleanup: {ca:?}; report cleanup: {cb:?}"
        ));
    }
    Ok(())
}

pub(crate) fn execute(args: &Args) -> Result<(Value, String), Diagnostic> {
    let input = args
        .file
        .as_deref()
        .ok_or_else(|| Diagnostic::new("internal", "missing input", INTERNAL))?;
    let output = args
        .output
        .as_deref()
        .ok_or_else(|| Diagnostic::new("internal", "missing output", INTERNAL))?;
    let report_path = report_path(args, output);
    check_destinations(output, &report_path)?;
    let prepare = || {
        let doc = skysheet_midi::read_music(
            &crate::midi::read_midi(input)?,
            input.file_name().map(|n| n.to_string_lossy().into_owned()),
        )
        .map_err(crate::midi::failure)?;
        let projection = skysheet_midi::project_music(&doc).map_err(crate::midi::failure)?;
        Ok::<_, Diagnostic>((doc, projection))
    };
    let (doc, projection) = prepare().map_err(|mut error| {
        let mut report = json!({
            "arrangement_report_version":2, "stage":"preparation_failed",
            "source_file":input.file_name().map(|n|n.to_string_lossy()), "source_document_id":null,
            "target_instrument":skysheet_score::SKY_15_INSTRUMENT_ID,
            "requested_transpose_semitones":args.transpose,
            "original_notes":null,"kept":null,"dropped":null,
            "score_generated":false,"score_file":null,"report_file":report_path.to_string_lossy(),
            "score_validation":false,"time_preserved":false,"pitch_mapping_valid":false,
            "error_kind":error.kind,"message":error.message,"midi_report":error.report
        });
        let saved = serde_json::to_vec_pretty(&report)
            .map_err(|e| e.to_string())
            .and_then(|bytes| crate::midi::write_new(&report_path, &bytes).map_err(|e| e.message));
        match saved {
            Ok(()) => error
                .issues
                .push(format!("Preparation report: {}", report_path.display())),
            Err(e) => {
                report["report_file"] = Value::Null;
                error
                    .message
                    .push_str(&format!("; could not save preparation report: {e}"));
            }
        }
        error.arrangement_report = Some(Box::new(ArrangementDiagnostic::Preparation(report)));
        error
    })?;
    let mode = args
        .transpose
        .map_or(TransposeMode::Auto, TransposeMode::Fixed);
    let mut result =
        skysheet_arrange::arrange_with_options(&doc, &projection, mode, &args.arrangement)
            .map_err(|e| {
                let mut diagnostic = Diagnostic::new("arrangement", e.message, INVALID_SCORE);
                let mut report = e.report;
                if let Some(report) = &mut report {
                    report.report_file = Some(report_path.to_string_lossy().into_owned());
                    let saved = serde_json::to_vec_pretty(report)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| {
                            crate::midi::write_new(&report_path, &bytes).map_err(|e| e.message)
                        });
                    if let Err(e) = saved {
                        report.report_file = None;
                        diagnostic
                            .message
                            .push_str(&format!("; diagnostic report could not be saved: {e}"));
                    }
                }
                diagnostic.arrangement_report =
                    report.map(|r| Box::new(ArrangementDiagnostic::Complete(r)));
                diagnostic
            })?;
    result.report.score_file = Some(output.to_string_lossy().into_owned());
    result.report.report_file = Some(report_path.to_string_lossy().into_owned());
    let encode = |e: serde_json::Error| Diagnostic::new("internal", e.to_string(), INTERNAL);
    let bytes = serde_json::to_vec_pretty(&result.score).map_err(encode)?;
    // Read these exact output bytes through the production N2 reader.
    let reread: skysheet_score::Score = serde_json::from_slice(&bytes).map_err(encode)?;
    reread
        .validate()
        .map_err(|e| Diagnostic::new("score_validation", e.to_string(), INVALID_SCORE))?;
    let diagnostic = serde_json::to_vec_pretty(&result.report).map_err(encode)?;
    if diagnostic.len() > 64 * 1024 * 1024 {
        return Err(Diagnostic::new(
            "arrangement_resource_limit",
            "report exceeds 64 MiB; no output files created",
            INVALID_SCORE,
        ));
    }
    if let Err(e) = write_pair(output, &bytes, &report_path, &diagnostic) {
        result.report.stage = "file_write_failed";
        result.report.score_generated = false;
        result.report.score_file = None;
        result.report.report_file = None;
        let mut error = Diagnostic::new("file_write", e, IO_ERROR);
        error.arrangement_report = Some(Box::new(ArrangementDiagnostic::Complete(Box::new(
            result.report,
        ))));
        return Err(error);
    }
    let melody = &result.report.role_retention[0];
    let support_total: usize = result.report.role_retention[1..5]
        .iter()
        .map(|r| r.total)
        .sum();
    let support_kept: usize = result.report.role_retention[1..5]
        .iter()
        .map(|r| r.kept)
        .sum();
    let mut human = format!(
        "arrange: {}\nOutput: {}\nReport: {}\nGlobal transpose: {} semitones ({})\nRetained: {}/{}; dropped: {}\nMelody: {:?}/{:?}; accompaniment: {}/{}\nScore validation: OK; D6 projection time preserved: true\n{}\n",
        input.display(),
        output.display(),
        report_path.display(),
        result.report.transpose_semitones.unwrap(),
        result.report.arrangement_mode,
        result.report.kept,
        result.report.original_notes,
        result.report.dropped,
        melody.kept,
        melody.total,
        support_kept,
        support_total,
        result.report.melody.reason
    );
    for n in result.report.note_mappings.iter().filter(|n| !n.mapped) {
        human.push_str(&format!("Dropped track {} event {:?} {}: {} ({}) -> MIDI {}, source tick {} duration {}, reason {:?}\n",n.source_track,n.source_event_index,n.source_note_id,n.source_pitch,n.source_midi,n.target_midi,n.source_start_tick,n.source_duration_ticks,n.drop_reason));
    }
    Ok((
        json!({"output_path":output.to_string_lossy(),"report_path":report_path.to_string_lossy(),"report":result.report}),
        human,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_report_collision_cleans_only_our_reserved_score() {
        let dir =
            std::env::temp_dir().join(format!("skysheet-arrange-write-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let score = dir.join("score.json");
        let report = dir.join("report.json");
        fs::write(&report, b"existing report").unwrap();
        let error = write_pair(&score, b"score", &report, b"report").unwrap_err();
        assert!(error.contains("cleanup"));
        assert!(!score.exists());
        assert_eq!(fs::read(&report).unwrap(), b"existing report");
        fs::remove_file(report).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
