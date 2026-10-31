# Independent numeric verification (Rust only)

**Migration implemented; N2 conformance still fails.** The project technology
stack prohibits Python in production, development, testing, CI, validation, builds
and automation. This supersedes N1's former implementation choice, not its
independent-verification goal. N2-N4 expectations are unchanged. No MIDI work or
production-reader fix is included.

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
3. The root `numeric_probe` observes unchanged `Score` parsing/semantic validation,
   serialization and readback through both string and reader paths. It contains
   no expected outcomes or Oracle.
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
with the root workspace's existing default-feature reader. The tool does not link
`skysheet-score`: it builds the root example in a separate Cargo process/target
and exchanges JSONL. It checks the root feature tree and errors if either feature
is enabled there without a later N4 change. Tool-side parsing serves Schema and
protocols, never the Oracle. Root manifests/lock, production sources, public JSON,
Schema assertions and logical instrument mappings are unchanged.

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

## Minimal reproductions and current status

```text
cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked -- --case "tempo 1.7976931348623158e308" --report target/repro-overflow.json
cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked -- --case "midpoint/3ff0000000000000/tie" --report target/repro-tie.json
cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked -- --case "sample/00167d447773ebd6" --report target/repro-roundtrip.json
```

| Case | Unchanged N2 expectation | Existing root reader |
|---|---|---|
| `1.7976931348623158e308` | Accept, bits `7fefffffffffffff`. | Rejects as out of range. |
| `1.00000000000000011102230246251565404236316680908203125` | Tie rounds to `3ff0000000000000`. | Produces `3ff0000000000001`. |
| In-memory `00167d447773ebd6` | Preserve bits through save/read. | Text `3.127525706621981e-308` reads as `00167d447773ebd7`; another save/read returns `00167d447773ebd6`. |

The migrated run still fails on 2,981 cases (10 fixed, 2,971 sampled). Counting one
input path, there are 3 acceptance, 2,978 decode-bit and 988 readback differences;
categories overlap. Both input paths match archived observations. This is not a
failure-rate estimate for user scores. No tolerance, ignored case or allow-list
was added. Phase 1 remains incomplete.

A parser correction could broaden accepted boundary literals, change adjacent
decoded values and stabilize round trips, affecting equality, saved output and
later timing calculations. Migration changes none of these behaviors. Parser
feature evaluation remains a separately authorized N4 task.

Local execution results are recorded in the migration audit. Hosted GitHub Actions
cannot run the new workflow until its changes are uploaded under separate
commit/push authorization. Local checks are not reported as hosted CI success.
No MIDI implementation or Phase 2 entry is authorized.

Cleanup limitation: automatic approval review rejected removal of the ignored
project-local `target/numeric-venv/` and `target/numeric-linux-wheels/` directories
with only "blocked by policy" as its reason. They remain unused on this machine;
no source, Cargo command or CI step invokes them. Tracked scripts, runtime lock
and installer configuration have been removed. Their deletion remains pending;
the restriction was not bypassed through another filesystem API.

## References

- [Rust jsonschema exact-number support](https://docs.rs/jsonschema/0.60.2/jsonschema/): the structural engine, not the Oracle.
- Historical sample provenance: [integer seed and word generation](https://github.com/python/cpython/blob/v3.13.14/Modules/_randommodule.c) and [published repeatability vectors](https://github.com/python/cpython/blob/v3.13.14/Lib/test/test_random.py). These are references, not executable project dependencies.
