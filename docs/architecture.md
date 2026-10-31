# Architecture

## Product definition

SkySheet Station is a music analysis and arrangement engine with a CLI-first interface. It analyzes audio or MIDI and produces score data adapted to a target instrument, initially Sky: Children of the Light's 15-key instruments.

The project is not an auto-playing bot. It does not press keys, schedule operating-system input, or implement game-specific automation.

## Architectural principles

All implementation and tooling must follow the technology-stack rules in
[AGENTS.md](../AGENTS.md): Rust first; Go/C++ only with owner approval;
Node.js/Bun only where necessary for auxiliary/frontend work; Python prohibited
in every project context. The independent numeric tool uses a separate Rust
workspace to prevent Schema dependencies from changing the tested parser features.

- **Rust first:** core processing and the CLI are implemented in Rust.
- **CLI first:** every core capability must be accessible without a GUI.
- **Reusable core:** business logic belongs in Rust libraries, not in CLI command handlers.
- **GUI as a client:** the Tauri application delegates conversion and inspection to the packaged CLI.
- **Interoperability:** JSON is the default native score format; MIDI is supported for import and export.
- **Explicit boundaries:** actual playback and key-input scheduling belong to downstream applications.
- **MCP deferred:** MCP is out of scope for the initial implementation.

## Component responsibilities

### Rust core libraries

Own audio decoding, transcription, MIDI parsing, music analysis, arrangement, instrument constraints, score validation, and serialization. The core must not depend on Tauri, a terminal UI, or an automation framework.

### CLI

Own command parsing, input/output paths, user-facing diagnostics, machine-readable output, and orchestration of core library operations. The CLI must not contain a second implementation of music algorithms. It calls the core libraries directly.

### Tauri desktop application

Provides graphical workflows and editing/visualization. Its Rust backend invokes the packaged SkySheet CLI as a sidecar and consumes its outputs. It must not reimplement transcription or arrangement logic.

The CLI remains independently usable when the desktop application is not installed.

### Downstream playback clients

A separate script or application may read generated JSON and perform playback. That client is responsible for scheduling, simultaneous key presses, key release behavior, repeated-note retriggering, and game input.

The score format describes musical note events and target-instrument compatibility; it does not prescribe a particular automation implementation.

### MCP

MCP may later expose capabilities to agents, but it is deliberately deferred. It should be another client of the core/CLI contract, not a prerequisite for the core.

## Suggested Rust workspace

The exact crate boundaries can evolve. A practical starting point is:

    crates/
      skysheet-core/       # shared domain types and orchestration
      skysheet-audio/      # decoding and audio preprocessing
      skysheet-midi/       # MIDI import/export
      skysheet-analysis/   # transcription and music analysis
      skysheet-arrange/    # target-instrument adaptation
      skysheet-score/      # score model, validation, JSON
      skysheet-cli/        # binary and commands
    apps/
      desktop/             # Tauri + frontend
    docs/

Avoid creating a crate for every small type prematurely. Split crates when dependency direction or independent testing justifies it.

## Data flow

    MP3/WAV/FLAC -> decode -> transcription --+
                                               +-> music representation
    MIDI --------> MIDI import ----------------+
      -> analysis -> arrangement for instrument -> validation
      -> JSON (default) / MIDI export

## Non-goals for the initial release

- Simulating keyboard or touch input.
- Playing songs inside the core.
- Automatically detecting or transcribing lyrics.
- Automatically dividing every song into phrases or sections.
- MCP server implementation.
- Cloud processing as a requirement.

## Design constraints

- Keep unrestricted source music separate from instrument-specific arrangements.
- The sky-15 C4-C6 pitch set is a logical score/key mapping (C4 = MIDI 60), independent of actual game sound. Game-recording collection and acoustic calibration are not required.
- Never claim a note is playable if its pitch is not in the target instrument's allowed pitch set.
- Preserve provenance or diagnostics where practical so users can understand lossy transformations.
- Treat JSON format changes as compatibility concerns and use an explicit format version.
