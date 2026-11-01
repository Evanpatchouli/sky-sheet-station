use serde::Deserialize;
use serde_json::Value;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const EXAMPLE: &str = include_str!("../../../docs/examples/sky-15-example.json");
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
