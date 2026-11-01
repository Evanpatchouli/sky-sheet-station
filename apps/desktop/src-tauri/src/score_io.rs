use serde::{Deserialize, Serialize};
use skysheet_score::Score;
use std::{
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_BYTES: u64 = 32 * 1024 * 1024;
static NEXT_SNAPSHOT: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize)]
pub struct LoadedScore {
    pub score: Score,
    // Computed by the core Pitch API; the frontend never parses pitch spellings.
    pub midi_keys: Vec<u8>,
    pub source: String,
}

pub fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| format!("无法读取 {}：{e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("此桌面预览版本最多加载 32 MiB；这不是 Score 格式限制。".into());
    }
    Ok(bytes)
}

pub struct Snapshot {
    pub path: PathBuf,
}

impl Snapshot {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "skysheet-{}-{stamp}-{}",
            std::process::id(),
            NEXT_SNAPSHOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).map_err(|e| format!("无法创建验证快照：{e}"))?;
        let snapshot = Self {
            path: dir.join("score.json"),
        };
        fs::write(&snapshot.path, bytes).map_err(|e| format!("无法写入验证快照：{e}"))?;
        Ok(snapshot)
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        if let Some(dir) = self.path.parent() {
            let _ = fs::remove_dir(dir);
        }
    }
}

pub fn validation_args(path: &Path) -> Vec<OsString> {
    vec![
        "validate".into(),
        "--json".into(),
        "--".into(),
        path.as_os_str().into(),
    ]
}

#[derive(Deserialize)]
struct CliResponse {
    output_version: u8,
    command: Option<String>,
    ok: bool,
    exit_code: i32,
    result: Option<ValidationResult>,
    error: Option<CliError>,
}
#[derive(Deserialize)]
struct ValidationResult {
    valid: bool,
}
#[derive(Deserialize)]
struct CliError {
    kind: String,
    message: String,
    issues: Vec<String>,
    line: Option<usize>,
    column: Option<usize>,
}

pub fn check_validation(code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> Result<(), String> {
    let response: CliResponse = serde_json::from_slice(stdout).map_err(|e| {
        format!(
            "CLI 返回无效响应（退出码 {code:?}）：{e} {}",
            String::from_utf8_lossy(stderr)
        )
    })?;
    if response.output_version != 1
        || response.command.as_deref() != Some("validate")
        || code != Some(response.exit_code)
    {
        return Err("CLI 响应版本、命令或退出码不匹配，请重新构建 sidecar。".into());
    }
    if response.ok
        && code == Some(0)
        && response.error.is_none()
        && response.result.is_some_and(|r| r.valid)
    {
        return Ok(());
    }
    if let Some(e) = response.error {
        return Err(format!(
            "{}：{}{}{}",
            e.kind,
            e.message,
            e.line
                .map(|line| format!("（行 {line}，列 {}）", e.column.unwrap_or(0)))
                .unwrap_or_default(),
            if e.issues.is_empty() {
                String::new()
            } else {
                format!("\n{}", e.issues.join("\n"))
            }
        ));
    }
    Err(format!("CLI 验证失败（退出码 {code:?}）。"))
}

pub fn decode_validated(bytes: &[u8], source: String) -> Result<LoadedScore, String> {
    // Use the existing raw-byte Score deserializer, including its N2 adapter.
    // Never parse input through Value or implement a GUI tempo/score validator.
    let score: Score = serde_json::from_slice(bytes).map_err(|e| format!("Score 读取失败：{e}"))?;
    let midi_keys = score
        .instrument
        .keys
        .iter()
        .map(|pitch| pitch.midi_number())
        .collect();
    Ok(LoadedScore {
        score,
        midi_keys,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXAMPLE: &[u8] = include_bytes!("../../../../docs/examples/sky-15-example.json");

    fn real_validate(bytes: &[u8]) -> Result<LoadedScore, String> {
        let snapshot = Snapshot::new(bytes)?;
        let cli = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/skysheet-x86_64-pc-windows-msvc.exe");
        let output = std::process::Command::new(cli)
            .args(validation_args(&snapshot.path))
            .output()
            .unwrap();
        check_validation(output.status.code(), &output.stdout, &output.stderr)?;
        decode_validated(bytes, "test".into())
    }

    #[test]
    fn real_cli_loads_example_and_derives_keys() {
        let loaded = real_validate(EXAMPLE).unwrap();
        assert_eq!(loaded.midi_keys.len(), 15);
        assert_eq!(loaded.midi_keys[0], 60);
        assert_eq!(
            loaded
                .score
                .tracks
                .iter()
                .map(|t| t.notes.len())
                .sum::<usize>(),
            6
        );
    }

    #[test]
    fn n2_frozen_cases_survive_cli_and_desktop_projection() {
        #[derive(Deserialize)]
        struct Fixture {
            cases: Vec<Case>,
        }
        #[derive(Deserialize)]
        struct Case {
            id: String,
            token: String,
            valid: bool,
            decoded_bits: Option<String>,
        }
        let fixture: Fixture = serde_json::from_str(include_str!(
            "../../../../crates/skysheet-score/tests/fixtures/n2-tempo.json"
        ))
        .unwrap();
        assert_eq!(fixture.cases.len(), 84);
        for case in fixture.cases {
            let text = std::str::from_utf8(EXAMPLE).unwrap().replacen(
                "\"tempo\": 120",
                &format!("\"tempo\": {}", case.token),
                1,
            );
            let result = real_validate(text.as_bytes());
            assert_eq!(result.is_ok(), case.valid, "{}", case.id);
            if let Ok(loaded) = result {
                let expected =
                    u64::from_str_radix(case.decoded_bits.as_deref().unwrap(), 16).unwrap();
                assert_eq!(
                    loaded.score.timeline.tempo.to_bits(),
                    expected,
                    "{}",
                    case.id
                );
                let serialized = serde_json::to_string(&loaded.score).unwrap();
                let readback: Score = serde_json::from_str(&serialized).unwrap();
                assert_eq!(readback.timeline.tempo.to_bits(), expected, "{}", case.id);
            }
        }
    }

    #[test]
    fn real_cli_rejects_original_duplicate_members_and_invalid_notes() {
        let text = std::str::from_utf8(EXAMPLE).unwrap();
        for bad in [
            text.replacen("\"tempo\": 120", "\"tempo\": 120, \"tempo\": 120", 1),
            text.replacen("\"duration_ticks\": 480", "\"duration_ticks\": 0", 1),
            text.replacen("\"start_tick\": 0", "\"start_tick\": 0.0", 1),
        ] {
            assert!(real_validate(bad.as_bytes()).is_err());
        }
    }

    #[test]
    fn snapshot_handles_unicode_spaces_and_shell_metacharacters() {
        let snapshot = Snapshot::new(EXAMPLE).unwrap();
        let path = snapshot
            .path
            .parent()
            .unwrap()
            .join("乐谱 & 空格 ' ; $.json");
        fs::write(&path, EXAMPLE).unwrap();
        assert_eq!(read_file(&path).unwrap(), EXAMPLE);
        let cli = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/skysheet-x86_64-pc-windows-msvc.exe");
        let output = std::process::Command::new(cli)
            .args(validation_args(&path))
            .output()
            .unwrap();
        check_validation(output.status.code(), &output.stdout, &output.stderr).unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn snapshot_is_cleaned_up_and_transport_failure_is_explicit() {
        let snapshot = Snapshot::new(EXAMPLE).unwrap();
        let path = snapshot.path.clone();
        drop(snapshot);
        assert!(!path.exists());
        assert!(
            check_validation(None, b"", b"missing sidecar")
                .unwrap_err()
                .contains("CLI")
        );
    }
}
