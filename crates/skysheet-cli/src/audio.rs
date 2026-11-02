use crate::{Diagnostic, INTERNAL, INVALID_SCORE, IO_ERROR, args::Args};
use serde_json::{Value, json};
use std::{io::Write, path::PathBuf};

pub fn execute(args: &Args) -> Result<(Value, String), Diagnostic> {
    let input = args.file.as_ref().unwrap();
    let output = args.output.as_ref().unwrap();
    let report_path = args.report.clone().unwrap_or_else(|| {
        let mut name = output.as_os_str().to_os_string();
        name.push(".transcription.json");
        PathBuf::from(name)
    });
    crate::arrange::check_destinations(output, &report_path)?;
    let result = skysheet_audio::transcribe(input, &args.audio, &|progress| {
        if args.progress {
            let mut stderr = std::io::stderr().lock();
            if let Ok(line) = serde_json::to_string(&progress) {
                let _ = writeln!(stderr, "{line}");
            }
        }
        true
    })
    .map_err(|e| {
        let code = if e.code == "audio_read" {
            IO_ERROR
        } else {
            INVALID_SCORE
        };
        Diagnostic::new(e.code, e.message, code)
    })?;
    // Validate the actual SMF through the existing source reader before writing.
    let source = skysheet_midi::read_music(
        &result.midi,
        Some(
            input
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
    )
    .map_err(|e| Diagnostic::new("audio_midi", e.to_string(), INTERNAL))?;
    let report = serde_json::to_vec_pretty(&result.report)
        .map_err(|e| Diagnostic::new("internal", e.to_string(), INTERNAL))?;
    crate::arrange::write_pair(output, &result.midi, &report_path, &report)
        .map_err(|e| Diagnostic::new("file_write", e, IO_ERROR))?;
    Ok((
        json!({
            "output_path": output.to_string_lossy(), "report_path": report_path.to_string_lossy(),
            "source_document_id": source.id, "report": result.report,
        }),
        format!(
            "Transcribed {} notes from {}\nMIDI: {}\nReport: {}\nReview candidate notes before arrangement.\n",
            source.notes.len(),
            input.display(),
            output.display(),
            report_path.display()
        ),
    ))
}
