# SkySheet Station Goals

## Product goal
Build a reliable, Rust-first, CLI-first tool that converts audio and MIDI into readable, editable scores adapted to Sky: Children of the Light's 15-key instruments.

The central product value is not merely detecting notes. It is producing a musically recognizable arrangement that respects the target instrument's limited pitch set and practical polyphony.

## Primary user outcomes
- Convert MP3 and other supported audio into a candidate note transcription.
- Import MIDI as a first-class input and avoid audio transcription when note events are already available.
- Analyze musical material sufficiently to identify candidate melody and accompaniment roles.
- Produce a 15-key playable arrangement in JSON by default.
- Export MIDI for interoperability.
- Inspect and validate score files from the command line.
- Allow other applications and scripts to consume score output without requiring the GUI.

## Product principles
1. CLI first: useful in a headless environment.
2. Rust first: core algorithms and interfaces are implemented in Rust where practical.
3. Music-aware adaptation: preserve melody, contour, rhythm, and useful harmony instead of blindly deleting unsupported notes.
4. Transparent loss: report octave shifts, pitch substitutions, removed notes, and other lossy transformations where feasible.
5. Human-readable formats: JSON is the default native score format; MIDI is supported for import/export.
6. Separation of concerns: original music/transcription data is distinct from instrument-specific arrangements.
7. Playback is external: SkySheet does not automate game controls or implement the downstream playback scheduler.
8. No assumed lyrics: text is user-authored metadata; lyric recognition is not a product requirement.
9. Incremental quality: validate the data contract and MIDI path before investing heavily in complex audio transcription.

## Initial scope
- Rust workspace and reusable core crates.
- Score model, JSON serialization, JSON Schema, semantic validation.
- MIDI import and export.
- Audio decoding and transcription research/prototyping.
- Music analysis and candidate melody extraction.
- Arrangement for the Sky 15-key instrument.
- CLI commands for conversion, inspection, validation, and export.
- Tauri GUI later, consuming the CLI.

## Explicit non-goals for the initial release
- Auto-playing or simulated key input.
- Playback scheduling as part of the core.
- MCP server.
- Cloud processing as a requirement.
- Automatic lyric transcription.
- Mandatory automatic phrase or bar segmentation.
- A universal DAW or full notation editor.
- Recording or measuring in-game sounding pitches to calibrate the Sky key mapping; the default C4-C6 mapping defines logical score pitches.

## Quality attributes
- Correctness and deterministic validation of score files.
- Clear diagnostics for unsupported inputs and lossy conversion.
- Testable crate boundaries and reproducible builds.
- Transcription quality measured against representative audio, not claimed from model choice alone.
- A stable, documented CLI and JSON contract for third-party clients.

## Success criteria
The first useful release should be able to:
- read a representative MIDI file;
- convert it to a valid Sky 15-key JSON arrangement;
- explain which notes were changed or removed;
- export a MIDI representation of the arrangement;
- validate the result independently of the GUI.

Audio-to-score quality is a later acceptance gate and must be evaluated on a curated test set.
