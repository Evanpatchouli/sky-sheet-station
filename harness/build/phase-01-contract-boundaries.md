# Phase 1 preparation: three contract boundaries

**Status:** Approved scope complete, 2026-11-01. This is a partial Phase 1
milestone, not completion of Phase 1 or a freeze of the full v1 format.

## Approved decisions and compatibility

- Both instrument keys and note pitches must map to MIDI 0..=127. Rust already
  enforced this at `Pitch` construction; the spec and Schema description now
  explicitly assign the range check to semantic validation. The Schema pattern
  still checks syntax only. Pitch spelling and Sky key mapping are unchanged.
- Meter numerator is 1..=65535, retaining `u16`; Schema now has the upper bound.
- Start ticks are 0..=9007199254740991; duration ticks are
  1..=9007199254740991. Rust retains `u64`, adds `MAX_TICK`, and validates these
  narrower public ranges.
- The derived end position must also be <= 9007199254740991. It is not a new
  JSON field. Rust uses `checked_add` so even invalid `u64` inputs produce errors
  without panicking or wrapping.
- Meter denominator remains one of 1, 2, 4, 8, 16, 32, 64; the spec now states
  the same supported set as Schema and Rust.

The user explicitly approved this as a v1 pre-freeze contract revision with
format_version 1.0 retained. Tick limits and the end limit narrow earlier Rust
acceptance; numerator bounds narrow earlier Schema acceptance. These are contract
changes, not merely implementation fixes. Invalid values are rejected, never
clamped, rounded, or normalized. The existing example remains valid and unchanged.

No new production or development dependency was added. General version policy,
pitch normalization, game calibration, MIDI handling, and all other roadmap work
were outside this task and remain undecided or unimplemented as before.

## Files and verification design

- `docs/music-data-format.md`: normative bounds, validation layers, examples of
  legal/illegal note ends, and compatibility notes.
- `docs/schemas/sky-score-v1.schema.json`: scalar maxima and descriptions of
  semantic checks; no new JSON field or pitch-pattern restriction.
- `crates/skysheet-score/src/lib.rs`: tick constant, diagnostics with note ID and
  offending values, checked end validation, and documentation of existing limits.
- `crates/skysheet-score/tests/fixtures/contract-boundaries.json`: 30 mutations
  of the documented example, with separate Schema and Rust expectations.
- `crates/skysheet-score/tests/contract_boundaries.rs`: four integration tests
  covering all 30 cases and round-tripping accepted scores.
- `scripts/test-score-schema.ps1`: validates the unchanged example and the same
  30 cases using PowerShell 7.4+ `Test-Json`, without new project dependencies.

Coverage includes MIDI endpoints and outside-range keys/notes, exact instrument
membership, numerator 1/65535/65536/0, every supported denominator and rejected
0/3/128, zero/maximum/over-maximum tick values, sums equal to and above the limit,
and `u64::MAX + 1`. Some cases intentionally pass Schema and fail Rust: Schema
does not enforce pitch range, membership, or cross-field note ends.

## Observed verification on 2026-11-01

Environment: Windows MSVC, Rust/Cargo 1.99.1. The existing Cargo bin directory was
added only to the process PATH. No toolchain or machine configuration was changed.

| Command/check | Result |
|---|---|
| Boundary tests before implementation | Rust tick test and Schema script both rejected the old acceptance of start_tick = 9007199254740992. |
| `cargo fmt --all` and `cargo fmt --all -- --check` | Passed. |
| `cargo test -p skysheet-score --test contract_boundaries --locked` | 4 integration tests passed, covering 30 cases. |
| `cargo test --workspace --locked` | 19 unit tests + 4 integration tests passed; 0 doc-tests. |
| `cargo test -p skysheet-score --release --test contract_boundaries --locked` | All 4 integration tests passed, including overflow rejection. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0. |
| `pwsh -NoProfile -File scripts/test-score-schema.ps1` | Documented example and all 30 Schema expectations passed. |
| Example and mapping consistency | Inline example equals the standalone fixture; Schema key order matches it. Original fixture and dependency manifests have no diff. |
| Final diff review and `git diff --check` | Passed. Reviewed the Rust changes against a snapshot taken after Phase 0 to separate this revision from earlier uncommitted work; instrument mapping is unchanged. |

Some local Cargo runs printed an incremental-cache access-denied note, with exit
code 0 and successful checks; it affects reuse of build artifacts. Hosted CI was
not run. The new Schema script is a documented local check, not an added CI job.

All work remains unstaged and uncommitted. No other Phase 1 task or later phase
was started; full v1 freeze criteria remain unmet.
