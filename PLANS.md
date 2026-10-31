# SkySheet Station Development Plan

This is a staged roadmap, not authorization to implement every phase in one task. Work only on the phase requested by the user. Update status and evidence as phases are completed.

## Phase 0: Repository and agent foundation
**Status:** In progress

- [x] Initial docs directory and architecture notes.
- [x] Initial score-format proposal, JSON Schema, and example.
- [x] Initial Rust workspace and score crate skeleton.
- [ ] Establish and verify a reproducible local Rust test workflow.
- [ ] Review the current score crate against the score specification and fix contract mismatches.

Exit criteria: a clean checkout can run formatting, tests, and lint; the example fixture deserializes, validates, and round-trips.

## Phase 1: Freeze the score contract
**Status:** Planned

- [ ] Resolve the in-game octave/pitch calibration.
- [ ] Finalize v1 JSON schema and semantic validation.
- [ ] Add positive and negative fixture coverage.
- [ ] Document serialization compatibility and format-version policy.
- [ ] Decide whether tempo-map flattening is acceptable for v1 MIDI import.

Exit criteria: score specification, schema, example fixtures, Rust types, and tests agree.

## Phase 2: MIDI interoperability
**Status:** Planned

- [ ] Select and document the Rust MIDI library.
- [ ] Import tracks, notes, note-on/off timing, meter, and tempo metadata.
- [ ] Define deterministic handling for tempo changes and unsupported MIDI constructs.
- [ ] Export a score to MIDI and test round-trip properties.
- [ ] Report information that cannot be preserved.

Exit criteria: representative MIDI fixtures import, validate, and export with documented behavior.

## Phase 3: Music analysis primitives
**Status:** Planned

- [ ] Define an unrestricted internal music/transcription representation.
- [ ] Add tempo, key/scale, and note-event analysis where measurable.
- [ ] Prototype candidate melody extraction and role estimation.
- [ ] Create a small licensed/owned evaluation corpus.
- [ ] Record accuracy, runtime, and memory results.

Exit criteria: analysis outputs include diagnostics and are evaluated against reference examples.

## Phase 4: Sky 15-key arrangement
**Status:** Planned

- [ ] Implement target-instrument pitch constraints.
- [ ] Compare global transposition candidates.
- [ ] Implement context-aware pitch mapping and octave placement.
- [ ] Implement melody-first and polyphonic arrangement modes.
- [ ] Implement note reduction and playability diagnostics.
- [ ] Evaluate recognizability and musical coherence with human review.

Exit criteria: generated arrangements are playable and retain recognizable musical structure on a reviewed test set.

## Phase 5: CLI product surface
**Status:** Planned

- [ ] Implement convert, inspect, validate, and export commands.
- [ ] Define stable exit codes and diagnostics.
- [ ] Keep JSON output clean for scripting.
- [ ] Add end-to-end CLI tests using fixture files.

Exit criteria: documented workflows work headlessly from a clean checkout.

## Phase 6: Tauri GUI
**Status:** Deferred until core CLI workflows are stable

- [ ] Design GUI workflows around the CLI capabilities.
- [ ] Package the CLI as a Tauri sidecar.
- [ ] Implement progress, cancellation, and error presentation.
- [ ] Avoid duplicating music algorithms in the GUI backend.

## Deferred: MCP
MCP is a possible future integration. Do not implement it as part of the initial phases unless the product owner explicitly reprioritizes it.
