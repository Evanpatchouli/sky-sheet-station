# AGENTS.md

## Project identity
SkySheet Station is a Rust-first, CLI-first music analysis and arrangement tool. Its first target is converting audio or MIDI into scores adapted to Sky: Children of the Light's 15-key instruments.

Read GOALS.md, PLANS.md, and docs/README.md for product context. The score contract is in docs/music-data-format.md.

## Source of truth and decision discipline
- Treat checked-in documentation and current code as the source of truth. If they conflict, report the conflict before making a broad change.
- Distinguish confirmed decisions from proposals and open questions. Never silently turn assumptions into requirements.
- Do not invent product requirements, roadmap scope, performance claims, or supported platforms.
- Keep changes within the user's requested scope. Do not start the next roadmap phase automatically.
- If ambiguity changes architecture, public formats, dependencies, or compatibility, ask one focused question or present a short decision proposal.
- If implementation reveals a design flaw, explain the evidence and propose a doc change. Do not quietly redefine a public contract in code.

## Product boundaries
- Rust is the primary implementation language.
- CLI is the primary product interface and must work without a GUI.
- Business logic belongs in reusable Rust crates, not CLI command handlers.
- Tauri is a GUI client. Its backend invokes the packaged CLI; it must not duplicate transcription or arrangement logic.
- JSON is the default score format. MIDI import and export are supported.
- The core describes and transforms music; it does not press keys, schedule game input, or implement auto-playing.
- Playback scheduling and key-input behavior belong to downstream clients consuming score files.
- MCP is explicitly deferred.
- Do not add lyric recognition or automatic phrase/bar segmentation unless the product owner changes scope.

## Required workflow
1. Preflight: inspect repository status, relevant files, applicable nested AGENTS.md files, and existing tests before editing.
2. Understand: read the smallest relevant set of docs and code. Prefer targeted inspection over broad rewrites.
3. Plan: for non-trivial work, state intended files, approach, risks, and verification before implementation.
4. Implement: make the smallest coherent change. Avoid unrelated cleanup and speculative abstractions.
5. Verify: run formatting, relevant tests, and relevant validation. Never claim a command passed unless it was actually run and observed.
6. Review: inspect the final diff for scope, compatibility, generated files, secrets, and documentation drift.
7. Report: summarize changes, tests actually run, outcomes, and unresolved risks.

For multi-phase work, stop after the requested phase and wait for the user before starting the next one.

## Approval boundaries
Proceed with ordinary in-scope edits and local tests when requested. Ask before:
- adding, removing, or materially changing production dependencies;
- changing the public JSON score format or MIDI compatibility behavior;
- changing instrument pitch mappings or timing-field semantics;
- deleting user data or performing destructive migrations;
- adding network services, cloud dependencies, telemetry, or external credentials;
- releasing, publishing, deploying, or changing repository permissions/settings.

Never commit, push, merge, publish a release, or deploy unless the user explicitly asks. Never expose secrets or copy credentials into repository files.

## Rust conventions
- Follow the workspace edition and existing crate conventions.
- Prefer explicit domain types and validated constructors for values with invariants.
- Keep serialization decisions in the score crate and its tests; do not duplicate schemas in CLI code.
- Keep core crates independent of Tauri, terminal UI frameworks, and game automation.
- Prefer small, focused modules and public APIs. Avoid premature crate proliferation.
- Use descriptive errors that identify the invalid field/value and preserve actionable context.
- Do not use floating-point values for exported score tick positions or durations.
- Preserve the distinction between unrestricted source/transcription data and instrument-constrained exported arrangements.
- Do not add MIDI velocity to the exported Sky arrangement note model.
- Derive game key indices from the ordered instrument pitch list; do not duplicate key index and pitch in each note.

## Score-format invariants
Before changing score serialization or validation, read docs/music-data-format.md, the JSON Schema, and the example fixture.

Current v1 proposal:
- JSON format version is 1.0.
- PPQ is 960; note positions and durations are integer ticks.
- start_tick is absolute from score start; duration_ticks is positive.
- Pitch strings use SkySheet's explicit convention: C4 = MIDI note 60; sharps are canonical.
- Tempo decoding uses nearest binary64 with ties to even and preserves accepted bits through save/read (N2). Known reader differences and the independent gate are documented in docs/numeric-verification.md; do not weaken expectations or change parser features without approval.
- Instrument keys are ordered; array index + 1 derives the game key number.
- A playable note pitch must exist in the selected instrument's keys.
- trigger and sustain describe instrument behavior; they are not playback commands.
- sky-15 uses the default C4-C6 logical pitch mapping (C4 = MIDI 60); this does not assert actual in-game sounding pitches. Game-audio recording or acoustic calibration is not required, including for Phase 1 completion.
- The exported score has no velocity field.
- Notes are independent events; simultaneous notes form chords by sharing a start tick.
- No automatic phrase or lyric segmentation is required. Root-level text is optional user-authored text.

If these are intentionally changed, update the specification, JSON Schema, example fixture, Rust model, and tests together.

## Commands and verification
Run from the repository root unless noted:
- Format check: cargo fmt --all -- --check
- Workspace tests: cargo test --workspace
- Lint: cargo clippy --workspace --all-targets -- -D warnings

Run the narrowest relevant test during iteration, then workspace tests when a shared contract or cross-crate behavior changes. If tooling or network access is unavailable, report that plainly.

## Documentation
- Update docs when a public behavior, format, CLI contract, architecture decision, or roadmap status changes.
- Keep AGENTS.md concise; put detailed design in docs/ and repeatable procedures in skills.
- Mark uncertain decisions as open questions rather than facts.
- Keep examples valid and synchronized with JSON Schema and Rust serialization tests.

## Git hygiene
- Do not use destructive git commands unless explicitly requested.
- Do not overwrite unrelated user changes.
- Do not stage or commit unrelated files.
- Do not add generated build output, local caches, secrets, or personal environment files.
