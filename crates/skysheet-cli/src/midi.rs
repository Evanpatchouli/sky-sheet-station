use crate::{
    Diagnostic, INTERNAL, INVALID_SCORE, IO_ERROR,
    args::{Args, Command},
    read_score,
};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
};

fn failure(error: skysheet_midi::Failure) -> Diagnostic {
    let mut diagnostic = Diagnostic::new("midi_conversion", error.to_string(), INVALID_SCORE);
    diagnostic.report = Some(error.report);
    diagnostic
}

fn read_midi(path: &Path) -> Result<Vec<u8>, Diagnostic> {
    let file =
        fs::File::open(path).map_err(|e| Diagnostic::new("file_read", e.to_string(), IO_ERROR))?;
    let mut bytes = Vec::new();
    file.take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Diagnostic::new("file_read", e.to_string(), IO_ERROR))?;
    Ok(bytes)
}

// create_new refuses existing files, symlinks and hard-link aliases of input.
// Conversion and encoding have already succeeded before this function is called.
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Diagnostic> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            Diagnostic::new(
                "file_write",
                format!(
                    "{}: {e}; existing destinations are never overwritten",
                    path.display()
                ),
                IO_ERROR,
            )
        })?;
    let result = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    if let Err(e) = result {
        let cleanup = fs::remove_file(path);
        return Err(Diagnostic::new(
            "file_write",
            format!("{e}; partial-output cleanup: {cleanup:?}"),
            IO_ERROR,
        ));
    }
    Ok(())
}

pub(crate) fn execute(args: &Args) -> Result<(Value, String), Diagnostic> {
    let input = args
        .file
        .as_deref()
        .ok_or_else(|| Diagnostic::new("internal", "missing input", INTERNAL))?;
    let (bytes, report) = match args.command {
        Command::ImportMidi => {
            let options = skysheet_midi::ImportOptions {
                reference_tempo: args.tempo,
                name: input.file_stem().map(|s| s.to_string_lossy().into_owned()),
            };
            let (score, report) =
                skysheet_midi::import(&read_midi(input)?, &options).map_err(failure)?;
            let bytes = serde_json::to_vec_pretty(&score)
                .map_err(|e| Diagnostic::new("internal", e.to_string(), INTERNAL))?;
            (bytes, report)
        }
        Command::ExportMidi => skysheet_midi::export(&read_score(input)?).map_err(failure)?,
        Command::InspectMidi => (
            Vec::new(),
            skysheet_midi::inspect(&read_midi(input)?).map_err(failure)?,
        ),
        _ => {
            return Err(Diagnostic::new(
                "internal",
                "unexpected MIDI command",
                INTERNAL,
            ));
        }
    };
    if let Some(output) = &args.output
        && let Err(mut e) = write_new(output, &bytes)
    {
        e.report = Some(Box::new(report));
        return Err(e);
    }
    let mut human = format!(
        "{}: {}\nTracks: {}\nNotes: {}\n",
        args.command.name(),
        input.display(),
        report.track_count,
        report.note_count
    );
    if let Some(path) = &args.output {
        human.push_str(&format!("Output: {}\n", path.display()));
    }
    for d in &report.diagnostics {
        human.push_str(&format!(
            "[{}] {}: {} (track {:?}, event {:?}, tick {:?})\n",
            d.severity, d.code, d.message, d.track, d.event, d.source_tick
        ));
    }
    Ok((
        json!({"output_path": args.output.as_ref().map(|p| p.to_string_lossy()), "report": report}),
        human,
    ))
}
