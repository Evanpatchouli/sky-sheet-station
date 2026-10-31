# CLI Specification

**Status:** initial command proposal. Exact flags and subcommands may change before implementation.

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

Inspection reports format version, duration, tempo, track count, note count, pitch range, and instrument compatibility.

### Export MIDI

    skysheet export output.json --format midi --output output.mid

### Validate a score

    skysheet validate output.json

Validation checks the schema, timing values, unique identifiers, and target-instrument pitch constraints.

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

## Exit-code proposal

| Code | Meaning |
|---:|---|
| 0 | Success |
| 1 | General processing failure |
| 2 | Invalid arguments or options |
| 3 | Unsupported or invalid input |
| 4 | Score validation failure |
| 5 | Output I/O failure |

The exact codes should be centralized and documented in the CLI implementation.

## Tauri integration

The Tauri backend invokes the packaged CLI as a sidecar using argument arrays rather than shell-concatenated command strings. It captures stdout and stderr, reports progress where available, handles cancellation, and checks CLI compatibility.

The Tauri frontend does not independently implement transcription or arrangement algorithms.

## Deferred

- MCP server command.
- Daemon/server mode.
- Remote processing.
- Playback or key-input commands.
- Batch conversion, until the single-file contract is stable.
