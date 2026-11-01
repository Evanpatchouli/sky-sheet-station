# CLI Specification

**Status:** Phase 1.5 implements `--help`, `--version`, `inspect`, `validate`
and `--json`. Phase 2 adds `import-midi`, `export-midi`, and `inspect-midi`;
see [MIDI commands and limits](midi.md). See [CLI Alpha](cli-alpha.md) for the
original executable contract. Generic `convert`/`export` below remain proposals.

Phase 3 adds `inspect-music <FILE.mid> [--json]` and `analyze <FILE.mid> [--json]`;
`analyze-midi` remains an alias for analysis. JSON uses the same envelope with
version 2 of the internal `result.report`, including
source facts, statistics, uncertain candidates and diagnostics. It accepts neither
output paths nor reference-tempo overrides. See [music analysis](music-analysis.md).

Phase 4-A adds `arrange <FILE.mid> -o <SCORE.json> [--auto | --transpose <SEMITONES>]
[--report <REPORT.json>] [--json]`. Automatic global transposition is the default;
fixed offsets are never replaced. Every loss is reported separately from Score.
Phase 4-B/C now defaults to context-aware `melody-first` with octave adaptation.
Use `--mode melody-first|simple-accompaniment|polyphonic`, `--no-octave-shifts`,
`--melody-priority 0..5` (default 3), `--bass-priority 0..5` (2),
`--accompaniment-priority 0..5` (1), and optional `--max-polyphony 1..128`.
No polyphony ceiling applies unless supplied. `--auto` and `--transpose` are
mutually exclusive; fixed -127..127 remains exact, even when it loses every note.
Report v2 retains v1 fields and adds final role counts, octave/interval traces,
onset/chord diagnostics and playability checks. Candidate counts are pre-DP
retention upper bounds; use final report counts for actual output. Errors retain
exit codes 1/2/3/4; `--json` emits one response. Existing destinations are refused.

```text
skysheet arrange audios/ode-to-joy.mid -o melody.json --mode melody-first --auto --json
skysheet arrange audios/ode-to-joy.mid -o simple.json --mode simple-accompaniment
skysheet arrange audios/ode-to-joy.mid -o poly.json --mode polyphonic --max-polyphony 4
skysheet arrange audios/ode-to-joy.mid -o fixed.json --mode melody-first --transpose 12
skysheet validate melody.json --json
```

See [implemented rules, parameter meanings and evidence](phase-4bc-arrangement.md).

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
