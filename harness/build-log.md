# Build Log

Append a short entry when an approved phase is completed or materially blocked. Do not use this file as a transcript of every agent action.

## Entry template

### YYYY-MM-DD — Phase NN: title

- Outcome:
- Main files changed:
- Verification performed:
- Result:
- Follow-up:

### 2026-11-01 — Phase 00: Repository Foundation

- Outcome: completed the approved foundation scope; contract boundary differences
  are explicitly recorded rather than resolved or treated as a frozen v1 contract.
- Main files changed: score crate tests/parser, CI workflow, `.gitignore`,
  `Cargo.lock`, README files, `PLANS.md`, and the Phase 0 verification record.
- Verification performed: formatting; 19 crate/workspace tests; 6 release pitch
  tests; Clippy; fresh-directory fmt/test/lint; example Schema validation; inline
  example equality; final diff review.
- Result: checks passed. A workspace incremental-cache access-denied note did not
  recur in the successful fresh-directory check. Hosted CI was not run.
- Follow-up: pitch range, numeric limits, and meter denominator differences remain
  open. Phase 1 has not started. Public format and instrument mapping are unchanged;
  no commit or push was made.

### 2026-11-01 — Phase 01 partial: approved contract boundaries

- Outcome: resolved only the approved pitch range, integer bounds/note end limit,
  and meter denominator differences as a v1 pre-freeze revision.
- Main files changed: format spec, Schema, Rust score validator, shared boundary
  fixtures and integration tests, Schema verification script, plan and records.
- Verification performed: formatting, 19 unit tests and 4 integration tests
  covering 30 cases, release boundary tests, Clippy, example and boundary Schema
  validation.
- Result: checks passed; some local builds emitted a non-fatal incremental-cache
  access-denied note. Hosted CI was not run.
- Follow-up: remaining Phase 1 work is incomplete and was not started. Instrument
  mapping, JSON field shapes, and dependencies are unchanged. No commit or push.

### 2026-11-01 — Phase 01 partial: conformance audit and decision draft

- Outcome: expanded existing-rule coverage to 96 shared cases, added the Schema
  CI step, and prepared the pending-policy draft plus calibration/tempo acceptance
  preparation. No draft decision was adopted automatically.
- Main files changed: integration tests/fixtures, observational raw-input audit,
  Schema script and CI, decision/audit documents, documentation links and plan.
- Findings: integer-token and duplicate-member parsing differences; local
  `Test-Json` limits for extreme tempo; Rust incorrectly accepts arrays in some
  object positions. Findings are recorded, not silently fixed or redefined.
- Verification: fmt and Clippy passed; 19 unit tests + 10 integration tests passed
  with one intentionally ignored observational audit; 10 release integration
  tests passed; example and 96 Schema cases passed. The 15 audit probes were run
  separately and their discrepancies recorded. Hosted CI was not run; existing
  incremental-cache notes were non-fatal.
- Follow-up: approve the focused object-only reader fix and review pending
  policies; provide calibration evidence. Full Phase 1 remains incomplete;
  no Phase 2 work, commit, or push was performed.

### 2026-11-01 — Phase 01 partial: object-only reader correction

- Outcome: all seven score object types reject positional arrays, matching the
  existing contract. Fields and serialization have a single declaration source;
  a map-only visitor retains the original Serde field-reading stream.
- Main files changed: score object declarations/private deserializer helper,
  object-type regression tests and shared fixtures/runners, plan and audit records.
- Verification: all 7 new object tests failed before the fix and passed after;
  fmt, 37 normal Rust tests, 18 release integration tests, Clippy, and example plus
  103 shared Schema cases passed. One observational audit is ignored by default
  and was run explicitly for comparison: all 12 non-object outcomes are unchanged.
- Scope: Schema, field types, pitch mapping, example, dependencies, and D1-D8
  decisions unchanged. Local cache notes were non-fatal; hosted CI was not run.
- Follow-up: remaining input-policy/validator questions and calibration evidence
  still prevent full Phase 1 completion. No Phase 2 work or commit/push.

### 2026-11-01 — Phase 01 partial: approved D2-D4 input rules

- Outcome: adopted strict original integer spelling, unique decoded JSON member
  names, and finite positive binary64 tempo; D1 and D5-D8 remain pending.
- Main files changed: specification, Schema descriptions/comments, decision/audit
  records, raw-input regression tests/expectations, test-tool explanations and plan.
- Verification: 43 normal Rust tests passed with none ignored; 6 release input-rule
  tests, fmt, Clippy and 103 structural Schema cases passed. The 15 old raw probes
  now have assertions; added cases cover all integer fields/object levels and
  binary64 subnormal/maximum endpoints.
- Scope: no production Rust, field/range, instrument mapping or dependency change.
  The local Schema tool's extreme-tempo limitations remain documented. Hosted CI
  was not run; no other pending decision, Phase 2 work, commit or push was performed.

### 2026-11-01 — Phase 01 partial: compatibility policies and partial D6

- Outcome: adopted D1/D5/D7/D8 and only D6's diagnostic flattening principle and
  default short-note quantization error. Recorded the owner's confirmation that
  no external consumers currently depend on draft 1.0.
- Main files changed: decision/specification/pipeline documentation, Schema
  descriptions, README files and plan; serialization compatibility tests and
  shared conformance cases; audit follow-up.
- Verification: 47 workspace tests, 15 release integration tests, fmt and Clippy
  passed; the example and all 120 shared Schema cases passed. Four Rust tests and
  17 shared cases were added without changing the existing production reader.
- Scope: format 1.0, production Rust, field/range assertions, mapping, example and
  dependencies unchanged. No profile-version/provenance field or report system.
- Follow-up: remaining D6 details, acoustic calibration and independent numeric
  verification remain open; no full freeze or Phase 2 entry. Existing local cache
  notes were non-fatal. Hosted CI was not run; no commit or push.

### 2026-11-01 — Phase 01 partial: approved D6-A through D6-F documentation

- Outcome: recorded approval of reference/source-tempo rules, absolute rounding,
  reliable arithmetic/failure, nominal meter and PPQ-based SMF Format 0/1 scope.
  D6 policy is approved; importer/quantizer implementation and real-file verification
  remain unperformed. Format 2/SMPTE support requires separate decisions.
- Main files changed: decision/specification/pipeline documentation, README files,
  plan and audit records. Added analytical boundary examples and clarified the
  earlier error-distribution wording; no production or test algorithm was added.
- Verification: formatting, 16 existing contract/input-rule tests, and the example
  plus all 120 existing Schema cases passed. Checked 52 relative links/anchors,
  inline example equality and exact arithmetic for the documented boundaries.
  Snapshot comparison confirms only eight Markdown files changed. These checks
  do not verify MIDI conversion.
- Follow-up: numeric implementation/file acceptance work needs separate authorization;
  calibration and independent score-numeric verification still prevent full freeze.
  Local cache notes were non-fatal. No Phase 2 work, commit or push.
