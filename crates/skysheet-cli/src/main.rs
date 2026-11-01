mod args;
mod midi;

use args::Command;
use serde::Serialize;
use serde_json::{Value, json};
use skysheet_score::Score;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const INVALID_SCORE: u8 = 1;
const USAGE: u8 = 2;
const IO_ERROR: u8 = 3;
const INTERNAL: u8 = 4;

#[derive(Debug, Serialize)]
struct Diagnostic {
    kind: &'static str,
    message: String,
    issues: Vec<String>,
    line: Option<usize>,
    column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<Box<skysheet_midi::Report>>,
    #[serde(skip)]
    exit_code: u8,
}

impl Diagnostic {
    fn new(kind: &'static str, message: impl Into<String>, exit_code: u8) -> Self {
        Self {
            kind,
            message: message.into(),
            issues: Vec::new(),
            line: None,
            column: None,
            report: None,
            exit_code,
        }
    }
}

#[derive(Serialize)]
struct Response {
    output_version: u8,
    ok: bool,
    command: Option<&'static str>,
    path: Option<String>,
    exit_code: u8,
    result: Option<Value>,
    error: Option<Diagnostic>,
}

fn read_score(path: &Path) -> Result<Score, Diagnostic> {
    let bytes = std::fs::read(path).map_err(|error| {
        let kind = if error.kind() == io::ErrorKind::NotFound {
            "file_not_found"
        } else {
            "file_read"
        };
        Diagnostic::new(kind, error.to_string(), IO_ERROR)
    })?;
    // This is the production raw-byte entry point. Never put Value between the
    // file and Score: that would lose tempo digits and duplicate object members.
    let score: Score = serde_json::from_slice(&bytes).map_err(|error| {
        use serde_json::error::Category;
        let (kind, code) = match error.classify() {
            Category::Syntax | Category::Eof => ("json_syntax", INVALID_SCORE),
            Category::Data => ("score_structure", INVALID_SCORE),
            Category::Io => ("file_read", IO_ERROR),
        };
        let mut diagnostic = Diagnostic::new(kind, error.to_string(), code);
        diagnostic.line = (error.line() > 0).then_some(error.line());
        diagnostic.column = (error.column() > 0).then_some(error.column());
        diagnostic
    })?;
    score.validate().map_err(|error| {
        let mut diagnostic = Diagnostic::new(
            "score_validation",
            "Score semantic validation failed",
            INVALID_SCORE,
        );
        diagnostic.issues = error.issues;
        diagnostic
    })?;
    Ok(score)
}

fn inspect(score: &Score) -> Value {
    // Output-only projection. Core types are serialized directly where useful;
    // this summary is not a second score model or an input deserializer.
    json!({
        "format_version": score.format_version,
        "metadata": score.metadata,
        "instrument": score.instrument,
        "timeline": score.timeline,
        "track_count": score.tracks.len(),
        "note_count": score.tracks.iter().map(|track| track.notes.len()).sum::<usize>(),
        "tracks": score.tracks.iter().map(|track| json!({
            "id": track.id, "name": track.name, "role": track.role,
            "note_count": track.notes.len()
        })).collect::<Vec<_>>()
    })
}

fn human_inspect(score: &Score, path: &Path) -> String {
    let mut text = format!(
        "File: {:?}\nScore: {:?} ({:?})\nFormat version: {}\nInstrument: {:?} ({:?}), {} keys\nTempo: {} quarter notes/min\nPPQ: {}\nTime signature: {}/{}\nTracks: {}\nNotes: {}\n",
        path,
        score.metadata.name,
        score.metadata.id,
        score.format_version,
        score.instrument.name,
        score.instrument.id,
        score.instrument.keys.len(),
        score.timeline.tempo,
        score.timeline.ppq,
        score.timeline.time_signature.numerator,
        score.timeline.time_signature.denominator,
        score.tracks.len(),
        score
            .tracks
            .iter()
            .map(|track| track.notes.len())
            .sum::<usize>()
    );
    for track in &score.tracks {
        text.push_str(&format!(
            "  {:?} ({:?}): {} notes\n",
            track.name,
            track.id,
            track.notes.len()
        ));
    }
    text.push_str("Validation: OK\n");
    text
}

fn execute(args: &args::Args) -> Result<(Value, String), Diagnostic> {
    match args.command {
        Command::ImportMidi | Command::ExportMidi | Command::InspectMidi => midi::execute(args),
        Command::Help => Ok((json!({"help": args::HELP}), args::HELP.into())),
        Command::Version => Ok((
            json!({"name": "skysheet", "version": VERSION}),
            format!("skysheet {VERSION}\n"),
        )),
        Command::Inspect | Command::Validate => {
            let path = args.file.as_deref().ok_or_else(|| {
                Diagnostic::new("internal", "missing parsed input path", INTERNAL)
            })?;
            let score = read_score(path)?;
            if args.command == Command::Inspect {
                Ok((inspect(&score), human_inspect(&score, path)))
            } else {
                Ok((json!({"valid": true}), format!("Valid score: {path:?}\n")))
            }
        }
    }
}

fn write_stdout(bytes: &[u8]) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes)?;
    stdout.flush()
}

fn emit(response: &Response, human: &str, json_mode: bool) -> u8 {
    if json_mode {
        let mut encoded = match serde_json::to_vec_pretty(response) {
            Ok(encoded) => encoded,
            Err(error) => {
                let _ = writeln!(
                    io::stderr().lock(),
                    "internal: cannot encode result: {error}"
                );
                return INTERNAL;
            }
        };
        encoded.push(b'\n');
        if let Err(error) = write_stdout(&encoded) {
            let _ = writeln!(
                io::stderr().lock(),
                "output_io: cannot write stdout: {error}"
            );
            return IO_ERROR;
        }
    } else if let Some(error) = &response.error {
        let mut stderr = io::stderr().lock();
        let _ = writeln!(
            stderr,
            "{}: {}{}",
            error.kind,
            response
                .path
                .as_ref()
                .map(|p| format!("{p:?}: "))
                .unwrap_or_default(),
            error.message
        );
        for issue in &error.issues {
            let _ = writeln!(stderr, "  - {issue}");
        }
        if let Some(report) = &error.report {
            for d in &report.diagnostics {
                let _ = writeln!(
                    stderr,
                    "  [{}] {}: {} (track {:?}, event {:?}, tick {:?})",
                    d.severity, d.code, d.message, d.track, d.event, d.source_tick
                );
            }
        }
    } else if let Err(error) = write_stdout(human.as_bytes()) {
        let _ = writeln!(
            io::stderr().lock(),
            "output_io: cannot write stdout: {error}"
        );
        return IO_ERROR;
    }
    response.exit_code
}

fn main() -> ExitCode {
    let raw = std::env::args_os().skip(1).collect::<Vec<_>>();
    let json_mode = args::json_requested(&raw);
    let parsed = args::parse(&raw);
    let command = parsed.as_ref().ok().map(|args| args.command.name());
    let path = parsed
        .as_ref()
        .ok()
        .and_then(|args| args.file.as_ref())
        .map(|p| p.to_string_lossy().into_owned());
    let outcome = parsed
        .map_err(|message| Diagnostic::new("usage", message, USAGE))
        .and_then(|args| execute(&args));
    let (result, error, human, exit_code) = match outcome {
        Ok((result, human)) => (Some(result), None, human, 0),
        Err(error) => {
            let code = error.exit_code;
            (None, Some(error), String::new(), code)
        }
    };
    let response = Response {
        output_version: 1,
        ok: error.is_none(),
        command,
        path,
        exit_code,
        result,
        error,
    };
    ExitCode::from(emit(&response, &human, json_mode))
}
