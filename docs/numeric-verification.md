# Independent score numeric verification

**N1-N4 approved, 2026-11-01. Tooling implemented; conformance is failing.**
The existing Rust reader has observed acceptance, rounding and round-trip
differences under the approved N2 rules. No parser feature or expected result has
been changed to hide them. Phase 1 is not complete; MIDI work remains unauthorized.

## Approved decisions and scope

| Decision | Rule |
|---|---|
| N1 | Independent Python standard-library numeric checks plus jsonschema 4.26.0, with version/hash-locked development and CI dependencies only. |
| N2 | Decode JSON decimals to the nearest binary64 value, ties to even; require finite positive tempo and bit-preserving serialization/readback of accepted tempos. This is not original decimal-text preservation. |
| N3 | The independent path is the required numeric CI gate. PowerShell Test-Json is auxiliary structural/diagnostic tooling, not a numeric authority. |
| N4 | Preserve strict expectations and report minimal reproductions and compatibility consequences of Rust differences before considering parser changes. |

N2 is separate from D6-C: decimal-to-binary64 ties are even; future musical tick
quantization ties go upward. Neither a MIDI quantizer nor an importer is built here.
The score still uses f64 tempo, existing integer fields/ranges, format 1.0 and the
same logical instrument mapping. The new precision requirement is documented as
an approved pre-freeze refinement; current implementation conformance is not claimed.

## Independent checks

`scripts/score_numerics.py` has three explicit layers:

1. Parse original JSON with number-token hooks and object-member pairs. Reject
   non-JSON constants and repeated decoded names before constructing dictionaries.
   D2 checks original tokens at the five integer-field locations, without numeric
   coercion, case folding or additional Unicode normalization.
2. Apply the existing Draft 2020-12 Schema to exact Decimal values. An integer
   type adapter recognizes integral Decimals and excludes booleans. Construction
   and comparisons do not use Decimal arithmetic rounding. External reference
   retrieval is disabled; the current Schema uses local references only.
3. Use Python integers for derived note ends, and exact rational/integer arithmetic
   for binary64 rounding. The oracle derives the exponent/significand and rounds
   the exact ratio to nearest/even, including subnormals and carry into infinity.
   The host Python float decoder is cross-checked against the oracle, not treated
   as the mathematical authority. Decimal context precision does not determine
   oracle results.

The returned `valid` status covers these layers only. It does not independently
check pitch parsing/membership, ID uniqueness or every Rust semantic constraint.
For the 120 existing structural cases, `schema_valid` and the existing full Rust
expectation are compared separately. Intentional Schema/semantic differences are
not treated as bugs in either tool. Original-input cases retain their JSON text
in `numeric-inputs.json`; malformed/duplicate-member inputs have no structural
Schema result (`null`), rather than a fabricated Schema failure.

`crates/skysheet-score/examples/numeric_probe.rs` is a development observation
bridge using the unchanged Score deserializer and current serde_json dev dependency.
It reads JSONL requests, checks both `from_str` and `from_reader`, calls semantic
validation, and reports parsed bits, serialized JSON and readback bits. It has no
oracle or expected outcomes and adds no production CLI/parser API. A bits request
also tests serialization starting from an in-memory f64. The Python comparison
gate, rather than the bridge's exit status, decides conformance.

## Reproducible environment and commands

The lock is scoped to CPython 3.13 on Windows x64 and Linux x86_64 for validation,
not a product-platform or Rust MSRV promise. It pins jsonschema 4.26.0, attrs
26.1.0, jsonschema-specifications 2025.9.1, referencing 0.37.0 and rpds-py 2026.9.1.
All required packages on Python 3.13 are locked with SHA-256 wheel digests; no
optional format extras or new production Rust dependency are used. Other Python
versions/environments need their own reviewed lock. Wheel installation can require
network access; validation uses local data and no remote Schema fetches.

From the repository root on Windows (Cargo must be on PATH):

```powershell
python -m venv target/numeric-venv
target/numeric-venv/Scripts/python.exe -m pip install --require-hashes --only-binary=:all: -r requirements/validation.lock
target/numeric-venv/Scripts/python.exe -B -m unittest discover -s scripts -p test_score_numerics.py
target/numeric-venv/Scripts/python.exe -B scripts/verify_score_numerics.py
```

On Linux use `python3 -m venv target/numeric-venv` and
`target/numeric-venv/bin/python` for the same commands. CI installs the lock in its
Python 3.13 environment, runs self-tests and the default gate, and retains the
generated report even on failure. Nothing uses `continue-on-error` or filters out
known differences. The gate checks installed versions against the lock.

Exit codes: **0** requested checks passed; **1** conformance/expectation differences;
**2** tool/environment failure. Missing dependencies are a tool error, not invalid
score data. `--independent-only` checks the independent fixed corpus without Rust;
its success does not mean the complete gate passed. `--case` isolates a named
case; `--samples 0` omits sampling for boundary iteration. Neither reduced run
replaces the default acceptance gate.

The default report is `target/numeric-conformance-report.json`; it contains the
seed, counts, runtime/package versions, original failing JSON, field/token context
and bit differences. It is generated output, not a checked-in acceptance baseline.
Parser syntax errors retain the parser's location; raw/numeric errors include a
JSON pointer. Resource/capability exceptions are tool errors, never implicit new
score-size/precision restrictions. The tool is a finite test harness, not a claim
to process arbitrary-size inputs within bounded resources.

## Acceptance corpus

- 120 unchanged shared structural cases with their separate Rust expectations.
- 114 original-text cases covering all five integer fields and seven object
  levels, duplicates including escaped names/nulls, malformed JSON numeric syntax,
  booleans/strings, integer/end bounds, extreme tempo and known expected bits.
- 33 exact midpoint-neighbor cases: zero/subnormal boundary, subnormal/normal
  boundary, ordinary exponents, ties with even/odd significands and overflow.
- 10,000 deterministic, distinct finite positive in-memory binary64 samples with
  seed `0x534B59` (5458777), covering every finite exponent field and selected edge
  significands. This is sampling, not exhaustive proof over all bit patterns.
- 15 checker self-tests, including known bit patterns, exact thresholds, Decimal
  context independence, adapter semantics, duplicate decoding, disabled remote
  references and deliberately wrong Rust observations that must be detected.

The full gate must have zero unexplained discrepancies and zero tool errors.
Source fields and original token forms are not narrowed to accommodate a validator.
Required Cargo tests, formatting, lint and documentation/Schema consistency must
also pass. Current discrepancies are not waived: resolving them requires the
separate N4 decision below. Passing this gate would not implement D6 or authorize
Phase 2; the remaining Phase 1 consistency review is still required.

## Observed differences and minimal reproductions

Observed locally with CPython 3.13.14, serde_json 1.0.154 default features and the
existing Cargo.lock. `float_roundtrip` is not enabled. Both string and reader paths
show the differences. Production source and Cargo dependency/features are unchanged.

| Case | N2 expectation | Observed Rust result |
|---|---|---|
| tempo `1.7976931348623158e308` | Accept, bits `7fefffffffffffff` (maximum finite). | Parse error: number out of range. |
| tempo `1.00000000000000011102230246251565404236316680908203125` | Exact midpoint; ties-even gives `3ff0000000000000` (1). | `3ff0000000000001`, the next value above 1. |
| In-memory bits `00167d447773ebd6` | Serialize and read back the same bits. | Serializes as `3.127525706621981e-308`, reads as `00167d447773ebd7`; another save/read changes back to `00167d447773ebd6`. |

Minimal gate commands after environment setup (each currently exits 1):

```powershell
target/numeric-venv/Scripts/python.exe -B scripts/verify_score_numerics.py --case "tempo 1.7976931348623158e308" --report target/repro-overflow.json
target/numeric-venv/Scripts/python.exe -B scripts/verify_score_numerics.py --case "midpoint/3ff0000000000000/tie" --report target/repro-tie.json
target/numeric-venv/Scripts/python.exe -B scripts/verify_score_numerics.py --case "sample/00167d447773ebd6" --report target/repro-roundtrip.json
```

The 267 fixed cases pass independent expectations; Rust disagrees on 10. The full
267-case plus 10,000-sample run reports 2,981 failing case records: 10 fixed and
2,971 sampled. Counting the `from_str` path once, there are 3 acceptance differences,
2,978 decode-bit differences and 988 readback differences; categories overlap.
`from_reader` has the same counts. Do not double-count paths as independent cases
or use this sample frequency as an estimate for ordinary user scores.

## Compatibility analysis and next decision (N4)

The current reader is not conformant to the newly approved precision rules, even
though the existing 47 Rust tests pass. A precision fix could make some previously
rejected boundary literals readable, change the decoded value of accepted literals
by an adjacent binary64 value, and stabilize saving/reloading. Consequently byte
output, equality comparisons and later time calculations could change. The file
shape, f64 field type, integer bounds and logical pitch mapping need not change.

The upstream `float_roundtrip` feature is a candidate for a separately authorized
evaluation, not an applied fix and not proof of full N2 conformance. serde_json is
currently a dev dependency in this workspace; enabling the feature would change
the tested input path, and a future production CLI would need explicit matching
configuration. Cargo feature unification must not accidentally make a different
test decoder appear to be the unchanged production path.

**Next approval needed:** evaluate an explicit parser-configuration change against
this same strict corpus and report the acceptance/bit-output impact before adopting
it. No feature, dependency version, production parser or expectation is changed
in this task. Do not permit a one-ULP tolerance, ignore these cases, or replace the
oracle with observed Rust results to make CI green.

References: [Python JSON hooks](https://docs.python.org/3.13/library/json.html),
[jsonschema type checking](https://python-jsonschema.readthedocs.io/en/stable/validate/),
[serde_json 1.0.154 feature definitions](https://github.com/serde-rs/json/blob/v1.0.154/Cargo.toml).
