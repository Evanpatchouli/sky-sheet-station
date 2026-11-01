# SkySheet Station Development Plan

This is a staged roadmap, not authorization to implement every phase in one task. Work only on the phase requested by the user. Update status and evidence as phases are completed.

## Phase 0: Repository and agent foundation
**Status:** Complete for the approved Phase 0 scope (2026-11-01); contract boundary differences were recorded at completion and addressed by the subsequent approved revision below.

- [x] Initial docs directory and architecture notes.
- [x] Initial score-format proposal, JSON Schema, and example.
- [x] Initial Rust workspace and score crate skeleton.
- [x] Establish and verify a reproducible local Rust test workflow.
- [x] Review the current score crate against the score specification, fix clear implementation mismatches, and record unresolved contract boundaries.

Exit criteria: a clean checkout can run formatting, tests, and lint; the example fixture deserializes, validates, and round-trips.

Evidence: formatting, 19 workspace tests, Clippy, and example Schema validation
passed locally. A fresh source snapshot with the lockfile and no build output
also passed formatting, tests, and lint. Six pitch-related tests passed in release
mode. CI now includes formatting and lint; hosted CI has not been run for these
uncommitted changes. See the [Phase 0 verification record](harness/build/phase-00-repository-foundation.md).

At Phase 0 completion, public JSON fields, Schema, fixture, and instrument mapping
were unchanged. Numeric limits, pitch range, and meter denominator differences
were recorded as open questions. The follow-up below resolves only those three
boundaries; neither milestone means that the full v1 contract is frozen.

## Phase 1: Freeze the score contract
**Status:** In progress. D1-D8 and N1-N4 are approved (2026-11-01). N2 RawValue repair acceptance is complete: Windows checks and Linux CI for `5cee8ef` pass. Remaining Phase 1 contract checks are separate; the full contract is not frozen. Acoustic calibration is outside scope; sky-15 retains its C4-C6 logical mapping. D6 approval is not MIDI implementation/verification; no Phase 2 work is authorized.

- [x] Resolve pitch range, integer bounds including note ends, and meter denominators as an approved v1 pre-freeze revision.
- [x] Expand existing-rule fixtures, configure Schema CI, audit raw inputs, and prepare non-normative decision/calibration/tempo-policy drafts.
- [x] Fix array representations incorrectly accepted for all seven score object types, with direct/nested regression coverage.
- [x] Adopt D2-D4 in the specification/Schema descriptions and promote direct raw-input cases to normal regression tests, without changing existing reader behavior.
- [x] Adopt D1/D5/D7/D8 and partial D6 in documentation, with regression coverage for existing serialization and validation behavior.
- [x] Document serialization compatibility and format-version policy; record the owner's confirmation that no external consumers currently depend on draft 1.0.
- [x] Approve and document D6-A through D6-F, PPQ-based SMF Format 0/1 scope and analytical boundary examples, without implementing conversion algorithms.
- [x] Define sky-15's unchanged C4-C6 mapping as logical score pitches; remove game-recording and acoustic-calibration requirements by owner decision.
- [x] Establish N1-N4 independent raw/Schema/numeric verification, locked development/CI dependencies, exact boundary expectations and deterministic sampling; report current failures without changing parser features.
- [x] Migrate all validation to isolated Rust tooling under the owner-approved stack; preserve 267 fixed cases, all 10,000 historical samples, original reports and strict N1-N4 outcomes. No parser precision fix is included.

At migration completion, local root/tool checks passed while full numeric conformance
reported the same 2,981 failures. That historical baseline is preserved; the
subsequent RawValue correction is recorded below. Two ignored legacy environment/cache directories remain unused because
automatic approval review blocked their deletion; tracked runtime dependencies
and invocation paths are removed. See the
[migration record](harness/build/phase-01-conformance-audit.md#rust-only-verification-migration).

The owner-approved N2 production correction now uses a JSON-specific RawValue
field adapter and Rust standard-library conversion, retaining `tempo: f64`, the
format and validation rules. Windows checks pass: 267 fixed cases, all 10,000
historical samples, the separate 3,016 research cases, 84 focused regressions,
51 Score tests (debug/release), 20 independent tests, fmt and Clippy. Oracle,
comparison rules, historical evidence and both lockfiles are unchanged.
Linux CI at baseline `36a5073` failed on the old 2,981 differences. After the
owner-authorized commit/push, [Linux CI for `5cee8ef`](https://github.com/Evanpatchouli/sky-sheet-station/actions/runs/41256992799)
passed all steps. Downloaded artifacts confirm zero differences for the historical
corpus, 3,016 research cases and 84 focused cases. N2 repair acceptance is complete;
see the [repair record](harness/build/phase-01-n2-raw-value.md).

The revision keeps format_version 1.0 and existing JSON field/Rust storage types.
Pitch values map to MIDI 0..=127; meter numerators are 1..=65535; denominators are
1, 2, 4, 8, 16, 32, 64; tick fields and derived note ends are bounded by 2^53 - 1.
Schema covers field bounds; Rust additionally checks pitch range and note ends.
No instrument mapping, pitch-spelling policy, or MIDI importer behavior changed.
Verification: 19 unit tests plus 4 integration tests covering 30 shared boundary
cases; release boundary tests; formatting; Clippy; example and boundary Schema
checks. See the [revision record](harness/build/phase-01-contract-boundaries.md).

The follow-up audit adds 66 conformance cases (96 shared cases total) and documents
integer-token differences, duplicate-member parsing differences, local Schema
validator limitations for extreme tempo, and a confirmed Rust array/object
deserialization mismatch. Production code, Schema, mapping, and accepted ranges
were not changed in that task. See the [audit](harness/build/phase-01-conformance-audit.md)
and [pending decision draft](docs/phase-1-decisions.md). Draft preparation does not
approve its proposals; calibration and real MIDI validation had not been performed
at that milestone. The former calibration prerequisite is now superseded by the
logical-pitch decision below, not marked as a completed measurement.
Verification passed: 19 unit tests + 10 integration tests, release integration
tests, fmt, Clippy, and all 96 shared Schema cases. The 15 observational probes
document known differences; they are not evidence that those differences are resolved.

The subsequent owner-approved fix now requires objects for `Score`, `Metadata`,
`Instrument`, `Timeline`, `TimeSignature`, `Track`, and `Note`. It preserves the
existing field reader and serialization, with no Schema, mapping, dependency,
or other acceptance-policy changes. All 7 new object-type tests first failed on
the original positional arrays, then passed after the fix. Verification passed:
37 normal Rust tests, 18 release integration tests, fmt, Clippy, and 103 shared
Schema cases. The 12 non-object audit observations and their diagnostics match
the pre-fix snapshot. See the [audit follow-up](harness/build/phase-01-conformance-audit.md#approved-follow-up-object-only-deserialization).
That repair did not adopt D1-D8. The subsequent D2-D4 approval below is separate.

D2 now requires plain unsigned integer tokens for all five integer fields; D3
rejects duplicate decoded member names; D4 uses finite positive decoded binary64
tempo, including positive subnormals. Original JSON must be read directly before
information is discarded, then the score must pass semantic validation. The
existing reader needs no production change for these approved cases. Schema
assertions, field shapes/ranges, mapping, and dependencies are unchanged; only
Schema descriptions/comments explain the additional input checks. The current
`Test-Json` numeric limitation remains explicit and does not redefine validity.
See the [decision record](docs/phase-1-decisions.md) and
[D2-D4 audit follow-up](harness/build/phase-01-conformance-audit.md#approved-follow-up-d2-d4-input-rules).
Verification passed: 43 normal Rust tests (none ignored), 6 release input-rule
tests, fmt, Clippy, and 103 structural Schema cases. The former 15 raw probes
are now asserted normal tests; the optional Schema raw diagnostics still expose
known engine limits. No production code or dependency change was required.

The subsequent compatibility approval preserves pitch strings and exact key
identity, establishes exact-version support and post-freeze version rules, adds
no profile-version field, and keeps provenance outside v1. D6 permits flattening
with a loss diagnostic and defaults short-note quantization failures to errors;
reference/source-tempo, rounding, numeric-failure and meter rules remained pending
at that milestone.
Current reader behavior, format version 1.0, mappings, example and dependencies
are unchanged. See the [decision record](docs/phase-1-decisions.md) and
[compatibility follow-up](harness/build/phase-01-conformance-audit.md#approved-follow-up-d1-d5-d7-d8-and-partial-d6)
for scope and verification. These approvals do not complete the freeze gate.

The owner then approved D6-A through D6-F. The documentation-only follow-up records
explicit-or-initial reference tempo, standard missing-source defaults, diagnosed
cross-track merging with same-tick conflict errors, absolute endpoint rounding,
reliable arithmetic and whole-conversion failure, initial nominal meter and
diagnosed discarded changes. Scope is PPQ-based SMF Format 0/1. See
[format section 16](docs/music-data-format.md#16-approved-midi-timing-conversion-policy-d6)
for boundary examples and future verification, and the
[D6 documentation record](harness/build/phase-01-conformance-audit.md#approved-follow-up-d6-a-through-d6-f-documentation-only).
No importer/quantizer, dependency or test algorithm was added. Format 2/SMPTE
support remains outside this approval. Phase 2 remains planned and unauthorized.

The owner subsequently clarified that actual Sky sounding pitches may vary with
the game environment. The existing ordered C4-C6 profile is the standard logical
score mapping, not an acoustic measurement or a promise of a fixed game tuning.
Game recordings and physical pitch validation are no longer required. This removes
the calibration prerequisite without changing keys, JSON fields, Rust structures
or the remaining verification requirements. See the
[logical-pitch decision](docs/phase-1-decisions.md#logical-sky-pitches-and-actual-game-sound).

The following broader Phase 1 items remain incomplete:

- [ ] Finalize v1 JSON schema and semantic validation.
- [ ] Complete positive/negative fixture coverage for the finalized contract (existing rules expanded; unresolved findings/policies still require follow-up).
- [x] Complete N2 repair acceptance: Windows numeric/engineering checks and Linux CI for `5cee8ef` pass, with original expectations and evidence preserved. See [numeric verification](docs/numeric-verification.md). This does not complete the remaining Phase 1 checks or authorize Phase 2.

Exit criteria: score specification, schema, example fixtures, Rust types, and tests
agree, including a passing independent numeric gate with no unresolved N2 differences. Acoustic
calibration and game recordings are not Phase 1 completion conditions. Removing
that requirement does not itself complete Phase 1 or start MIDI implementation.

## Phase 1.5: CLI Alpha — first runnable experience
**Status:** Implemented and locally verified on Windows (2026-11-01). This is
the current delivery milestone; no Phase 2 work is started. N2 is complete and
its original production reader, expectations and evidence remain unchanged.

- [x] Add `skysheet` help/version and inspect/validate commands using the existing Score API.
- [x] Read raw JSON directly, then call `Score::validate()` without an alternate validator.
- [x] Document stable JSON results, error categories and exit codes 0–4.
- [x] Add real-process CLI integration tests, including the existing 84 N2 cases.
- [x] Build and run Windows x86_64 MSVC Release executable on real fixtures.
- [x] Run workspace and isolated numeric fmt/tests/Clippy and the full numeric gate.

Exit criteria: Windows users can run `skysheet.exe`, inspect and validate the
example, consume success/failure JSON, and distinguish invalid scores, usage and
I/O failures by exit code. Tests and executable identity are recorded in the
[Alpha verification record](harness/build/phase-015-cli-alpha.md); usage is in
[CLI Alpha](docs/cli-alpha.md). Local verification does not claim new hosted CI
results or a published release.

Remaining Phase 1 work is explicitly separate:

- CLI blockers: none identified by implementation and verification.
- Nonblocking contract work: final specification/Schema/semantic audit remains open.
- After Alpha: finish positive/negative coverage for the finalized contract;
  retain the passing N2 gate. Real MIDI/D6 implementation tests belong to Phase 2.

P2 follow-ups: richer field locations, shell completion, localization and release
automation. MIDI, audio/music analysis and arrangement remain later phases. GUI
was deferred at Alpha delivery; the owner subsequently authorized the Score-first
Phase 6 below. Phase 1 is not marked complete.

## Phase 2: MIDI interoperability
**Status:** Implemented and locally verified on Windows (2026-11-01), from main
`6a91bef` on `codex/phase-2-midi`, after explicit dependency approval. This
supersedes earlier milestone statements that Phase 2 was unauthorized, without
completing Phase 1 or starting Phases 3/4. See [MIDI usage and verification](docs/midi.md)
and the [approved dependency/design record](docs/phase-2-midi.md).

- [x] Select and document the Rust MIDI library.
- [x] Import tracks, notes, note-on/off timing, meter, and tempo metadata.
- [x] Implement and verify the approved D6 timing policy within its scope; document rejected unsupported MIDI constructs.
- [x] Export a score to MIDI and test round-trip properties and protocol limits.
- [x] Report information that cannot be preserved.
- [x] Connect MIDI import through the packaged CLI to the existing desktop player,
  preserving the current Score on cancellation or failure and exposing diagnostics.

Exit criteria: representative MIDI fixtures import, validate, and export with documented behavior.

Evidence: 84 root tests, 17 MIDI tests in release, 7 desktop Rust tests, 8 frontend
tests, three-workspace formatting/Clippy, frontend/release application builds,
and unchanged independent 20-test / 267-fixed / 10,000-sample numeric checks passed.
Native Windows file selection loaded a Unicode/space-path MIDI, with advancing
playhead and key highlights; the supplied ode-to-joy file failed for 22 out-of-range
notes while preserving the loaded Score. Local Playwright/Edge checked reports,
cancel/error retention and 1280/800/390 widths. No new CI run, commit or publication
is claimed; physical speaker output was not measured.

## Phase 3: Music analysis primitives
**Status:** P3-A/B/C implemented and locally verified on Windows after the full-prompt reconciliation. Required CLI commands and the native desktop analysis flow display the real ode-to-joy source. Heuristics remain uncalibrated; external asset provenance and human musical review remain open. Phase 2 hosted CI has no run, not a passing result. See [final delivery evidence](docs/phase-3-delivery.md).

- [x] Complete P3-A: optional MIDI provenance, generic notes, unrestricted pitch display/helpers and source preservation.
- [x] Complete P3-B: register/track/pair analysis, musical positions, score components and specific diagnostics.
- [x] Prototype traceable candidate melody extraction and role estimation.
- [x] Create 14 original synthetic evaluation cases, separate from owner-supplied MIDI.
- [x] Add explicit score-component/ranking/Unknown regressions and all 14 corpus cases through the MIDI path.
- [x] Complete P3-C: `inspect-music`, `analyze`, desktop analysis UI and actual ode-to-joy display.
- [x] Run CLI/desktop/frontend/build checks and verify existing Score playback.
- [ ] Obtain human-reviewed references and resolve external real-MIDI provenance.
- [x] Record provisional reference agreement, runtime, memory and engineering checks; human-validated accuracy remains pending.

`analyze-midi` uses the existing parser and reusable music crate. All 52
ode-to-joy notes, including 22 out-of-range accompaniment notes, remain unchanged.
Internal velocity never enters Score v1. Confidence is uncalibrated; Phase 4
arrangement is not included.

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
**Status:** Broader product surface planned; inspect/validate foundation delivered in Phase 1.5.

- [ ] Implement convert and export commands after their core phases.
- [x] Implement inspect and validate in Phase 1.5.
- [x] Define Alpha exit codes and diagnostics.
- [x] Keep Alpha JSON output clean for scripting.
- [x] Add end-to-end inspect/validate CLI tests using fixture files.
- [ ] Extend diagnostics and end-to-end tests to future convert/export workflows.

Exit criteria: documented workflows work headlessly from a clean checkout.

## Phase 6: Tauri Desktop Player
**Status:** Score-first desktop player implemented and locally verified on Windows (2026-11-01), within the owner's Phase 6 authorization. Audio conversion is absent; no Phases 2–5 implementation is implied. No installer/release has been published.

- [x] Implement React/TypeScript/Vite desktop workflows around actual CLI capabilities.
- [x] Package the CLI as a Tauri sidecar with argument-array invocation.
- [x] Reuse raw-byte Score parsing and CLI validation, including N2; isolate desktop dependencies.
- [x] Canvas piano roll, dynamic ordered-key keyboard, nominal meter guides, zoom and scrolling.
- [x] Local synthesized audition, synchronized pointer/highlights, pause/stop/seek/speed/loop controls.
- [x] File-picker cancellation, loading state and actionable CLI errors.
- [x] Audio-file selection explicitly marked as unconnected; no conversion output fabricated.
- [x] Complete Windows executable and rendered/native workflow verification.

Local playback is a desktop-client responsibility, not new core or game-input
behavior. Conversion progress/cancellation is deferred until a real CLI conversion
command exists. See [desktop player](docs/desktop-player.md) for usage and limitations.

Evidence: root 64 tests, desktop 5 Rust tests (including 84 frozen N2 cases),
8 frontend timing/playback tests, isolated numeric 20 tests, fmt/Clippy, the full
267-fixed/10,000-sample gate, TypeScript/Vite production build, and Windows release
build passed locally. Native Release loaded a real Unicode/space path through its
file picker and CLI with the dev server stopped; playback advanced the pointer
and key highlights. Local browser checks exercised controls, errors, responsive
layout and a 600-note validated test fixture. See the verification section in
[desktop player](docs/desktop-player.md#local-verification-2026-11-01).

## Deferred: MCP
MCP is a possible future integration. Do not implement it as part of the initial phases unless the product owner explicitly reprioritizes it.
