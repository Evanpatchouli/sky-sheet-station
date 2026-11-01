# CLI Specification

**Status:** Phase 1.5 implements `--help`, `--version`, `inspect`, `validate`
and `--json`. See [CLI Alpha](cli-alpha.md) for the current executable contract,
build instructions and limitations. The other commands below remain proposals.

## Goals

- The CLI is the primary interface to SkySheet.
- All core workflows work without Tauri or a GUI.
- JSON is the default score output.
- MIDI can be imported and exported.
- Commands are scriptable and produce predictable exit codes.
- Playback and game input are outside the CLI's scope.

## Proposed commands

### Convert audio or MIDI into an instrument arrangement

    skysheet convert input.mp3 --instrument sky-15 --mode melody --output output.json
    skysheet convert input.mid --instrument sky-15 --mode polyphonic --output output.json

Input format is detected from the file or can be specified explicitly if needed. JSON is the default output format.

### Inspect a score

    skysheet inspect output.json
    skysheet inspect output.json --json

Alpha inspection reports format version, score/instrument identity, tempo, PPQ,
time signature, track count and per-track note counts after Score validation.
Duration and pitch-range summaries remain future proposals.

### Export MIDI

    skysheet export output.json --format midi --output output.mid

### Validate a score

    skysheet validate output.json

Alpha validation directly deserializes original JSON through the Score reader,
then calls `Score::validate()` for timing, IDs and instrument pitch constraints.
It does not run a separate production JSON Schema validator.

### Show available instruments

    skysheet instruments list
    skysheet instruments show sky-15

## Arrangement modes

Initial modes:

- **melody:** prioritize a coherent lead line and produce a monophonic arrangement where feasible.
- **polyphonic:** preserve selected simultaneous notes and useful harmonic context while respecting the target instrument's pitch set.

Further modes should be added only after quality can be evaluated against reference examples.

## Output and diagnostics

Recommended conventions:

- Normal human-readable summaries go to stdout.
- Machine-readable result JSON goes to stdout when explicitly requested.
- Diagnostics, warnings, and progress go to stderr.
- A command returns a non-zero exit code on failure.
- Do not mix progress text into a JSON document written to stdout.
- Output files should be written atomically where practical, so an interrupted conversion does not leave a seemingly valid partial score.

A future machine-readable progress mode may use JSON Lines on stderr or a separately selected event stream. It is not required for the first CLI prototype.

## Implemented Alpha exit codes

| Code | Meaning |
|---:|---|
| 0 | Success |
| 1 | Score parsing or semantic validation failure |
| 2 | Invalid arguments or options |
| 3 | Filesystem or I/O failure |
| 4 | Internal execution error |

These supersede the original unimplemented proposal. Stable diagnostic kinds and
JSON response fields are documented in [CLI Alpha](cli-alpha.md).

## Tauri integration

The Tauri backend invokes the packaged CLI as a sidecar using argument arrays rather than shell-concatenated command strings. It captures stdout and stderr, reports progress where available, handles cancellation, and checks CLI compatibility.

The Tauri frontend does not independently implement transcription or arrangement algorithms.

## Deferred

- MCP server command.
- Daemon/server mode.
- Remote processing.
- Playback or key-input commands.
- Batch conversion, until the single-file contract is stable.
