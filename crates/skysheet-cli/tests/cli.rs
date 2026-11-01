use serde::Deserialize;
use serde_json::Value;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const EXAMPLE: &str = include_str!("../../../docs/examples/sky-15-example.json");

#[test]
fn required_music_commands_have_versioned_reports_and_stable_error_categories() {
    let sandbox = Sandbox::new();
    let path = sandbox.file(
        "欢乐颂.mid",
        include_bytes!("../../../audios/ode-to-joy.mid"),
    );
    for command in ["inspect-music", "analyze"] {
        let r = machine(&run(command, &path, true), 0);
        assert_eq!(r["command"], command);
        assert_eq!(r["result"]["report"]["report_version"], 2);
        assert_eq!(r["result"]["report"]["note_statistics"]["total_notes"], 52);
        assert_eq!(
            r["result"]["report"]["structure_analysis"]["tracks"][1]["pitch_range"]["lowest_name"],
            "C5"
        );
        let human = run(command, &path, false);
        assert!(human.status.success());
        assert!(human.stderr.is_empty());
        let text = String::from_utf8(human.stdout).unwrap();
        assert!(text.contains("Tempo Map"));
        assert!(text.contains("Meter Map"));
        machine(&run(command, &sandbox.0.join("missing.mid"), true), 3);
        let bad = sandbox.file("bad.mid", b"broken");
        machine(&run(command, &bad, true), 1);
        machine(&cli(&[OsStr::new(command), OsStr::new("--json")]), 2);
        let output = sandbox.0.join("not-created.json");
        machine(&convert(command, &path, &output), 2);
        assert!(!output.exists());
        if command == "analyze" {
            assert_eq!(r["result"]["report"]["track_ranking"][0]["track"], 1);
            assert!(text.contains("continuity"));
        } else {
            assert!(r["result"]["report"].get("melody_candidates").is_none());
        }
    }
}

#[test]
fn analyze_midi_preserves_source_and_existing_cli_contracts() {
    let sandbox = Sandbox::new();
    let raw = include_bytes!("../../../audios/ode-to-joy.mid");
    let path = sandbox.file("欢乐颂 & source.mid", raw);
    let response = machine(&run("analyze-midi", &path, true), 0);
    let report = &response["result"]["report"];
    assert_eq!(response["command"], "analyze-midi");
    assert_eq!(report["report_version"], 2);
    assert_eq!(report["note_statistics"]["total_notes"], 52);
    assert_eq!(report["source"]["notes"].as_array().unwrap().len(), 52);
    assert_eq!(report["source"]["ppq"], 480);
    assert!(report["confidence"]["probability"].is_null());
    assert!(report["source"].get("original_midi").is_none());
    assert!(response["result"].get("score").is_none());
    let human = run("analyze-midi", &path, false);
    assert_eq!(human.status.code(), Some(0));
    assert!(human.stderr.is_empty());
    assert!(
        String::from_utf8(human.stdout)
            .unwrap()
            .contains("Notes: 52")
    );
    assert_eq!(std::fs::read(&path).unwrap(), raw);
    machine(&run("inspect-midi", &path, true), 0);
    let output = sandbox.0.join("unchanged-policy.json");
    machine(&convert("import-midi", &path, &output), 1);
    assert!(!output.exists());
    machine(&convert("analyze-midi", &path, &output), 2);
    assert!(!output.exists());
    machine(
        &cli(&[
            OsStr::new("analyze-midi"),
            path.as_os_str(),
            OsStr::new("--tempo"),
            OsStr::new("60"),
            OsStr::new("--json"),
        ]),
        2,
    );
    machine(
        &run("analyze-midi", &sandbox.0.join("missing.mid"), true),
        3,
    );
    let bad = sandbox.file("bad.mid", b"broken");
    machine(&run("analyze-midi", &bad, true), 1);
}

fn convert(command: &str, input: &Path, output: &Path) -> Output {
    cli(&[
        OsStr::new(command),
        input.as_os_str(),
        OsStr::new("-o"),
        output.as_os_str(),
        OsStr::new("--json"),
    ])
}

#[test]
fn midi_cli_roundtrip_real_files_and_source_preservation() {
    let sandbox = Sandbox::new();
    let input = sandbox.file("原始 & score.json", EXAMPLE);
    let midi = sandbox.0.join("导出 & 空格.mid");
    let exported = machine(&convert("export-midi", &input, &midi), 0);
    assert_eq!(exported["result"]["report"]["note_count"], 6);
    let raw_midi = std::fs::read(&midi).unwrap();
    assert_eq!(&raw_midi[..4], b"MThd");
    machine(&run("inspect-midi", &midi, true), 0);
    let output = sandbox.0.join("导入.json");
    let imported = machine(&convert("import-midi", &midi, &output), 0);
    assert_eq!(imported["result"]["report"]["note_count"], 6);
    machine(&run("validate", &output, true), 0);
    assert_eq!(std::fs::read_to_string(input).unwrap(), EXAMPLE);
    assert_eq!(std::fs::read(midi).unwrap(), raw_midi);
}

#[test]
fn midi_cli_refuses_overwrites_aliases_and_missing_parents() {
    let sandbox = Sandbox::new();
    let input = sandbox.file("source.json", EXAMPLE);
    machine(&convert("export-midi", &input, &input), 3);
    let existing = sandbox.file("existing.mid", b"keep me");
    machine(&convert("export-midi", &input, &existing), 3);
    assert_eq!(std::fs::read(existing).unwrap(), b"keep me");
    let alias = sandbox.0.join("alias.mid");
    std::fs::hard_link(&input, &alias).unwrap();
    machine(&convert("export-midi", &input, &alias), 3);
    assert_eq!(std::fs::read_to_string(input).unwrap(), EXAMPLE);
    let missing = sandbox.0.join("missing").join("output.mid");
    machine(&convert("export-midi", &alias, &missing), 3);
    assert!(!missing.exists());
}

#[test]
fn midi_failures_have_reports_and_never_create_outputs() {
    let sandbox = Sandbox::new();
    let bad = sandbox.file("broken.mid", b"MThd");
    let output = sandbox.0.join("failed.json");
    let response = machine(&convert("import-midi", &bad, &output), 1);
    assert_eq!(response["error"]["report"]["report_version"], 1);
    assert!(!output.exists());
    let duplicate = sandbox.file(
        "duplicate.json",
        EXAMPLE.replacen("\"tempo\": 120", "\"tempo\": 120, \"tempo\": 120", 1),
    );
    machine(&convert("export-midi", &duplicate, &output), 1);
    assert!(!output.exists());
    let unsupported = sandbox.file("pitch.json", EXAMPLE.replace("\"C4\"", "\"C#4\""));
    machine(&convert("export-midi", &unsupported, &output), 1);
    assert!(!output.exists());
    machine(
        &cli(&[
            OsStr::new("import-midi"),
            bad.as_os_str(),
            OsStr::new("--json"),
        ]),
        2,
    );
    machine(
        &cli(&[
            OsStr::new("export-midi"),
            bad.as_os_str(),
            OsStr::new("-o"),
            output.as_os_str(),
            OsStr::new("--tempo"),
            OsStr::new("60"),
            OsStr::new("--json"),
        ]),
        2,
    );
}
static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "skysheet-cli-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn cli(args: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_skysheet"))
        .args(args)
        .output()
        .unwrap()
}

fn run(command: &str, path: &Path, json: bool) -> Output {
    let mut args = vec![OsStr::new(command), path.as_os_str()];
    if json {
        args.push(OsStr::new("--json"));
    }
    cli(&args)
}

fn machine(output: &Output, code: i32) -> Value {
    assert_eq!(output.status.code(), Some(code), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(!output.stdout.contains(&0x1b));
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["output_version"], 1);
    assert_eq!(result["exit_code"], code);
    assert_eq!(result["ok"], code == 0);
    if code == 0 {
        assert!(result["error"].is_null());
    } else {
        assert!(result["result"].is_null());
    }
    result
}

#[test]
fn help_and_version_are_real_commands() {
    for flag in ["--help", "-h", "--version", "-V"] {
        let output = cli(&[OsStr::new(flag)]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        if flag == "--help" || flag == "-h" {
            assert!(text.contains("skysheet inspect <FILE>"));
            assert!(text.contains("skysheet validate <FILE>"));
        } else {
            assert_eq!(text, format!("skysheet {}\n", env!("CARGO_PKG_VERSION")));
        }
        machine(&cli(&[OsStr::new(flag), OsStr::new("--json")]), 0);
    }
    assert!(
        cli(&[OsStr::new("inspect"), OsStr::new("--help")])
            .status
            .success()
    );
}

#[test]
fn inspect_and_validate_human_output_preserve_input() {
    let sandbox = Sandbox::new();
    let path = sandbox.file("乐谱 example.json", EXAMPLE);
    let output = run("inspect", &path, false);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "Format version: 1.0",
        "Tempo: 120",
        "PPQ: 960",
        "Time signature: 4/4",
        "Tracks: 2",
        "Notes: 6",
        "Melody",
        "Harmony",
        "3 notes",
        "Validation: OK",
    ] {
        assert!(text.contains(expected), "{text}");
    }
    let output = run("validate", &path, false);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Valid score:")
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), EXAMPLE);
}

#[test]
fn json_results_have_stable_fields_and_no_diagnostics() {
    let sandbox = Sandbox::new();
    let path = sandbox.file("example.json", EXAMPLE);
    let first = run("inspect", &path, true);
    assert_eq!(first.stdout, run("inspect", &path, true).stdout);
    let inspected = machine(&first, 0);
    assert_eq!(inspected["command"], "inspect");
    assert_eq!(inspected["path"], path.to_str().unwrap());
    assert_eq!(inspected["result"]["format_version"], "1.0");
    assert_eq!(inspected["result"]["timeline"]["tempo"], 120.0);
    assert_eq!(inspected["result"]["timeline"]["ppq"], 960);
    assert_eq!(
        inspected["result"]["timeline"]["time_signature"],
        serde_json::json!({"numerator": 4, "denominator": 4})
    );
    assert_eq!(inspected["result"]["track_count"], 2);
    assert_eq!(inspected["result"]["note_count"], 6);
    assert_eq!(inspected["result"]["tracks"][0]["name"], "Melody");
    assert_eq!(inspected["result"]["tracks"][1]["note_count"], 3);
    let validated = machine(&run("validate", &path, true), 0);
    assert_eq!(validated["command"], "validate");
    assert_eq!(validated["result"], serde_json::json!({"valid": true}));
    assert_eq!(std::fs::read_to_string(path).unwrap(), EXAMPLE);
}

#[test]
fn missing_and_unreadable_paths_are_distinct_io_errors() {
    let sandbox = Sandbox::new();
    for (path, kind) in [
        (sandbox.0.join("missing.json"), "file_not_found"),
        (sandbox.0.clone(), "file_read"),
    ] {
        // A directory is reliably unreadable as a score, including privileged CI.
        for command in ["inspect", "validate"] {
            let result = machine(&run(command, &path, true), 3);
            assert_eq!(result["error"]["kind"], kind);
            assert_eq!(result["path"], path.to_str().unwrap());
            let human = run(command, &path, false);
            assert_eq!(human.status.code(), Some(3));
            assert!(human.stdout.is_empty());
            assert!(String::from_utf8(human.stderr).unwrap().contains(kind));
        }
    }
}

#[cfg(windows)]
#[test]
fn windows_file_sharing_denial_is_a_read_error() {
    use std::os::windows::fs::OpenOptionsExt;
    let sandbox = Sandbox::new();
    let path = sandbox.file("locked.json", EXAMPLE);
    let _lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    assert_eq!(
        machine(&run("validate", &path, true), 3)["error"]["kind"],
        "file_read"
    );
}

#[test]
fn syntax_errors_include_real_locations() {
    let sandbox = Sandbox::new();
    for raw in [
        "".to_owned(),
        "{\n  \"format_version\": !\n}".into(),
        format!("{EXAMPLE} trailing"),
        EXAMPLE.replacen("\"tempo\": 120", "\"tempo\": 1e", 1),
    ] {
        let path = sandbox.file("invalid.json", raw);
        for command in ["inspect", "validate"] {
            let result = machine(&run(command, &path, true), 1);
            assert_eq!(result["error"]["kind"], "json_syntax");
            assert!(result["error"]["line"].as_u64().unwrap() >= 1);
            assert!(result["error"]["issues"].as_array().unwrap().is_empty());
        }
    }
}

#[test]
fn structural_errors_use_the_production_reader() {
    let sandbox = Sandbox::new();
    for (raw, context) in [
        (
            EXAMPLE.replacen("\"format_version\": \"1.0\",", "", 1),
            "format_version",
        ),
        (
            EXAMPLE.replacen("\"ppq\": 960", "\"ppq\": \"960\"", 1),
            "invalid type",
        ),
        (
            EXAMPLE.replacen("\"ppq\": 960", "\"ppq\": 960.0", 1),
            "invalid type",
        ),
        (
            EXAMPLE.replacen("\"tempo\": 120", "\"tempo\": 1e400", 1),
            "timeline.tempo",
        ),
        (
            EXAMPLE.replacen("\"tempo\": 120", "\"tempo\": \"120\"", 1),
            "timeline.tempo",
        ),
        (
            EXAMPLE.replacen("\"tempo\": 120", "\"tempo\": 120, \"tempo\": 120", 1),
            "duplicate field",
        ),
        (
            EXAMPLE.replacen("\"ppq\": 960", "\"ppq\": 960, \"extra\": true", 1),
            "unknown field",
        ),
        ("[]".into(), "Score"),
    ] {
        let path = sandbox.file("structure.json", raw);
        let result = machine(&run("validate", &path, true), 1);
        assert_eq!(result["error"]["kind"], "score_structure");
        assert!(
            result["error"]["message"]
                .as_str()
                .unwrap()
                .contains(context),
            "{result}"
        );
    }
}

#[test]
fn semantic_errors_preserve_core_context_without_inventing_locations() {
    let sandbox = Sandbox::new();
    for (raw, context) in [
        (
            EXAMPLE.replacen("\"tempo\": 120", "\"tempo\": 0", 1),
            "timeline.tempo",
        ),
        (
            EXAMPLE.replacen("\"tempo\": 120", "\"tempo\": -1", 1),
            "timeline.tempo",
        ),
        (
            EXAMPLE.replacen("\"tempo\": 120", "\"tempo\": 1e-400", 1),
            "timeline.tempo",
        ),
        (
            EXAMPLE.replacen("\"duration_ticks\": 480", "\"duration_ticks\": 0", 1),
            "note-001",
        ),
        (
            EXAMPLE.replacen("\"pitch\": \"C4\"", "\"pitch\": \"C#4\"", 1),
            "not playable",
        ),
        (
            EXAMPLE.replacen(
                "\"format_version\": \"1.0\"",
                "\"format_version\": \"2.0\"",
                1,
            ),
            "format_version",
        ),
    ] {
        let path = sandbox.file("semantics.json", raw);
        for command in ["inspect", "validate"] {
            let result = machine(&run(command, &path, true), 1);
            assert_eq!(result["error"]["kind"], "score_validation");
            assert!(result["error"]["line"].is_null());
            assert!(result["error"]["column"].is_null());
            assert!(
                result["error"]["issues"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|issue| issue.as_str().unwrap().contains(context))
            );
            let human = run(command, &path, false);
            assert_eq!(human.status.code(), Some(1));
            assert!(human.stdout.is_empty());
            assert!(String::from_utf8(human.stderr).unwrap().contains(context));
        }
    }
}

#[test]
fn invalid_usage_is_never_a_file_error() {
    for args in [
        vec![],
        vec!["unknown"],
        vec!["inspect"],
        vec!["validate"],
        vec!["inspect", "one", "two"],
        vec!["inspect", "--bogus"],
        vec!["--version", "inspect"],
        vec!["--help", "--version"],
    ] {
        let mut args = args.iter().map(OsStr::new).collect::<Vec<_>>();
        let human = cli(&args);
        assert_eq!(human.status.code(), Some(2));
        assert!(human.stdout.is_empty());
        assert!(!human.stderr.is_empty());
        args.push(OsStr::new("--json"));
        assert_eq!(machine(&cli(&args), 2)["error"]["kind"], "usage");
    }
}

#[test]
fn options_work_before_or_after_the_path_and_double_dash_quotes_filenames() {
    let sandbox = Sandbox::new();
    let path = sandbox.file("file.json", EXAMPLE);
    for args in [
        vec![
            OsStr::new("--json"),
            OsStr::new("inspect"),
            path.as_os_str(),
        ],
        vec![
            OsStr::new("inspect"),
            OsStr::new("--json"),
            path.as_os_str(),
        ],
    ] {
        machine(&cli(&args), 0);
    }
    sandbox.file("--json", EXAMPLE);
    let output = Command::new(env!("CARGO_BIN_EXE_skysheet"))
        .current_dir(&sandbox.0)
        .args(["validate", "--", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .starts_with("Valid score:")
    );
}

#[derive(Deserialize)]
struct TempoFixture {
    cases: Vec<TempoCase>,
}
#[derive(Deserialize)]
struct TempoCase {
    id: String,
    token: String,
    valid: bool,
    decoded_bits: Option<String>,
}
#[derive(Deserialize)]
struct TempoOutput {
    result: TempoSummary,
}
#[derive(Deserialize)]
struct TempoSummary {
    timeline: skysheet_score::Timeline,
}

#[test]
fn n2_frozen_cases_survive_the_actual_cli_including_long_midpoints() {
    let sandbox = Sandbox::new();
    // Reuse all 84 already-certified expectations, including 30 long cases.
    // No Oracle, conversion algorithm or new numeric expectations live here.
    let fixture: TempoFixture = serde_json::from_str(include_str!(
        "../../skysheet-score/tests/fixtures/n2-tempo.json"
    ))
    .unwrap();
    assert_eq!(fixture.cases.len(), 84);
    for case in fixture.cases {
        let raw = EXAMPLE.replacen("\"tempo\": 120", &format!("\"tempo\": {}", case.token), 1);
        let path = sandbox.file("tempo.json", &raw);
        let code = if case.valid { 0 } else { 1 };
        machine(&run("validate", &path, true), code);
        let inspected = run("inspect", &path, true);
        machine(&inspected, code);
        if case.valid {
            let summary: TempoOutput = serde_json::from_slice(&inspected.stdout).unwrap();
            let bits = u64::from_str_radix(case.decoded_bits.as_ref().unwrap(), 16).unwrap();
            assert_eq!(summary.result.timeline.tempo.to_bits(), bits, "{}", case.id);
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
    }
}
