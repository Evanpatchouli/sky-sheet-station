# Phase 1 N2: RawValue tempo correction

**Status: production correction implemented; Windows acceptance passes; Linux
acceptance pending. N2 is not marked complete.** No Phase 2 work is included.

## Preflight and authorization

On 2026-11-01 the owner explicitly approved the field-level RawValue + Rust
standard-library conversion and promotion of the existing serde_json dependency.
The task began on branch `main`, HEAD
`36a507377f8c94d6941282cefb0bbc4a5dcdcf3d`, with a clean working tree.
There were no user modifications to overwrite. Both lockfiles, original score
source, Oracle, runner, validation and frozen numeric fixtures matched the prior
research snapshot. Its reports showed zero differences for 267 fixed + 10,000
historical cases and the separate 3,016 supplemental cases using the prototype.

The production adapter follows that prototype. The only additional error-handling
detail is explicit `timeline.tempo` context on text conversion failure. No
architecture or format decision was silently changed.

## Files and purpose

| File | Change |
|---|---|
| `Cargo.toml` | Enable only `raw_value` alongside serde_json's existing default features. |
| `crates/skysheet-score/Cargo.toml` | Promote existing workspace serde_json from dev-dependency to normal dependency. |
| `crates/skysheet-score/src/lib.rs` | Add `deserialize_with` on tempo and a small JSON-specific adapter. |
| `crates/skysheet-score/examples/numeric_probe.rs` | Report decoded bits on semantic rejection, without any expected results. |
| `crates/skysheet-score/tests/n2_tempo.rs` | Four regressions covering exact bits, long boundaries, repeated save/read and invalid JSON/non-number inputs. |
| `crates/skysheet-score/tests/fixtures/n2-tempo.json` | 84 new, separately identified focused cases with provenance and frozen Oracle expectations. |
| `tools/numeric-validation/tests/n2_regressions.rs` | Reproduce the 3,016 research cases, certify all focused expectations, compare both DUT paths using the unchanged comparator, and retain per-run reports. |
| `.github/workflows/rust.yml` | Retain supplemental reports in the existing always-upload artifact step; ordinary tool tests already execute the added suite. |
| `.gitattributes` | Preserve bytes of the new evidence and focused fixture across checkouts. Existing rules remain. |
| `docs/numeric-verification.md` | Document the current decoder, allocations, compatibility, separate evidence and pending Linux verification. |
| `docs/music-data-format.md` | Replace the current-reader gap description with actual implementation and validation status; normative fields/rules unchanged. |
| `docs/README.md`, `PLANS.md` | Reflect Windows success and pending Linux acceptance; leave N2/Phase 1 incomplete. |
| `harness/verification/n2-raw-value-20261101/` | Add new reports and hashes without touching any old reports. `focused-80.json` is the intermediate run before adding four long-exponent/integer cases; `focused-84.json` is the final focused run. |
| This record | Implementation, command results, compatibility, remaining work and Git disposition. |

## Core implementation

The public field remains `Timeline::tempo: f64`:

```rust
#[serde(deserialize_with = "deserialize_tempo")]
pub tempo: f64,
```

```rust
fn deserialize_tempo<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let raw = Box::<serde_json::value::RawValue>::deserialize(deserializer)?;
    let value = raw.get().parse::<f64>().map_err(|error| {
        serde::de::Error::custom(format!("timeline.tempo: expected a JSON number: {error}"))
    })?;
    if !value.is_finite() {
        return Err(serde::de::Error::custom(
            "timeline.tempo: number out of range",
        ));
    }
    Ok(value)
}
```

RawValue preserves the complete token after serde_json checks JSON grammar.
There is no intermediate Number/Value, truncation, new length limit or approximate
scaling. Rust's converter rounds the decimal to binary64. Nonfinite results are
parse errors with the required context; zero and negative results reach the
unchanged `Score::validate()`. Strings/non-numbers and nonstandard numeric syntax
cannot become valid tempo values. Serialization, integers, pitches, instrument
mapping, timeline structure and format version 1.0 are unchanged.

The adapter is JSON-specific; arbitrary non-JSON Serde compatibility is not
promised. Original JSON must be read directly. A prior Value conversion cannot
restore discarded digits or duplicate members. Error category/position can change
because custom errors differ from serde_json's built-in errors; the existing
diagnostic regressions pass, and no expected diagnostic was relaxed.

## Numeric evidence

The original Oracle, `runner::compare`, feature prohibition checks, 267 fixed
cases, 10,000 historical samples and archived failures are byte-preserved.
The historical gate remains a separate run with its original counts. No DUT
result generates an expected value. The new fixture's expected bits are checked
against the old Oracle; exact zero cells and IEEE sign bits distinguish invalid
zero/negative values from overflow. The bridge now exposes bits before semantic
rejection, so a zero-boundary test cannot pass merely because some error occurred.

The added 3,016 suite reproduces the research construction exactly: 3,000 midpoint
cell values from every tenth historical sample, plus 16 long midpoint spellings.
An asserted digest freezes IDs, tokens and order. The 84 focused cases additionally
cover all 33 existing exact midpoint constructions, 30 long zero/one midpoint
spellings (below/tie/above at 0, 714, 715, 800 and 1,200 appended zeros), 17 literal
boundary/sign/exponent cases and four long exponent/integer cases. They include
the exact requested 715-zero coefficient and `2^-1075` with both neighbors.

| Scope | Required | Observed | Result |
|---|---|---|---|
| Original fixed corpus | 267, zero differences | 267, 0 failures | Pass |
| Original historical samples | 10,000, zero differences | 10,000, 0 failures | Pass |
| Research supplement | 3,016, zero differences | 3,016, 0 failures | Pass |
| Focused fixture | Oracle-certified exact bits/status | 84, 0 failures | Pass |
| Three historical minimal reproductions | Correct boundary, tie and round-trip | Each rerun separately, 0 failures | Pass |
| Midpoint regressions | Strict `to_bits()` equality | 714/715 zeros, neighbors and longer spellings agree | Pass |
| Zero boundary | Exact zero or minimal subnormal as applicable | Correct bits before semantic validation | Pass |
| `from_str` | All acceptance/bit comparisons pass | Historical + both added groups pass | Pass |
| `from_reader` | Same as string path | Historical + both added groups pass | Pass |
| Serialization Oracle | Saved decimal denotes accepted bits | All accepted cases pass | Pass |
| Save/read | Identical binary64 bits | All accepted cases pass; root regression also checks pretty JSON and three cycles | Pass |

The research report records **per input path**: 3,016 acceptance checks, 3,016
decode-bit checks, 3,014 serialization-Oracle checks and 3,014 readbacks. Two zero
ties are correctly invalid and are not required to serialize as valid scores.
The final focused report records per path: 84 acceptance checks, 79 finite
decoded-bit checks, 60 serialization-Oracle checks and 60 readbacks. Five overflow
cases have no finite decoded value. Invalid zero/negative cases are rejected by
semantic validation after exact decoding. These counts and the empty failure
arrays were inspected, not inferred solely from process exit codes.

Preserved new reports:

- [Historical corpus](../verification/n2-raw-value-20261101/historical-after.json)
- [3,016 research cases](../verification/n2-raw-value-20261101/research-3016.json)
- [84 focused cases](../verification/n2-raw-value-20261101/focused-84.json)
- [Overflow reproduction](../verification/n2-raw-value-20261101/repro-overflow.json)
- [Tie reproduction](../verification/n2-raw-value-20261101/repro-tie.json)
- [Round-trip reproduction](../verification/n2-raw-value-20261101/repro-roundtrip.json)
- [Evidence and protected-file hashes](../verification/n2-raw-value-20261101/manifest.json)

`baseline_equivalence.matches` is false by design: new results differ from old
failures. That informational audit never changes the gate's expected outcomes.

## Engineering checks actually executed

Environment: Windows x86_64 MSVC, rustc 1.99.1 (`58cf8f9c3`), Cargo 1.99.1,
serde_json 1.0.154. Cargo's existing bin directory was added to process PATH only.

| Command | Observed result |
|---|---|
| `cargo test -p skysheet-score --locked` | 51 passed: 19 unit + 32 integration; no ignored tests. |
| `cargo test -p skysheet-score --release --locked` | Same 51 passed. |
| `cargo test --workspace --locked` | 51 passed. |
| `cargo test --manifest-path tools/numeric-validation/Cargo.toml --workspace --locked -- --nocapture` | 20 passed: 19 unchanged self-tests + new integrated regression test. |
| `cargo fmt --all -- --check` | Exit 0. |
| `cargo fmt --manifest-path tools/numeric-validation/Cargo.toml --all -- --check` | Exit 0. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0. |
| `cargo clippy -p skysheet-score --all-targets --locked -- -D warnings` | Exit 0. |
| `cargo clippy --manifest-path tools/numeric-validation/Cargo.toml --workspace --all-targets --locked -- -D warnings` | Exit 0. |
| `cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked -- --report target/n2-fix-20261101/historical-after.json` | 267 + 10,000; exit 0, zero differences. |
| Three documented `--case` runs with distinct new `--report` paths | Each exit 0; reports inspected. |
| Existing feature isolation check | Runs inside the unchanged runner; raw_value present, float_roundtrip/arbitrary_precision absent; preserve_order policy unchanged. |
| `git diff --check` and protected-file hashes | Passed; old evidence/Oracle/runner/corpora intact. |

Both lockfiles remain byte-identical, without manual editing:

- Root: `e102df124fbccec05c9f9a78f3ce584c761aa0aef0bf0bb93ada918bfb711249`.
- Tool: `57fbe7147a2ee626d13736654f1f52cee0ee1c312b6eba819fcc9850518a8ad6`.

No dependency version or package was added. Promoting serde_json into normal
dependencies is still a real production dependency change, explicitly approved
by the owner; its previous presence in the lockfile did not substitute for that
manifest change. No numeric algorithm dependency was introduced.

## Linux, performance and remaining acceptance

Existing [Linux CI run 41253767247](https://github.com/Evanpatchouli/sky-sheet-station/actions/runs/41253767247)
at the pre-repair HEAD completed with failure. Root/tool fmt, tests and Clippy
passed there, while the numeric gate reported the original 2,981 differences.
Its logs were inspected. This is baseline evidence, not validation of the new code.

Local WSL Ubuntu 26.04 is available, but Cargo, rustc and a C compiler/linker were
not found. Docker could not connect to its Linux engine. No Linux test execution
is claimed, and no global toolchain installation was performed. The repair is
uncommitted/unpushed, so hosted CI cannot validate it yet. N2 remains pending
Linux verification; the roadmap checkbox stays unchecked. This task does not
freeze Phase 1 or authorize MIDI work.

Source inspection of serde_json 1.0.154 `raw.rs`/`read.rs` confirms a temporary
owned token allocation per tempo: string input copies the slice; reader input
builds a byte buffer then transfers its string into Box<RawValue>, possibly
reallocating while reducing capacity. Space is O(token length), released after
conversion; the stored field stays an f64. No exact allocator-count benchmark
or timing benchmark was run. No whole-Score speed percentage or general
cross-platform correctness result is claimed.

No unresolved numerical difference is observed in the requested Windows suites.
Remaining acceptance is Linux execution of this revision. The documented
JSON-specific adapter and error category/location differences are compatibility
boundaries, not changes to public JSON fields or musical semantics.

## Git disposition

Branch and HEAD remain `main` / `36a507377f8c94d6941282cefb0bbc4a5dcdcf3d`.
Only the files listed above are modified or new, all unstaged. No commit, push,
merge, release, cleanup of historical evidence or unrelated refactor occurred.
