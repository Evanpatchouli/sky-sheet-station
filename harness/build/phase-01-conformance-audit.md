# Phase 1: conformance audit and decision preparation

**Historical tooling notice:** pre-migration interpreter and PowerShell commands
below record earlier verification only; they are retired, not current setup or
runtime requirements. The current Rust-only workflow is in
[numeric-verification.md](../../docs/numeric-verification.md). The latest migration
record below preserves outcomes without approving old runtimes for future work.

**Initial audit scope:** existing-contract tests and CI, raw-input observations, and preparation
of a decision draft. The owner approved these tasks after the three-boundary
revision. No new input policy, public field, instrument mapping, or production
dependency is authorized here. Full Phase 1 completion and Phase 2 are excluded.

**Approved follow-ups:** the owner subsequently authorized the object-only repair,
D2-D4 input rules, D1/D5/D7/D8 and partial D6 adoption, then the documentation-only
D6-A through D6-F follow-up. See the records below. Historical observations are
preserved. The owner's later logical-pitch decision removes acoustic calibration
and recording collection from scope; earlier calibration requirements in this
record are historical and superseded. N1-N4 now implement the independent numeric
gate; its observed Rust/N2 differences remain unresolved (see the latest follow-up);
D6 policy approval is not implementation/file verification. The full contract is
not frozen and no Phase 2 implementation is authorized.

## Rule-to-implementation coverage

Rule numbers refer to section 11 of `docs/music-data-format.md`. “Schema” here
means its declared constraints, not a claim that every validator has identical
numeric parsing behavior. Rust validation means direct parsing plus `Score::validate`.

| Rule | Schema responsibility | Rust responsibility | Evidence |
|---|---|---|---|
| 1: format version | `const` 1.0 | Exact version check | `semantic: unknown version`; unit version test |
| 2: required objects/fields | `required` and types | Object-only entry plus Serde field reading after the approved fix | 24 `required:` cases, 8 `types:` cases, 7 shared object cases, and direct/nested tests for all 7 object types |
| 3: PPQ | `const` 960 | PPQ and approved D2 plain-token reading | `semantic: wrong ppq`; raw-input rule tests |
| 4: finite positive tempo | Positive numeric value | Approved D4 finite positive decoded `f64`, including subnormals | Shared cases, nonfinite unit cases, raw tests through binary64 extremes |
| 5: meter | Numerator bounds and denominator enum | `u16`, positive numerator and supported denominator | Existing numerator/denominator boundary cases; raw spelling audit |
| 6: start tick | Integer bounds | `u64`, upper bound | Tick boundary cases; negative/fractional unit cases; string type case |
| 7: duration | Positive integer bounds | `u64`, positive and upper bounds | Boundary and fractional-duration cases |
| 8: exact pitch membership | Not expressed dynamically | Exact `Pitch` string equality | In-range but unplayable case; custom-profile boundary example |
| 9: unique ordered keys | Nonempty array, `uniqueItems` | Nonempty keys, string uniqueness, retained order | Empty/duplicate custom keys; derived key-number unit test |
| 10: Sky profile | Exact canonical array | Exact canonical array | Wrong-order/count shared cases |
| 11: playback mode | Enum | Serde enum | Invalid mode shared case; trigger example and sustain/overlap case |
| 12: unique track IDs | Not expressed across objects | Track ID set | Duplicate track shared case |
| 13: unique note IDs | Not expressed across nested arrays | Score-wide note ID set | Within-track and across-track duplicate shared cases |
| 14: legal overlap | No overlap prohibition | No overlap prohibition | Simultaneous example and sustain/overlap case |
| 15: unknown fields | `additionalProperties: false` | Serde `deny_unknown_fields` | Shared cases at every object level, including forbidden velocity |
| 16: MIDI pitch range | Syntax only, semantic requirement described | Validated `Pitch` parser | Endpoint and outside-range keys/notes; pitch unit tests |
| 17: note end | Individual fields only | Checked sum and limit | Equal/over-limit ends and `u64` overflow boundary cases |
| 18: integer spelling (D2) | Described, not enforced on parsed numbers | Direct unsigned integer field deserialization | All five integer fields reject decimal/exponent forms, negative zero, and strings |
| 19: member-name uniqueness (D3) | Described; parsed objects may already have lost duplicates | Direct field reader rejects repeated decoded member names | All seven object levels, same/different values, escape equivalence, null text |

Additional field rules: nonempty IDs/names have seven shared negative cases;
optional text has omitted/null/empty positive cases and a numeric negative case;
Unicode strings and empty track/note arrays retain current acceptance. Track role
variants and invalid role are covered by existing unit tests and the shared suite.

The earlier 30 boundary cases remain intact. The initial audit added 66 conformance cases
and five grouped integration tests, plus a catalog test ensuring every case name
is unique within its fixture and assigned to one test group. No production reader
or validator behavior was changed during that initial audit. The separately
approved repair below adds 7 shared cases, for 103 in the current suite.

## Raw JSON audit: historical observations and current regression status

Reproduction inputs are in `crates/skysheet-score/tests/fixtures/raw-input-audit.json`.
Twelve probes replace the first specified text token in the unchanged example,
preserving number spelling and duplicate members; their direct path has no initial
generic JSON parse. Three additional object-shape probes substitute arrays for
objects structurally. Rust reports a separate `Value`-first path for comparison.

Historical commands from the original audit (the Schema script is now retired):

```text
cargo test -p skysheet-score --test raw_input_audit --locked -- --nocapture
pwsh -NoProfile -File scripts/test-score-schema.ps1 -AuditRaw
```

Originally these probes were ignored observations. After explicit D2-D4 approval,
their direct Rust outcomes are asserted in normal Cargo/CI regression tests.
The Value-first comparison remains diagnostic, not a valid original-file input
path. `-AuditRaw` still prints tool observations after the structural checks;
it does not assert complete score validity or hide numeric limitations.

Original observations before the object-only repair, with the existing lockfile,
Rust/Cargo 1.99.1 and PowerShell 7.6.1. Re-running now rejects the three array
cases; the other 12 probe results are unchanged:

| Probe | Direct Rust | Rust via generic `Value` | Local `Test-Json` | Classification |
|---|---|---|---|---|
| start_tick 1.0 / 1e0 / -0 | Rejects during parsing | Rejects | Accepts | Lexical integer-input policy unresolved |
| ppq 960.0; numerator 4e0 | Rejects during parsing | Rejects | Accepts | Same integer-token issue for metadata |
| Duplicate metadata `name` | Rejects duplicate field | Accepts after duplicate information is lost | Rejects during JSON parsing | Input-path difference; duplicate-member policy should be explicit |
| Duplicate `text`, null/string in either order | Rejects duplicate field | Accepts last value | Rejects during JSON parsing | Same duplicate-member issue, including optional data |
| tempo 1e400 | Rejects numeric overflow | Rejects | Rejects during JSON parsing | Representation boundary, not a proposed musical tempo limit |
| tempo 1e-400 | Decodes to zero, then rejects validation | Same | Rejects positive-number check after effective zero | Representation/underflow policy unresolved |
| tempo 1e-300 | Accepts finite positive value | Accepts | Rejects positive-number check, diagnostic shows zero | Local Schema-validator numeric limitation |
| tempo 1e300 | Accepts finite positive value | Accepts | Rejects during JSON parsing | Local Schema-validator numeric limitation |
| metadata as `["score-001", "Example song"]` | Accepts and validates | Accepts | Rejects: object required | Confirmed implementation/Schema mismatch, not an open format choice |
| meter as `[4, 4]` | Accepts and validates | Accepts | Rejects: object required | Same structural mismatch |
| note as `["note-001", "C4", 0, 480]` | Accepts and validates | Accepts | Rejects: object required | Same structural mismatch |

The array/object cases exposed Serde's sequence representation for derived structs.
The public format already requires objects. The initial audit left the reader
unchanged and recorded these as contract failures, not supported encodings. The
approved follow-up checked all seven object types and fixed their entry points;
named-field serialization is retained. The three old probes remain available for
historical comparison, while proper rejection is now asserted by regression tests.

JSON Schema defines `integer` by zero fractional part, rather than original token
spelling ([type definition](https://json-schema.org/draft/2020-12/json-schema-validation#section-6.1.1)).
Its numeric model does not impose implementation-specific bounds
([numeric interoperability](https://json-schema.org/draft/2020-12/json-schema-validation#section-4.2)).
RFC 8259 recommends unique object names and describes inconsistent duplicate-name
handling across implementations ([section 4](https://www.rfc-editor.org/rfc/rfc8259.html#section-4)).
These sources explain why observations must be separated from product decisions.

The approved D2-D4 decisions retain strict integer input and finite positive
binary64 tempo; they do not silently coerce ticks or clamp tempo. The local
validator is not authoritative over the full contract. Passing shared cases is
not proof of full Rust/Schema-validator equivalence. Current decisions and pending
proposals are in the [decision record](../../docs/phase-1-decisions.md).

## CI and verification

At the original audit, the workflow invoked `scripts/test-score-schema.ps1` using `shell:
pwsh`. The script requires PowerShell 7.4+ and tests the example plus the shared
fixture files. After D2-D4 approval, direct Rust raw-input tests run in normal
Cargo CI; the Schema tool's raw observations are still not a full-validity gate.
No new action, package, production dependency, or external service was added.
The shell configuration follows [GitHub Actions shell syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idstepsshell).

Observed during the initial audit on 2026-11-01, before the object-only fix,
Windows MSVC, Rust/Cargo 1.99.1, PowerShell 7.6.1:

| Command/check | Result |
|---|---|
| `cargo fmt --all` then `cargo fmt --all -- --check` | Passed. |
| `cargo test -p skysheet-score --test contract_boundaries --locked` | Passed during iteration; final suite includes 10 integration tests including the catalog check. |
| `cargo test --workspace --locked` | 19 unit tests and 10 integration tests passed; 1 observational audit intentionally ignored; 0 doc-tests. |
| `cargo test -p skysheet-score --release --test contract_boundaries --locked` | All 10 integration tests passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0. |
| `pwsh -NoProfile -File scripts/test-score-schema.ps1` | Example plus all 96 shared cases passed. |
| Explicit Rust audit and Schema `-AuditRaw` | All 15 probes executed; differing outcomes are recorded above, not asserted as approved behavior. |
| Scope comparison against the start-of-task snapshot | Production Rust source, Schema, example, dependency manifests and lockfile unchanged. |
| Inline example comparison and `git diff --check` | Passed. |

Some Cargo invocations emitted an existing incremental-cache access-denied note;
checks completed successfully with exit 0. No cache cleanup or machine-wide
configuration change was performed. Hosted CI was configured but not executed,
because no commit or push is authorized. All changes remain unstaged/uncommitted.

## Approved follow-up: object-only deserialization

Completed on 2026-11-01 under explicit authorization to fix only array-for-object
acceptance. This is a correction to the existing contract: previously accepted
positional arrays now fail to deserialize; valid JSON objects retain their shape
and behavior. Format version, Schema, fixture, instrument mapping, dependencies,
and the pending D1-D8 policies are unchanged.

Inventory of all exported score object types:

| Rust type | JSON location | Coverage |
|---|---|---|
| `Score` | Root | Direct and root-array rejection |
| `Metadata` | `metadata` | Direct and nested rejection |
| `Instrument` | `instrument` | Direct and nested rejection |
| `Timeline` | `timeline` | Direct and nested rejection |
| `TimeSignature` | `timeline.time_signature` | Direct and nested rejection |
| `Track` | `tracks[]` entries | Direct and nested rejection |
| `Note` | `tracks[].notes[]` entries | Direct and nested rejection |

`Pitch` and the enums are string representations; collections remain arrays.
`ValidationErrors` has no Serde score-format representation. No other current
score object types were found.

Implementation: a private `score_object!` declaration macro keeps each public
struct and its private Serde-derived field reader in a single field declaration.
The entry visitor requests a map and forwards its original `MapAccess` stream to
that reader. It does not deserialize through a buffered `Value`, normalize input,
or run additional semantic checks. Existing field types, defaults, missing-field,
duplicate-field, and unknown-field handling remain with Serde; serialization
continues to use the original named-field derive.

Tests in `tests/object_shapes.rs` cover all seven types through `from_str`,
`from_reader`, and `from_value`, plus nested `Score` input through text and Value.
They reject complete positional arrays, empty/short/long arrays, and scalar/null
inputs. Positive tests reverse member order, require object serialization, and
round-trip values. Existing tests cover optional text, unknown/missing fields,
enums, pitch strings, tick boundaries, and other established behavior.

Seven shared Schema/Rust cases were added in `fixtures/object-shapes.json`.
The fixture runners now support root and array-element replacement so Score,
Track, and Note cases exercise their actual locations. The earlier 96 cases
and their expectations are unchanged. The existing CI command runs all 103 cases.

Verification observed:

| Check | Result |
|---|---|
| Before-fix `cargo test -p skysheet-score --test object_shapes --locked` | All 7 new tests failed because complete positional arrays were accepted. |
| After-fix object and shared integration tests | All 7 object-type tests and 11 shared-suite tests passed. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo test --workspace --locked` | 19 unit + 11 shared-suite + 7 object-type tests passed (37 total); 1 observational audit ignored; 0 doc-tests. |
| `cargo test -p skysheet-score --release --test object_shapes --test contract_boundaries --locked` | All 18 integration tests passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0. |
| `pwsh -NoProfile -File scripts/test-score-schema.ps1` | Example plus 103 shared cases passed. |
| Explicit raw audit versus a pre-fix snapshot | The 3 original array cases now reject; all 12 other observations, including diagnostics on both direct and Value-first paths, are identical. |

The audit comparison is evidence of no observed collateral change, not adoption
of any pending policy. In particular, the integer spelling, duplicate-member
information loss on a Value-first path, and extreme-tempo validator differences
remain as recorded. Existing incremental-cache access-denied notes did not fail
checks. Hosted CI was not run; no commit or push was performed.

## Remaining owner input and stop boundary

The owner approved D1-D8, including D6-A through D6-F, and confirmed no external
consumers currently depend on draft 1.0. Independent numeric verification and
remaining contract checks are still needed; acoustic calibration is no longer
required under the owner's logical-pitch decision. The approved D6 policy and analytical
examples are not a tested MIDI implementation. Format 2/SMPTE support remains a
separate scope decision. No full freeze or Phase 2 work is authorized.

## Approved follow-up: D2-D4 input rules

On 2026-11-01 the owner approved adopting only D2-D4. This follow-up writes the
rules into the normative specification and Schema descriptions, promotes the
original raw probes into asserted tests, and adds full field/object coverage.
D1 and D5-D8 remained pending at that milestone. No production Rust change, new public parser API,
dependency change, instrument mapping change, or Phase 2 work was needed.

- D2: all five integer fields require plain unsigned integer tokens, followed by
  their existing value checks. Decimal/exponent forms, negative zero, and numeric
  strings are rejected; no coercion is introduced.
- D3: every object's decoded member names must be unique, even for equal/null
  values. Escape-equivalent names count as duplicates. Comparison is case-sensitive,
  with no extra Unicode normalization. Direct input must be checked before any
  generic object representation can discard duplicate information.
- D4: tempo decodes as finite positive binary64, including positive subnormals.
  Ordinary rounding is allowed; overflow, underflow-to-zero and nonpositive values
  are rejected. No arbitrary musical BPM range is added to fit a test tool.

The public contract is now explicit for consumers that used only Schema or
last-value-wins/high-precision parsers; this can require additional input checks.
Existing direct Rust behavior and field shapes/ranges are preserved. Schema
changes are annotations only, not assertions pretending to recognize original
numeric tokens or discarded members.

`input_rules.rs` adds five tests covering every integer field, all seven object
levels, same/different repeated values, escaped names, optional null text, and
tempo bounds. The minimum positive subnormal (`5e-324`, `f64::from_bits(1)`) and
maximum finite `f64` both parse, validate and round-trip. The earlier 15 raw
probes now have `rust_error` expectations and no ignored-test marker. Generic
Value-first outcomes are printed only to show why that path cannot certify raw input.

Verification observed:

| Command/check | Result |
|---|---|
| `cargo test -p skysheet-score --test input_rules --locked` | All 5 new tests passed against the existing reader, before any production change (none was needed). |
| `cargo fmt --all` and `cargo fmt --all -- --check` | Passed. |
| `cargo test --workspace --locked` | 43 tests passed; 0 failed, 0 ignored, 0 doc-tests. Includes the 15-case raw-input regression. |
| `cargo test -p skysheet-score --release --test input_rules --test raw_input_audit --locked` | All 6 tests passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0. |
| `pwsh -NoProfile -File scripts/test-score-schema.ps1 -AuditRaw` | Example and 103 structural fixtures passed. Raw observations still expose the documented numeric-tool limitations; they are not full-contract assertions. |
| Final scope comparison and `git diff --check` | Passed. Production sources, example, dependencies/lockfile, mapping and existing structural cases match the pre-task snapshot. After removing descriptions/comments, the Schema is identical; its assertions did not change. |

The PowerShell tool still rejects allowed extreme tempo cases; it remains a
structural fixture checker, not a full-range file validator. Independent numeric
validation strategy evaluation remains outstanding and is not silently solved by
these tests. Existing Cargo incremental-cache notes were non-fatal. Hosted CI was
not run; its existing Cargo command will execute the newly active tests when run.

## Approved follow-up: D1, D5, D7, D8 and partial D6

On 2026-11-01 the owner approved preservation of pitch strings and string-based
key identity, the exact-version/major-minor policy, embedded profiles without a
profile-version field, and provenance outside v1. The owner confirmed that no
external consumers currently depend on draft 1.0. For D6, only flattening with a
loss diagnostic and default short-note quantization errors were approved. Source
and reference tempo, rounding, numeric-failure and meter-change details remain
pending; no MIDI implementation or real-file validation was performed.

Scope: synchronized the decision record, normative format specification,
conversion guidance, Schema descriptions, README files and plan. No production
Rust source, accepted input set, format version, field shape/range, instrument
mapping, example, dependency, CI command or Schema assertion was changed.

Four new tests in `serialization_compatibility.rs` cover decoded spelling
preservation (including escaped strings, leading/negative octave zero and
enharmonic boundary pitches), distinct custom same-sounding keys and derived key
numbers through serialization, array/reference preservation, exact version
rejection after parsing, and absent/null versus empty text semantics. Seventeen
shared conformance cases cover accepted aliases/references, exact membership and
duplicate rules, rejection of unsupported spellings/version encodings, unchanged
Sky profile constraints and rejection of profile-version/provenance fields.
The prior 103 shared cases are retained; there are now 120. New tests passed
against the existing production code without any behavioral repair.

Verification observed:

| Command/check | Result |
|---|---|
| `cargo test -p skysheet-score --test serialization_compatibility --test contract_boundaries --locked` | 4 new tests and 11 shared-suite tests passed. |
| `cargo fmt --all -- --check` | Passed; no formatting rewrite was needed. |
| `cargo test --workspace --locked` | 47 tests passed; 0 failed, 0 ignored, 0 doc-tests. |
| `cargo test -p skysheet-score --release --test serialization_compatibility --test contract_boundaries --locked` | All 15 integration tests passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0. |
| `pwsh -NoProfile -File scripts/test-score-schema.ps1` | Example and 120 shared structural cases passed. |
| Pre-task snapshot comparison and inline example check | Production sources, dependencies/lockfile, example, CI and Schema script unchanged. Schema assertions are identical after removing annotations; original fixtures retained; inline example matches the fixture. |

The process PATH was extended to the already installed Cargo directory. Existing
incremental-cache access-denied notes were non-fatal. The Schema tool's numeric
limitations remain unresolved; no new full-range conformance claim is made.
Hosted CI was not run. Calibration, remaining D6 decisions and independent numeric
verification still prevent full freeze. No commit, push or Phase 2 work occurred.

## Approved follow-up: D6-A through D6-F (documentation only)

On 2026-11-01 the owner approved all six remaining timing-policy recommendations
and requested documentation, boundary examples and consistency checks only.
D6 is now approved for PPQ-based SMF Format 0/1. Format 2/SMPTE support remains a
separate decision. Policy approval is not implementation, full MIDI coverage or
real-file verification, and does not complete the contract freeze gate.

The normative policy is centralized in format-specification section 16. Decision,
pipeline, README and plan status now distinguish current approval from historical
partial approval. The policy records explicit-or-initial reference tempo; source
defaults and diagnosed cross-track merging; same-tick conflict errors even within
one track; absolute endpoint rounding with half ticks upward; reliable arithmetic
and whole-conversion failure; initial nominal meter and diagnosed loss of changes.
The earlier error-distribution wording is clarified to forbid additional shifting
after absolute-position rounding. No new BPM bounds or JSON fields are introduced.

The analytical examples cover late/default/explicit source tempo, target changes,
held notes crossing a tempo boundary, conflicts, leading silence, simultaneous
positions, half ticks, collapsed/noncollapsed sub-tick notes, rounded tick limits,
the binary64 `M + 0.5` pitfall, nominal and discarded out-of-range meters, and
excluded formats. They are not MIDI fixtures or an implemented quantizer test suite.

Verification observed:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no source formatting changes. |
| `cargo test -p skysheet-score --test contract_boundaries --test input_rules --locked` | 16 existing tests passed; no tests were added or modified. |
| `pwsh -NoProfile -File scripts/test-score-schema.ps1` | Existing example and all 120 shared structural cases passed. |
| Documentation consistency and arithmetic | 52 relative links/anchors resolved; JSON syntax and inline example equality passed. Temporary exact-arithmetic checks confirmed the documented elapsed-time, rounding, upper-bound and error-bound examples, without writing implementation/test files. |
| Pre-task snapshot comparison | Only eight Markdown files changed. Production sources, tests/fixtures, Schema, example, dependencies/lockfile, CI and scripts are byte-for-byte unchanged. |

Existing Cargo incremental-cache access-denied notes were non-fatal. Full workspace,
release and Clippy checks were not rerun for this documentation-only task. Hosted
CI and real MIDI checks were not run. Independent score-numeric verification and
acoustic calibration remain open; numeric implementation and actual D6 acceptance
tests require separately authorized work. No importer/quantizer, dependencies,
report schema, test algorithm, Phase 2 entry, commit or push was introduced.

## Approved follow-up: N1-N4 independent numeric verification

The owner approved the Python/jsonschema development-only strategy, nearest/even
binary64 decoding and bit-preserving tempo round trips, a strict independent CI
gate, and reporting differences before changing the Rust parser. These are now
recorded in [numeric-verification.md](../../docs/numeric-verification.md), which
contains exact commands, scope, minimal reproductions and compatibility analysis.

Implementation adds a raw-token/decoded-member checker, exact Decimal Schema
view with an integer-type adapter, integer/rational binary64 oracle, a development
Rust JSONL observation bridge, 114 original-input fixtures and 33 generated exact
midpoint-neighbor cases. It reuses the 120 existing structural cases and their
distinct Rust expectations. A fixed-seed sample covers 10,000 distinct positive
finite bit patterns and all finite exponent fields. The independent checker is
not a duplicate of all score semantic validation and is not a MIDI implementation.

Development/CI packages are pinned with wheel hashes for CPython 3.13 on Windows
x64 and Linux x86_64. Installation was isolated under ignored `target/`; production
dependencies, Cargo.lock and serde_json features were not changed. The existing
CI numeric/Schema step is replaced by the strict independent gate; PowerShell
remains optional diagnostic tooling. CI has no known-failure bypass and will fail
on the current differences. Hosted CI has not been run.

Observed verification:

| Check | Result |
|---|---|
| Locked Windows install and `pip check` | Passed, with all five pinned packages and no optional extras. |
| Hash-checked CPython 3.13 Linux wheel download | Passed for the same lock; this is not execution on Linux. |
| Independent checker self-tests | 15 passed, including deliberate false observations and exact mathematical boundary controls. |
| Independent fixed corpus | All 267 cases passed (120 shared + 114 original text + 33 exact boundary cases). |
| Full Rust comparison, seed 5458777 | Failed as required: 2,981 case records across 267 fixed cases and 10,000 samples; no expectations relaxed. |
| Minimal acceptance, midpoint and round-trip reproductions | Each returns gate exit 1 with the documented difference. |
| Tool-error smoke check | Unknown case returns exit 2, not invalid-data/pass. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo test --workspace --locked` | Existing 47 tests passed; this does not override the new numeric gate failure. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed, including the observation example. |
| Scope comparison with pre-task snapshot | Production sources, Cargo manifests/lock, previous tests, example, audio and mapping unchanged. Schema differs only in the tempo description. |

The observed differences include rejection of `1.7976931348623158e308`, wrong-side
midpoint rounding and non-stable save/read bits. Per `from_str` path there are 3
acceptance, 2,978 decode-bit and 988 round-trip differences; categories overlap.
The reader path has the same counts. The 2,981 cases comprise 10 fixed and 2,971
sampled cases, not a real-world file failure-rate estimate.

N4 stop boundary: report the concrete findings and compatibility impact; obtain
separate approval before evaluating/adopting parser-feature or behavior changes.
No `float_roundtrip`/`arbitrary_precision` feature, production parser fix, one-ULP
tolerance or skip list was added. Existing non-fatal Cargo cache notes remain.
The numeric tool exists but conformance is unresolved; Phase 1 is not complete.
No Phase 2 work, commit or push was performed.

## Rust-only verification migration

The owner approved the migration plan after requiring an explicit project stack:
Rust preferred/core; Go/C++ only after owner approval; Node.js/Bun when necessary
for auxiliary/frontend work; Python prohibited in all project contexts. AGENTS.md
was updated before implementation. N1's implementation choice is superseded, while
the independence goal, N2 precision/round-trip requirements and N3/N4 discipline
are unchanged.

The new tool is a separately locked Rust workspace, not linked to the score crate.
It uses an exact interval-search Oracle with independently checked rounding-cell
certificates, a raw JSON scanner, an exact-number Rust Schema engine, and the
unchanged root observation reader in a separate process/target. This separation
prevents the Schema engine's serde_json features from unifying into the DUT.
Root feature inspection confirms the original default-feature path.

All 120 shared and 114 raw-input fixtures are unchanged. The 33 exact midpoint
cases and 10,000 historical bit samples are now frozen with checksums. Rust sample
recovery follows the historical seed/word/rejection stream, verified by published
vectors, complete regeneration equality and all historical failure IDs/order.
Seven original reports are byte-preserved with SHA-256 in
`harness/verification/numeric-before-rust/`; historical actual results never
replace approved expected values. No old runtime was executed for migration.

Actual local results (Windows, Rust 1.99.1):

| Command/check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed. |
| `cargo test --workspace` | 47 passed, none ignored. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed. |
| Tool workspace fmt check | Passed. |
| `cargo test --manifest-path tools/numeric-validation/Cargo.toml --workspace --locked` | 19 passed, including 10,000 exact-value and 1,000 midpoint-cell checks. |
| Tool workspace Clippy, all targets, warnings denied | Passed. |
| Rust independent-only gate | All 267 fixed expectations passed. |
| Full Rust numeric gate | Exit 1: 2,981 failing cases across 267 fixed + 10,000 samples; strict expectations retained. |
| Migration comparison | Ordered failure IDs/checks/expected and actual bits/acceptance states all match the archive; error wording/offsets excluded. |
| Three minimal reproductions | Each still exits 1 with its original acceptance, midpoint or round-trip difference. |
| Evidence/corpus checksums and root source/Schema comparison | Passed; production code, root Cargo manifests/lock, original fixtures, Schema, example and mapping unchanged. |
| Hosted GitHub Actions | Not executed: new workflow is uncommitted/unpushed. Local commands do not count as a hosted run or Linux execution. |

Tracked interpreter scripts, dependency lock and installer/CI steps were removed.
The obsolete auxiliary Schema script was also removed; all current validation
commands use Cargo. Remaining textual references are prohibition rules, algorithm
provenance or explicitly historical evidence. New Cargo dependencies do not enable
interpreter bindings or remote resolver defaults. Existing incremental-cache
access-denied notes were non-fatal.

Cleanup limitation: automatic approval review rejected the command to remove
the ignored local `target/numeric-venv` and `target/numeric-linux-wheels` directories,
returning only `blocked by policy`. They remain unused, with no current tool or CI
reference invoking them. Their deletion is pending; no alternate filesystem API
was used to bypass the rejection. System-wide installations were untouched.

The migration has not corrected the existing parser discrepancies or made Phase 1
complete. No parser feature, public format, tolerance or expectation was changed.
No MIDI implementation, Phase 2 work, commit or push occurred.
