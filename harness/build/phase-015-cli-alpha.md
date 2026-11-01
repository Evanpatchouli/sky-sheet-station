# Phase 1.5 CLI Alpha verification — 2026-11-01

## Scope and baseline

Delivered `skysheet 0.1.0-alpha.1`: help/version, inspect, validate, JSON success
and failure results, and exit codes 0–4. Only existing score-file workflows are
implemented. N2 research is complete and was not reopened. No MIDI, analysis,
arrangement, GUI or MCP work was started; Phase 1 is still not fully frozen.

Preflight found a clean `main` at
`1182830ff599a0d2a55b488e8d974b402c21909e`, matching the locally stored
`origin/main` reference. Its history includes `5cee8ef` (N2 RawValue correction)
and `1182830` (Linux acceptance documentation). No fetch/push or new remote CI
run was performed during the initial local implementation described here.

The new CLI uses `std::env::args_os`, existing workspace Serde/serde_json, and
the local Score crate. Original file bytes go directly to
`serde_json::from_slice::<Score>`, then `Score::validate()`. The generic JSON
value used for the output summary is never an input to Score. No core model,
production parser feature, Schema, example or existing test expectation changed.
The root lockfile adds only the local CLI package; all registry packages retain
their versions/checksums. The isolated numeric lockfile and historical evidence
are unchanged.

The old CLI document contained a proposed, unimplemented exit-code table. The
owner's Phase 1.5 table now defines the implemented contract. Root README's stale
claims of active N2 failures were corrected to agree with the accepted baseline.

## Windows executable

| Property | Observed value |
|---|---|
| Package / executable | `skysheet-cli` / `skysheet.exe` |
| Version | `0.1.0-alpha.1` |
| Target | `x86_64-pc-windows-msvc` |
| Profile | Release, optimized |
| Rust | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Cargo | `cargo 1.99.1 (5f94df478 2026-08-27)` |
| PE machine | `0x8664` (AMD64) |
| Size | 404,992 bytes |
| Path | `E:\sky-sheet-station\target\x86_64-pc-windows-msvc\release\skysheet.exe` |
| SHA-256 | `53d06d967e54f7ca18c3c60fd7ce20a4c5c507373123c77d3ca021180ff7e1bb` |
| Source identity | Baseline commit above plus uncommitted Phase 1.5 changes |

Build command, run successfully from the root:

```text
cargo build -p skysheet-cli --release --locked --target x86_64-pc-windows-msvc
```

This is a local Windows binary, not a Linux substitute or published release.
The compiler's installed Cargo bin directory was added to process PATH only.
No toolchain installation or machine-wide environment change was needed.
Build version/identity is also saved in `target/cli-alpha-20261101/build-info.json`.
The hash identifies this actual build; rebuilding with another environment is
not claimed to produce an identical hash.

## Executable smoke checks

All commands below used
`target/x86_64-pc-windows-msvc/release/skysheet.exe` from the repository root.
The table gives its arguments and observed exit status. Full stdout/stderr for
each invocation is saved in `target/cli-alpha-20261101/smoke-results.json` and
the corresponding `.stdout.txt`/`.stderr.txt` files. All captured stderr files
are empty (error checks below explicitly requested JSON).

| Arguments | Exit | Observed result |
|---|---:|---|
| `--help` | 0 | Help with inspect/validate syntax, options and exit codes |
| `--version` | 0 | `skysheet 0.1.0-alpha.1` |
| `inspect docs/examples/sky-15-example.json` | 0 | Summary reproduced below |
| `validate docs/examples/sky-15-example.json` | 0 | `Valid score: "docs/examples/sky-15-example.json"` |
| `inspect docs/examples/sky-15-example.json --json` | 0 | Parseable JSON; 2 tracks, 6 notes, original timeline and profile |
| `validate docs/examples/sky-15-example.json --json` | 0 | Parseable JSON with `result.valid: true` |
| `validate target/cli-alpha-20261101/invalid.json --json` | 1 | `json_syntax`; input is `{` |
| `validate target/cli-alpha-20261101/structure.json --json` | 1 | `score_structure`; input is `{}` |
| `validate target/cli-alpha-20261101/semantic.json --json` | 1 | `score_validation`; example with tempo 0 |
| `unknown --json` | 2 | `usage` |
| `inspect target/cli-alpha-20261101/missing.json --json` | 3 | `file_not_found` |
| `validate docs --json` | 3 | `file_read`; directory cannot be read as a score |

Actual inspect stdout:

```text
File: "docs/examples/sky-15-example.json"
Score: "Example song" ("score-001")
Format version: 1.0
Instrument: "Sky 15-key instrument" ("sky-15"), 15 keys
Tempo: 120 quarter notes/min
PPQ: 960
Time signature: 4/4
Tracks: 2
Notes: 6
  "Melody" ("track-001"): 3 notes
  "Harmony" ("track-002"): 3 notes
Validation: OK
```

Actual validate JSON stdout:

```json
{
  "output_version": 1,
  "ok": true,
  "command": "validate",
  "path": "docs/examples/sky-15-example.json",
  "exit_code": 0,
  "result": {
    "valid": true
  },
  "error": null
}
```

Actual missing-field JSON stdout:

```json
{
  "output_version": 1,
  "ok": false,
  "command": "validate",
  "path": "target/cli-alpha-20261101/structure.json",
  "exit_code": 1,
  "result": null,
  "error": {
    "kind": "score_structure",
    "message": "missing field `format_version` at line 1 column 2",
    "issues": [],
    "line": 1,
    "column": 2
  }
}
```

Exit 4 is reserved for internal execution/encoding errors; no artificial
production failure switch was introduced to trigger it. Ordinary failures were
observed as 1, 2 and 3, not conflated with internal errors.

## Tests actually run

| Command | Observed outcome |
|---|---|
| `cargo test -p skysheet-cli --offline` | 2 unit + 11 real-process integration tests passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --locked` | 64 tests passed: 13 CLI + 51 existing Score tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo test -p skysheet-cli --release --locked --target x86_64-pc-windows-msvc` | 2 unit + 11 integration tests passed against Release |
| `cargo fmt --manifest-path tools/numeric-validation/Cargo.toml --all -- --check` | Passed |
| `cargo test --manifest-path tools/numeric-validation/Cargo.toml --workspace --locked` | 20 tests passed, including supplementary N2 suite |
| `cargo clippy --manifest-path tools/numeric-validation/Cargo.toml --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked -- --report target/cli-alpha-numeric-20261101.json` | 267 fixed cases + 10,000 historical samples; zero failures |
| `git diff --check` | Passed |

CLI integration tests cover help/version; both text commands; stable JSON and
clean streams; missing/unreadable paths; Windows exclusive file-sharing denial;
empty/malformed JSON and trailing garbage; missing fields and wrong types;
unknown/repeated fields; invalid integer spelling; invalid tempo; semantic note,
pitch and version failures; unsupported/missing/extra arguments; Unicode/spaced
paths; option ordering and `--`; and input-byte preservation.

The CLI N2 test launches inspect and validate on all 84 existing frozen cases,
including 30 long-number cases. It checks acceptance and output tempo bits
against the existing fixture, without copying the Oracle or changing expectations.
Both debug and Release runs pass. The isolated suite independently passes all
3,016 research cases and 84 focused cases. Its additional reports are at:

```text
target/n2-regressions/1793494629218453500-30788/research-3016.json
target/n2-regressions/1793494629218453500-30788/focused-84.json
target/n2-regressions/1793494629218453500-30788/historical-max-repro.json
```

An initial Clippy run returned success but printed a Windows access-denied note
while finalizing an incremental cache. Both Clippy commands were rerun with
process-local `CARGO_INCREMENTAL=0` and passed without that note. No source or
validation rule was changed to accommodate the cache issue.

The initial local verification did not include a new hosted CI execution.
Existing workspace CI includes CLI tests when these changes are committed and
pushed. Windows-specific file-sharing coverage is guarded by `cfg(windows)`.

## Initial local delivery snapshot and remaining work

At the initial local delivery, branch and HEAD were `main` and
`1182830ff599a0d2a55b488e8d974b402c21909e`. At that point no changes were staged,
no commit had been created, and nothing had been pushed, merged, published or
deployed. The file lists below describe that historical pre-commit snapshot.

Modified tracked files:

```text
Cargo.toml
Cargo.lock
PLANS.md
README.md
docs/README.md
docs/cli.md
```

New, untracked files:

```text
crates/skysheet-cli/Cargo.toml
crates/skysheet-cli/src/args.rs
crates/skysheet-cli/src/main.rs
crates/skysheet-cli/tests/cli.rs
docs/cli-alpha.md
harness/build/phase-015-cli-alpha.md
```

Executables, smoke inputs/results and generated numeric reports remain under
ignored `target/`; historical tracked evidence is intact. No user changes were
present at preflight or overwritten.

No known blocker remains for the requested Alpha workflows. Remaining limitations:
whole-file memory use, English diagnostics, incomplete nested parser field paths,
and a local build without published-release automation. Phase 1 final contract
and fixture audits remain open; MIDI/D6 implementation tests are later Phase 2
work. These do not prevent inspection/validation of existing scores. See
[CLI Alpha](../../docs/cli-alpha.md) for the full output contract and limitations.
