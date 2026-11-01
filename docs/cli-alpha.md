# CLI Alpha (Phase 1.5)

`skysheet 0.1.0-alpha.1` reads, inspects and validates existing SkySheet score
JSON. The Windows Release target verified for this milestone is
`x86_64-pc-windows-msvc`. This is a local build, not a published release.

## Build and run

Use an edition-2024-capable Rust toolchain and the MSVC native linker/build
tools. The observed toolchain is Rust/Cargo 1.99.1; this is not a new MSRV policy.
From the repository root in PowerShell:

```powershell
cargo build -p skysheet-cli --release --locked --target x86_64-pc-windows-msvc
& .\target\x86_64-pc-windows-msvc\release\skysheet.exe --help
& .\target\x86_64-pc-windows-msvc\release\skysheet.exe --version
& .\target\x86_64-pc-windows-msvc\release\skysheet.exe inspect .\docs\examples\sky-15-example.json
& .\target\x86_64-pc-windows-msvc\release\skysheet.exe validate .\docs\examples\sky-15-example.json
```

If Cargo is installed but absent from PATH, add its existing directory for this
session: `$env:PATH = (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + $env:PATH`.
No GUI or extra runtime is used. Initial builds need registry access unless the
locked dependencies are already cached. For development, use
`cargo run -p skysheet-cli -- inspect docs/examples/sky-15-example.json`.

You can copy the resulting executable to a directory on PATH and invoke
`skysheet` directly. Build output remains ignored by Git. See the
[verification record](../harness/build/phase-015-cli-alpha.md) for the actual
artifact hash, commands, outputs and checks.

## Commands

```text
skysheet --help
skysheet --version
skysheet inspect example.json
skysheet validate example.json
skysheet inspect example.json --json
skysheet validate example.json --json
```

`inspect` displays the filename, score identity, format version, instrument,
tempo (quarter notes per minute), PPQ, nominal time signature, total tracks and
notes, and each track's name, ID and note count. Both commands require a valid
score; inspection does not produce a partial summary for invalid input.

Both read the original file bytes directly into the production `Score`
deserializer, then call `Score::validate()`. There is no intermediate JSON
`Value`, alternate validator, tempo converter or production Schema engine.
Unknown fields, repeated decoded member names, integer spelling, pitch parsing
and the N2 RawValue tempo path retain the Score crate's behavior. Input files
are read-only and never rewritten. The example has 2 tracks, 6 notes, tempo 120,
PPQ 960 and meter 4/4.

`--json` is accepted before the command, between the command and path, or after
the path. `--` ends option parsing, e.g. `skysheet inspect -- -score.json`.
Quote paths containing spaces. `-h` and `-V` alias help and version;
`inspect --help` and `validate --help` show help. Help/version also support JSON.
Unknown/repeated options, extra files and missing arguments are usage errors.
There is no implicit stdin mode: `-` is only a filename after `--`.

## JSON result contract

JSON mode emits exactly one UTF-8 JSON document plus a newline to stdout, for
both success and ordinary failure. Stderr is empty in those cases. There are no
ANSI decorations, timestamps, progress messages or mixed prose. Field names
and types are stable under `output_version: 1`; this version is independent of
the score format version. Output is a command result, not a saved score file.

Every response has these fields, including explicit nulls:

| Field | Meaning |
|---|---|
| `output_version` | Integer `1`, CLI result contract version. |
| `ok` | Boolean success flag. |
| `command` | `inspect`, `validate`, `help` or `version`; null on argument-parse failure. |
| `path` | Input path as supplied, or null for help/version/argument-parse failure. |
| `exit_code` | Integer process exit code for the result. |
| `result` | Command result on success, null on failure. |
| `error` | Diagnostic on failure, null on success. |

Inspect's result contains `format_version`, `metadata`, `instrument`, `timeline`
(using the existing core serialization), `track_count`, `note_count`, and
`tracks`. Each track summary contains `id`, `name`, `role`, `note_count`, retaining
input order. It omits note events and user-authored root text. Tempo is the
decoded binary64 value, not the original decimal spelling. Consumers needing
exact bits must use a correctly rounded binary64 JSON reader.

Validate success:

```json
{
  "output_version": 1,
  "ok": true,
  "command": "validate",
  "path": "example.json",
  "exit_code": 0,
  "result": { "valid": true },
  "error": null
}
```

A semantic error (other error kinds use the same shape):

```json
{
  "output_version": 1,
  "ok": false,
  "command": "validate",
  "path": "invalid.json",
  "exit_code": 1,
  "result": null,
  "error": {
    "kind": "score_validation",
    "message": "Score semantic validation failed",
    "issues": ["timeline.tempo must be finite and greater than zero"],
    "line": null,
    "column": null
  }
}
```

Help results contain `help` (string). Version results contain `name` (`skysheet`)
and `version` (package version string).

## Exit codes and diagnostics

| Code | Meaning | Error kinds |
|---:|---|---|
| 0 | Success | No error |
| 1 | Score parsing or validation failed | `json_syntax`, `score_structure`, `score_validation` |
| 2 | CLI usage error | `usage` |
| 3 | Filesystem or output I/O error | `file_not_found`, `file_read`; `output_io` on stderr if stdout fails |
| 4 | Internal execution/encoding error | `internal` |

These supersede the old unimplemented exit-code proposal in `cli.md`.
In ordinary text mode, successful summaries go to stdout; failures go to stderr
and leave stdout empty. File errors include the path and OS error message.
The production parser's Syntax/EOF categories map to `json_syntax`; its Data
category maps to `score_structure` (including tempo overflow rejected by the
field deserializer). Zero, negative or underflow-to-zero tempo is instead a
`score_validation` failure. Semantic errors preserve every core validation issue.

`error.message` is a string and `issues` is always an array of strings (empty
except for semantic validation). Wording may change with the OS or parser:
scripts should branch on `kind`/`exit_code`, not parse the prose. Field/ID context
is passed through when the core provides it. `line` and `column` are positive
parser-reported positions or null when unavailable. They are parser detection
positions, not guaranteed field-start locations. Semantic validation has no
source map and reports null positions. A file with multiple faults reports the
first deserialization error encountered, or all semantic issues after a successful
parse. No second pass reclassifies that first error.

If stdout itself fails (for example a closed pipe), a complete JSON result cannot
be guaranteed: exit 3 and best-effort stderr report the transport failure. An
unexpected response encoding failure exits 4 with best-effort stderr. Crashes or
external termination are outside the structured-result guarantee.

## Tests and limitations

```text
cargo test -p skysheet-cli --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Integration tests launch the actual `skysheet` binary, including all 84 existing
N2 tempo fixtures with frozen expectations (30 long-number cases). The existing
[isolated numeric checks](numeric-verification.md#commands-and-ci) remain required
and separate; their dependencies/features do not join the production workspace.

The entire input file is read into memory; no large-file streaming/performance
guarantee is made. Diagnostics currently use English, and complete nested field
paths are unavailable for some parser failures. Non-Unicode OS paths are opened
using native path types, but JSON path display replaces unrepresentable text.
JSON results contain no original score text, note events or token spellings.

Phase 1's final contract/fixture audit remains open and is not a prerequisite for
these existing Score workflows. MIDI import/export/playback/quantization, audio
analysis, arrangement, GUI, MCP, networking and database features are absent.
Shell completion, richer source locations, localization and release automation
are deferred. No further phase is started by this Alpha.
