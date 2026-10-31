# Phase 00: Repository Foundation

**Status:** In progress

## Outcome

Make the existing repository reproducible for contributors and coding agents, and verify that the first score-format implementation matches the documented contract.

## Context

Read:
- AGENTS.md
- GOALS.md
- PLANS.md
- docs/architecture.md
- docs/music-data-format.md
- docs/schemas/sky-score-v1.schema.json
- docs/examples/sky-15-example.json
- crates/skysheet-score/

The repository currently has an initial Cargo workspace and score crate. Do not assume it compiles until the commands below have been run in a Rust-enabled environment.

## In scope

- Verify Rust toolchain and workspace build/test commands.
- Review the score crate against the v1 spec, JSON Schema, and fixture.
- Fix only clear implementation/spec mismatches that do not change the public format.
- Add or correct tests needed to establish the score crate baseline.
- Record observed results and update Phase 0 status.

## Out of scope

- Audio transcription.
- MIDI implementation.
- Arrangement algorithms.
- CLI command implementation.
- Tauri application.
- MCP.
- Changes to public score fields or pitch calibration without explicit approval.

## Acceptance criteria

- A clean checkout can run formatting, workspace tests, and lint.
- The documented JSON fixture deserializes, validates, and round-trips.
- Semantic validation rejects unplayable pitches, duplicate IDs, invalid durations, and invalid timing metadata.
- Documentation, schema, fixture, and Rust types agree on the v1 contract.
- Any remaining blockers are recorded with reproduction steps.

## Verification

Run from the repository root:

- cargo fmt --all -- --check
- cargo test --workspace
- cargo clippy --workspace --all-targets -- -D warnings

If dependency fetching or toolchain setup fails, record the exact failure. Do not claim the phase is complete.

## Completion record

- Changes:
- Tests run and results:
- Remaining issues:
- Review reference:
