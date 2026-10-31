# Phase 00: Repository Foundation

**Status:** Complete for the approved Phase 0 scope (2026-11-01), with explicitly deferred contract boundary differences.

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

The repository contains one workspace member, `skysheet-score`. The observed
build and validation results are recorded below.

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

Scope clarification approved by the product owner on 2026-11-01: fix clear
implementation defects and establish the baseline, but record contract boundary
differences without changing the public JSON format or instrument mapping.
Full agreement on those boundaries remains unresolved; it is not claimed by
this Phase 0 completion record.

## Verification

Run from the repository root:

- cargo fmt --all -- --check
- cargo test --workspace --locked
- cargo clippy --workspace --all-targets --locked -- -D warnings

If dependency fetching or toolchain setup fails, record the exact failure. Do not claim the phase is complete.

## Completion record

### Changes

- `Pitch::parse` checks the existing Schema's octave syntax before numeric parsing,
  rejecting `C+4` and `C#+4`. Arithmetic widens the parsed `i16` octave to `i32`
  before addition/multiplication, preventing debug panics and release wrapping.
  Valid pitch strings are preserved; no pitch normalization or mapping change.
- Added 12 tests (19 total) covering direct and Serde pitch parsing, overflow,
  MIDI boundaries, invalid metadata, duplicate IDs/keys, key order/count, empty
  required strings, unknown fields including velocity, invalid ticks/enums,
  optional text, sustain, track roles, and legal overlapping notes.
- Formatted Rust code, added `/target/` to `.gitignore`, and retained the generated
  `Cargo.lock` as a repository deliverable. No dependency declarations changed.
- CI now requests rustfmt and Clippy and runs formatting, locked tests, and lint.
  README documents prerequisites, commands, PATH troubleshooting, and Schema checks.
- Fixed literal newline escapes in `docs/README.md`.

### Verification observed on 2026-11-01

Environment: Windows, `stable-x86_64-pc-windows-msvc`, rustc 1.99.1
(`58cf8f9c3`, 2026-10-13), Cargo 1.99.1 (`5f94df478`, 2026-08-27).
The existing Cargo bin directory was added to the process PATH; no machine-wide
configuration or toolchain installation was changed.

| Check | Result |
|---|---|
| Before-fix `cargo test -p skysheet-score pitch_ --locked` | Two new regression tests failed: arithmetic overflow and accepted `C+4`. |
| `cargo fmt --all` followed by `cargo fmt --all -- --check` | Passed. |
| `cargo test -p skysheet-score --locked` | 19 passed; 0 failed; 0 doc-tests. |
| `cargo test --workspace --locked` | 19 passed; 0 failed; 0 doc-tests. |
| `cargo test -p skysheet-score --release pitch_ --locked` | 6 passed, 13 filtered out; confirms overflow fix with release settings. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0. One workspace run printed an incremental-cache access-denied note; fresh-directory verification below passed without it. |
| Fresh source snapshot: fmt, locked workspace tests, locked Clippy | All passed using a new temporary directory with no copied `target/`; 19 tests passed. Existing Cargo registry cache was available. |
| `Test-Json` against `docs/schemas/sky-score-v1.schema.json` | Example passed. |
| Parsed inline example versus standalone fixture | Equal; Schema's exact Sky key list also matches fixture order. |
| `git diff --check` and final scope review | Passed; public format specification, Schema, fixture, dependency manifests, and pitch mapping unchanged. |

The clean-directory check copied current source files plus `Cargo.lock` and
`.gitignore`, rather than checking out a new commit: these changes are intentionally
uncommitted. Hosted Ubuntu CI has been configured but not executed in this task.
The toolchain version above is an observed baseline, not a new MSRV or platform policy.

### Open contract boundaries at Phase 0 completion (historical)

These observations were recorded without changing the format in Phase 0. The
subsequently approved [three-boundary revision](phase-01-contract-boundaries.md)
resolves their contract decisions; this table preserves the original findings.

| Boundary | Current difference and reproduction | Disposition |
|---|---|---|
| Pitch range | For a custom instrument, `keys: ["C10"]` with empty tracks passes Schema; `Pitch::parse("C10")` rejects MIDI values outside 0..=127. The prose does not explicitly impose that range. | Preserve existing behavior; decide the public range separately. |
| Integer limits | Set meter numerator to 65536: Schema accepts it, but Rust uses `u16`. Tick fields use `u64`, while Schema declares no upper bound. | Do not add Schema maxima or widen public types in Phase 0. |
| Meter denominator | Prose allows positive powers of two without an upper bound; Schema and Rust allow only 1 through 64. A denominator of 128 illustrates the difference. | Record the conflict; do not change acceptance or silently amend the normative spec. |

At Phase 0 completion these were unresolved design/compatibility questions, not newly approved requirements.
Schema structural validation is also intentionally insufficient for cross-track note
ID uniqueness and pitch membership; Rust semantic validation supplies those checks.
At that milestone, Sky octave labels were treated as an unverified calibration
assumption. That requirement is superseded by the owner's later
[logical-pitch decision](../../docs/phase-1-decisions.md#logical-sky-pitches-and-actual-game-sound):
the unchanged C4-C6 mapping no longer requires acoustic measurement. No MIDI behavior,
new public fields, or Phase 1 implementation was introduced.

### Review reference

Local final diff review for this Phase 0 task. All changes remain unstaged and
uncommitted; no push, release, or deployment was performed. The test and build
artifacts are outside versioned source or ignored under `target/`.
