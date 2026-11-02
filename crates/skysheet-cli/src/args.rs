use std::ffi::OsString;
use std::path::PathBuf;

pub const HELP: &str = "SkySheet CLI Alpha — inspect and validate score JSON

Usage:
  skysheet --help
  skysheet --version
  skysheet inspect <FILE> [--json]
  skysheet validate <FILE> [--json]
  skysheet import-midi <FILE> -o <OUTPUT.json> [--tempo <BPM>] [--json]
  skysheet export-midi <FILE> -o <OUTPUT.mid> [--json]
  skysheet inspect-midi <FILE> [--json]
  skysheet analyze-midi <FILE> [--json]
  skysheet inspect-music <FILE.mid> [--json]
  skysheet analyze <FILE.mid> [--json]
  skysheet arrange <FILE.mid> -o <OUTPUT.json> [--auto | --transpose <SEMITONES>] [--json]
  skysheet transcribe <FILE.mp3|wav|flac> -o <OUTPUT.mid> [--json] [--progress]

Options:
  -h, --help     Show help (also accepted with inspect/validate)
  -V, --version  Show program version
  --json         Emit one JSON result, including errors
  -o, --output   New output file (existing destinations are refused)
  --tempo        Import reference BPM (default: effective source tempo at zero)
  --auto         Search every global semitone offset (arrange default)
  --transpose    Fixed global offset -127..127; never replaced by auto search
  --report       New report path (default: OUTPUT.arrangement.json / OUTPUT.transcription.json)
  --mode         melody-first (default), simple-accompaniment, polyphonic
  --no-octave-shifts  Disable context octave placement (global-only pitch mapping)
  --melody-priority / --bass-priority / --accompaniment-priority  Integer 0..5 (3/2/1)
  --max-polyphony User overlap limit 1..128 (default unlimited; not a Score rule)
  --progress     Transcription progress as JSON lines on stderr (stdout stays clean)
  --onset-threshold / --frame-threshold  Transcription sensitivity 0.05..0.95 (0.5/0.3)
  --minimum-note-ms  Transcription minimum note length 30..1000 (127.7 ms)
  --             Treat remaining arguments as positional (for filenames starting with -)

Score inputs use original JSON and Score validation; input files are never modified.
Exit codes: 0 success; 1 invalid/unconvertible input; 2 usage; 3 I/O; 4 internal error.
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Inspect,
    Validate,
    ImportMidi,
    ExportMidi,
    InspectMidi,
    AnalyzeMidi,
    InspectMusic,
    Analyze,
    Arrange,
    Transcribe,
}

impl Command {
    pub fn name(self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Version => "version",
            Self::Inspect => "inspect",
            Self::Validate => "validate",
            Self::ImportMidi => "import-midi",
            Self::ExportMidi => "export-midi",
            Self::InspectMidi => "inspect-midi",
            Self::AnalyzeMidi => "analyze-midi",
            Self::InspectMusic => "inspect-music",
            Self::Analyze => "analyze",
            Self::Arrange => "arrange",
            Self::Transcribe => "transcribe",
        }
    }
}

#[derive(Debug)]
pub struct Args {
    pub audio: skysheet_audio::Options,
    pub progress: bool,
    pub arrangement: skysheet_arrange::ArrangementOptions,
    pub command: Command,
    pub file: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub tempo: Option<f64>,
    pub transpose: Option<i16>,
    pub report: Option<PathBuf>,
}

// Detect output mode even when argument parsing fails. After --, --json is a filename.
pub fn json_requested(args: &[OsString]) -> bool {
    args.iter()
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--json")
}

pub fn parse(args: &[OsString]) -> Result<Args, String> {
    let mut positional = Vec::new();
    let mut literal = false;
    let mut help = false;
    let mut version = false;
    let mut json = false;
    let mut output = None;
    let mut tempo = None;
    let mut transpose = None;
    let mut report = None;
    let mut auto = false;
    let mut arrangement = skysheet_arrange::ArrangementOptions::default();
    let mut audio = skysheet_audio::Options::default();
    let mut progress = false;
    let mut audio_flags = std::collections::BTreeSet::new();
    let mut arrangement_flags = std::collections::BTreeSet::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if !literal {
            match arg.to_str() {
                Some(
                    flag @ ("--onset-threshold" | "--frame-threshold" | "--minimum-note-ms"
                    | "--progress"),
                ) => {
                    if !audio_flags.insert(flag) {
                        return Err(format!("repeated option {flag}"));
                    }
                    if flag == "--progress" {
                        progress = true;
                        continue;
                    }
                    let value: f64 = iter
                        .next()
                        .and_then(|s| s.to_str())
                        .ok_or_else(|| format!("missing value for {flag}"))?
                        .parse()
                        .map_err(|_| format!("invalid value for {flag}"))?;
                    match flag {
                        "--onset-threshold" => audio.onset_threshold = value,
                        "--frame-threshold" => audio.frame_threshold = value,
                        _ => audio.minimum_note_ms = value,
                    }
                    audio.validate().map_err(|e| e.to_string())?;
                    continue;
                }
                Some(
                    flag @ ("--mode"
                    | "--no-octave-shifts"
                    | "--melody-priority"
                    | "--bass-priority"
                    | "--accompaniment-priority"
                    | "--max-polyphony"),
                ) => {
                    if !arrangement_flags.insert(flag) {
                        return Err(format!("repeated option {flag}"));
                    }
                    if flag == "--no-octave-shifts" {
                        arrangement.allow_octave_shifts = false;
                        continue;
                    }
                    let value = iter
                        .next()
                        .and_then(|s| s.to_str())
                        .ok_or_else(|| format!("missing value for {flag}"))?;
                    if flag == "--mode" {
                        arrangement.mode =
                            match value {
                                "melody-first" => skysheet_arrange::ArrangementMode::MelodyFirst,
                                "simple-accompaniment" => {
                                    skysheet_arrange::ArrangementMode::SimpleAccompaniment
                                }
                                "polyphonic" => skysheet_arrange::ArrangementMode::Polyphonic,
                                _ => return Err(
                                    "mode must be melody-first, simple-accompaniment or polyphonic"
                                        .into(),
                                ),
                            };
                    } else if flag == "--max-polyphony" {
                        arrangement.maximum_polyphony = Some(
                            value
                                .parse()
                                .map_err(|_| "max-polyphony must be an integer in 1..128")?,
                        );
                    } else {
                        let priority = value
                            .parse()
                            .map_err(|_| "priority must be an integer in 0..5")?;
                        match flag {
                            "--melody-priority" => arrangement.melody_priority = priority,
                            "--bass-priority" => arrangement.bass_priority = priority,
                            _ => arrangement.accompaniment_priority = priority,
                        }
                    }
                    arrangement.validate()?;
                    continue;
                }
                Some("--") => {
                    literal = true;
                    continue;
                }
                Some("--json") if !json => {
                    json = true;
                    continue;
                }
                Some("-o" | "--output") if output.is_none() => {
                    output = Some(PathBuf::from(iter.next().ok_or("missing output path")?));
                    continue;
                }
                Some("--tempo") if tempo.is_none() => {
                    let value: f64 = iter
                        .next()
                        .and_then(|s| s.to_str())
                        .ok_or("missing tempo")?
                        .parse()
                        .map_err(|_| "invalid reference tempo")?;
                    if !value.is_finite() || value <= 0.0 {
                        return Err("tempo must be finite and positive".into());
                    }
                    tempo = Some(value);
                    continue;
                }
                Some("--auto") if !auto => {
                    auto = true;
                    continue;
                }
                Some("--report") if report.is_none() => {
                    report = Some(PathBuf::from(iter.next().ok_or("missing report path")?));
                    continue;
                }
                Some("--transpose") if transpose.is_none() => {
                    let value: i16 = iter
                        .next()
                        .and_then(|s| s.to_str())
                        .ok_or("missing transpose semitones")?
                        .parse()
                        .map_err(|_| "transpose must be an integer in -127..127")?;
                    if !(-127..=127).contains(&value) {
                        return Err("transpose must be in -127..127".into());
                    }
                    transpose = Some(value);
                    continue;
                }
                Some("--help" | "-h") if !help => {
                    help = true;
                    continue;
                }
                Some("--version" | "-V") if !version => {
                    version = true;
                    continue;
                }
                _ if arg.to_string_lossy().starts_with('-') => {
                    return Err(format!("unknown or repeated option {arg:?}; use --help"));
                }
                _ => {}
            }
        }
        positional.push(arg);
    }
    if help && version {
        return Err("--help and --version cannot be combined".into());
    }
    if positional.is_empty() {
        let command = if help {
            Command::Help
        } else if version {
            Command::Version
        } else {
            return Err("missing command; use --help".into());
        };
        return Ok(Args {
            audio,
            progress,
            arrangement,
            command,
            file: None,
            output: None,
            tempo: None,
            transpose: None,
            report: None,
        });
    }
    let command = match positional[0].to_str() {
        Some("inspect") => Command::Inspect,
        Some("validate") => Command::Validate,
        Some("import-midi") => Command::ImportMidi,
        Some("export-midi") => Command::ExportMidi,
        Some("inspect-midi") => Command::InspectMidi,
        Some("analyze-midi") => Command::AnalyzeMidi,
        Some("inspect-music") => Command::InspectMusic,
        Some("analyze") => Command::Analyze,
        Some("arrange") => Command::Arrange,
        Some("transcribe") => Command::Transcribe,
        _ => {
            return Err(format!(
                "unsupported command {:?}; use --help",
                positional[0]
            ));
        }
    };
    if version {
        return Err("--version must be used without a subcommand".into());
    }
    if help && positional.len() == 1 {
        return Ok(Args {
            audio,
            progress,
            arrangement,
            command: Command::Help,
            file: None,
            output: None,
            tempo: None,
            transpose: None,
            report: None,
        });
    }
    if positional.len() != 2 || help {
        return Err(format!(
            "usage: skysheet {} <FILE> [--json]",
            command.name()
        ));
    }
    if matches!(
        command,
        Command::ImportMidi | Command::ExportMidi | Command::Arrange | Command::Transcribe
    ) {
        if output.is_none() {
            return Err("conversion requires -o <OUTPUT>".into());
        }
    } else if output.is_some() {
        return Err("--output is only valid for import-midi/export-midi/arrange/transcribe".into());
    }
    if tempo.is_some() && command != Command::ImportMidi {
        return Err("--tempo is only valid for import-midi".into());
    }
    if !audio_flags.is_empty() && command != Command::Transcribe {
        return Err("transcription options are only valid for transcribe".into());
    }
    if report.is_some() && !matches!(command, Command::Arrange | Command::Transcribe) {
        return Err("--report is only valid for arrange/transcribe".into());
    }
    if (auto || transpose.is_some() || !arrangement_flags.is_empty()) && command != Command::Arrange
    {
        return Err("arrangement options are only valid for arrange".into());
    }
    if auto && transpose.is_some() {
        return Err("--auto and --transpose cannot be combined".into());
    }
    Ok(Args {
        audio,
        progress,
        arrangement,
        command,
        file: Some(PathBuf::from(positional[1])),
        output,
        tempo,
        transpose,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_option_filename_does_not_enable_json() {
        let args = ["inspect", "--", "--json"].map(OsString::from);
        assert!(!json_requested(&args));
        assert_eq!(parse(&args).unwrap().file.unwrap(), PathBuf::from("--json"));
    }

    #[test]
    fn invalid_arguments_still_select_machine_diagnostics() {
        let args = ["unknown", "--json"].map(OsString::from);
        assert!(json_requested(&args));
        assert!(parse(&args).is_err());
    }
}
