# Independent numeric verification (Rust only)

**N2 RawValue repair acceptance complete: Windows checks and Linux CI pass.** The project technology
stack prohibits Python in production, development, testing, CI, validation, builds
and automation. This supersedes N1's former implementation choice, not its
independent-verification goal. N2-N4 expectations are unchanged. The owner-approved
production-reader fix is described below; no MIDI work is included. Linux CI for
`5cee8ef` completes this repair's acceptance, not the remaining Phase 1 freeze checks.

## Rules and independent paths

| Decision | Current implementation |
|---|---|
| N1 | Isolated Rust validation workspace, locked Rust dependencies and exact-number Oracle; no additional runtime. |
| N2 | Nearest binary64, ties to even, finite positive tempo, and bit-preserving save/read. |
| N3 | Strict numeric CI gate, with no tolerance, ignore list or continue-on-error. |
| N4 | Report differences before changing the observed parser's features or behavior. |

Four paths remain separate:

1. `raw_json.rs` scans original JSON grammar, preserves numeric token slices and
   positions, and detects repeated decoded names per object before building a
   map. It distinguishes `1`, `1.0`, `1e0`, `-0`, booleans and strings. Names are
   case-sensitive without extra Unicode normalization. Tool-side string unescaping
   handles escaped names; numeric tokens never pass through f64.
2. `validation.rs` applies the existing Draft 2020-12 Schema through Rust
   `jsonschema` with exact-number support. D2/D3 are not attributed to Schema.
   External references are rejected before compilation and resolver defaults are
   disabled. Original number tokens are retained in the Schema-only representation.
3. The root `numeric_probe` observes production `Score` parsing/semantic validation,
   serialization and readback through both string and reader paths. It contains
   no expected outcomes or Oracle. It also reports decoded tempo bits when semantic
   validation fails, allowing zero-boundary tests to distinguish correct zero
   decoding from rejection for some unrelated reason.
4. `oracle.rs` computes expectations from original decimals using integer ratios
   and binary64 rounding intervals. It never calls a floating-point text parser,
   Schema validator or production reader to obtain an expected value.

The independent `valid` result covers raw input, Schema and numeric rules, not
all score semantics such as pitch membership or cross-track ID uniqueness.
Structural and full Rust expectations remain separate for the 120 shared cases.
Syntax/duplicate-member failures have no Schema result (`null`). Derived note ends
are checked independently of individual field bounds.

## Build and dependency isolation

`tools/numeric-validation/` is a separate Cargo workspace, with its own Cargo.lock.
Direct dependencies are jsonschema 0.60.2, num-bigint 0.4.8, num-traits 0.2.19,
serde 1.0.229, serde_json 1.0.154 and sha2 0.11.1. These are verification dependencies,
not additions to the score crate. No interpreter, native numeric library or shell
validation script is required. Language bindings and remote resolvers are disabled.

Isolation is essential: the Schema crate enables serde_json `float_roundtrip`;
exact numbers also require `arbitrary_precision`. Those features must not unify
with the root workspace's reader (default features plus `raw_value`). The tool does not link
`skysheet-score`: it builds the root example in a separate Cargo process/target
and exchanges JSONL. It checks the root feature tree and errors if either feature
is enabled there. These prohibitions and `runner::compare` are unchanged by the
repair. Tool-side parsing serves Schema and protocols, never the Oracle. The
approved root manifest enables `raw_value`, and the score crate promotes its
existing serde_json dependency from development to normal dependencies. Both
lockfiles, public JSON, Schema assertions and logical instrument mappings remain
unchanged; no third-party numeric conversion package is added.

## Approved tempo reader correction

`Timeline::tempo` remains `f64`, with a field-level `deserialize_with` adapter.
It captures `Box<serde_json::value::RawValue>`, calls `get().parse::<f64>()` on
the complete original text, and rejects nonfinite results with an error containing
`timeline.tempo` and `number out of range`. Conversion errors also identify the
field. `Score::validate()` is unchanged and continues to reject zero, negative
values and in-memory nonfinite values. JSON grammar is still checked by serde_json;
strings, booleans, arrays, objects, NaN and Infinity are not coerced into numbers.
There is no truncation, approximation, new token-length limit or tolerance.

This field adapter is JSON-specific. Arbitrary non-JSON Serde deserializers are
not promised compatible. Read original JSON directly; a prior `Value` conversion
cannot restore lost digits or duplicate members. Serialization still uses the
existing `f64` representation. Custom parse errors can differ in category/location
from serde_json's built-in errors, while preserving the required diagnostics.

`float_roundtrip` is deliberately not enabled. In serde_json 1.0.154 it fixes all
historical cases, but its slow path treats a remaining all-zero integer suffix
as nonzero after retaining 768 digits. The exact midpoint `1 + 2^-53`, written as
the coefficient `100000000000000011102230246251565404236316680908203125`, followed
by 715 zeros and `e-768`, incorrectly rounds to `3ff0000000000001` rather than
`3ff0000000000000`. Equivalent long spellings of `2^-1075` can incorrectly become
the smallest positive subnormal. This is not permission to constrain input length.
See serde_json's [versioned slow-path source](https://github.com/serde-rs/json/blob/v1.0.154/src/lexical/bhcomp.rs)
and Rust's [versioned conversion implementation](https://github.com/rust-lang/rust/blob/1.99.1/library/core/src/num/imp/dec2flt/mod.rs).

Allocation review: `Box<RawValue>` owns the tempo token. String input copies the
borrowed token into owned storage; reader input accumulates a byte buffer and
transfers its string into the box, potentially reallocating when shrinking its
capacity. Temporary heap space is O(token length), freed after conversion. The
score stores only the resulting `f64`. This is a source inspection, not an
allocation-count or throughput benchmark; no whole-Score percentage is claimed.

## Oracle design and correctness argument

The new Oracle uses exact interval search, not the former exponent-normalization
and quotient-rounding implementation:

- Convert the decimal coefficient and scale into an exact ratio `n/d`.
- Positive finite binary64 bit patterns are monotone in their real values. Decode
  candidates into exact dyadics using integer significands and powers of two.
- Binary search maintains the invariant that the greatest finite value at or below
  `n/d` lies within the inclusive candidate interval. Exact cross-products shrink
  that interval until the two surrounding representable values are identified.
- Compare with their exact midpoint. Select the closer neighbor; equality selects
  the even significand (low bit zero).
- At `2^-1075`, the tie rounds to zero and fails positivity. At
  `2^1024 - 2^970`, rounding exceeds the finite range. Values below the latter
  threshold may round to the maximum finite value even if the decimal exceeds it.
  The normal/subnormal transition uses the same midpoint rule.
- A separate certificate checks membership in the proposed result's rounding
  interval. It performs no search and uses neither a host float decoder nor the
  DUT. Interval endpoints are included only when the significand is even. Wrong
  neighbors must fail certification.

Tests substantiate these invariants without claiming exhaustive proof: known bit
patterns, exact powers of two, normal/subnormal/overflow endpoints, even/odd ties,
all frozen midpoints, deliberate incorrect observations, 10,000 exact representable
values and 1,000 sampled adjacent midpoint cells (tie and both sides). Expected
values come from selected bits and parity, not parsed floating-point text. The
mathematical interval argument, not mere agreement with another implementation,
is the basis of the Oracle. D6's musical half-up quantizer is not implemented.

## Corpus and evidence preservation

All 267 fixed cases remain: 120 unchanged structural cases, 114 unchanged original
JSON cases, and 33 exact midpoint-neighbor cases frozen in `fixtures/midpoints.json`
under the tool directory. The 10,000 original samples are frozen, in order, as
hexadecimal bit patterns in `fixtures/samples.json`; default verification reads
this file, not a replacement RNG. The manifest records seed 5458777 (`0x534B59`),
counts and SHA-256 digests. Sample-file SHA-256:

`dd0bb76c6767194942a8810e062160443624525536d0c45b3fd451fd89c1b794`

The former system had no complete sample-list artifact. Rust recovery matches its
integer-seed MT19937 init-by-array, low-word-first bit assembly, rejection sampling
and insertion-order deduplication. Recovery is checked with published seed/word
and long-stream vectors, full regeneration equality, uniqueness, exponent coverage
and every historical failing sample ID/order. This is algorithm-based recovery,
not comparison with an unavailable old full-list hash. No historical interpreter
is executed. Ordinary runs need only the frozen manifest.

Seven original reports are byte-preserved in
`harness/verification/numeric-before-rust/`, with source commit, byte sizes and a
SHA-256 manifest. These include the complete 2,981-failure report and three minimal
reproductions. `.gitattributes` prevents checkout newline conversion from changing
hashed evidence/corpora. Old runtime metadata is historical evidence, not a setup
instruction or dependency.

The full Rust report includes a migration audit of ordered failure IDs, check
names, expected/actual bits and acceptance states. Incidental error wording/column
offsets are excluded. This audit is informational: approved expectations alone
determine conformance. Historical actual results never replace expected results.

Additional evidence is kept separate from this historical corpus:

- `tests/n2_regressions.rs` reproduces the research's 3,016 constructions: 1,000
  selected adjacent midpoint cells (below/tie/above), plus eight midpoint values
  written with 800 and 1,200 appended integer zeros. The ordered token digest is
  asserted as `015273ae1ef01adcb2ef4578b635e9d13d5b014a4f9e9d264d76323b85a87f60`.
- `crates/skysheet-score/tests/fixtures/n2-tempo.json` adds 84 focused cases with
  provenance and frozen expected bits. These include the historical problems,
  exact midpoint neighbors, 714/715-zero spellings, zero/overflow boundaries,
  normal/subnormal extremes, 1,200 leading exponent zeros and a 4,097-digit
  integer with a cancelling exponent. The isolated test rechecks all expectations
  with the unchanged Oracle; zero-cell/sign expectations additionally use exact
  dyadic constructions and IEEE sign bits, never a text-to-float decoder.
- The test invokes the unchanged runner to build the separate DUT and enforce
  feature isolation. It rejects duplicate/missing responses, calls the unchanged
  comparator and additionally checks bits before semantic rejection. Reports
  count acceptance, decode, serialization-Oracle and readback checks per input
  path. Invalid scores are not required to serialize as valid scores.
- Each supplementary run writes a unique directory under `target/n2-regressions/`.
  The original gate still contains exactly 267 fixed cases and 10,000 samples.
  New checks run in ordinary isolated workspace tests, including CI, whose artifact
  step now retains supplemental reports as well as the historical-corpus report.

## Commands and CI

From the repository root, using Rust and the target's native linker:

```text
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --manifest-path tools/numeric-validation/Cargo.toml --all -- --check
cargo test --manifest-path tools/numeric-validation/Cargo.toml --workspace --locked
cargo clippy --manifest-path tools/numeric-validation/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked --
```

For repair evidence, choose a fresh `--report` path rather than overwriting an
earlier local report. Run just the added suite with:

```text
cargo test -p skysheet-score --locked --test n2_tempo
cargo test --manifest-path tools/numeric-validation/Cargo.toml --locked --test n2_regressions -- --nocapture
```

Root checks do not cover the isolated tool. CI checks both workspaces and retains
the JSON report even when conformance fails. Initial Cargo downloads may need the
registry; validation uses local files. No additional language runtime is installed.

Exit codes remain 0 (requested checks pass), 1 (conformance differences), and 2
(tool/environment failure). `--independent-only` checks fixed independent
expectations without Rust observations; `--samples 0` omits sampling; `--case`
selects an exact ID, including `sample/<hex bits>`. Reduced runs do not replace
the full gate. `--report` overrides `target/numeric-conformance-report.json`.
Reports retain failing original JSON, tokens, locations, bit differences, root
feature tree, tool lock digest and migration audit. Scanner/Oracle resource limits
are tool errors, never new public JSON validity limits. No unbounded-input resource
guarantee is made.

## Historical minimal reproductions and current status

```text
cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked -- --case "tempo 1.7976931348623158e308" --report target/repro-overflow.json
cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked -- --case "midpoint/3ff0000000000000/tie" --report target/repro-tie.json
cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked -- --case "sample/00167d447773ebd6" --report target/repro-roundtrip.json
```

| Case | Unchanged N2 expectation | Pre-repair root reader (historical) |
|---|---|---|
| `1.7976931348623158e308` | Accept, bits `7fefffffffffffff`. | Rejects as out of range. |
| `1.00000000000000011102230246251565404236316680908203125` | Tie rounds to `3ff0000000000000`. | Produces `3ff0000000000001`. |
| In-memory `00167d447773ebd6` | Preserve bits through save/read. | Text `3.127525706621981e-308` reads as `00167d447773ebd7`; another save/read returns `00167d447773ebd6`. |

The preserved migrated run failed on 2,981 cases (10 fixed, 2,971 sampled). Counting one
input path, there are 3 acceptance, 2,978 decode-bit and 988 readback differences;
categories overlap. Both input paths match archived observations. This is not a
failure-rate estimate for user scores. No tolerance, ignored case or allow-list
was added. Phase 1 remains incomplete.

The approved correction broadens acceptance of boundary literals and corrects
adjacent decoded values, affecting equality and saved output as required by N2.
On Windows x86_64 MSVC with Rust/Cargo 1.99.1, all 267 fixed cases and 10,000
historical samples now pass, as do all 3,016 research cases, focused regressions
and all three separately rerun historical minimal reproductions. The historical
equivalence audit now reports `matches: false` because the old failures are fixed;
it is informational, not a weakened acceptance check.

The [repair record](../harness/build/phase-01-n2-raw-value.md) records commands,
counts and archived new reports. Root debug/release tests, isolated tests, fmt and
Clippy pass locally. Baseline Linux CI for `36a5073` failed on the old 2,981
differences. Following the owner-authorized commit/push, [Linux CI for `5cee8ef`](https://github.com/Evanpatchouli/sky-sheet-station/actions/runs/41256992799)
passed all steps on Ubuntu 26.04 with Rust 1.99.1. Downloaded artifacts confirm
267 fixed + 10,000 sampled cases, 3,016 research cases and 84 focused cases with
zero differences. The artifacts and hashes are separately archived under
`harness/verification/n2-raw-value-20261101/linux-5cee8ef/`; prior reports are intact.
N2 repair acceptance is complete. Local WSL was not used, performance benchmarks
remain unrun, and Linux release-mode tests are not a step in this workflow (release
tests passed on Windows). Phase 1 is not frozen; no Phase 2 entry is authorized.

Cleanup limitation: automatic approval review rejected removal of the ignored
project-local `target/numeric-venv/` and `target/numeric-linux-wheels/` directories
with only "blocked by policy" as its reason. They remain unused on this machine;
no source, Cargo command or CI step invokes them. Tracked scripts, runtime lock
and installer configuration have been removed. Their deletion remains pending;
the restriction was not bypassed through another filesystem API.

## References

- [Rust jsonschema exact-number support](https://docs.rs/jsonschema/0.60.2/jsonschema/): the structural engine, not the Oracle.
- Historical sample provenance: [integer seed and word generation](https://github.com/python/cpython/blob/v3.13.14/Modules/_randommodule.c) and [published repeatability vectors](https://github.com/python/cpython/blob/v3.13.14/Lib/test/test_random.py). These are references, not executable project dependencies.
