# Phase 3 evaluation record — 2026-11-01

**Historical baseline:** this records the initial implementation against the first,
truncated attachment and its original 12/17 role agreement. The complete task's
gaps were then audited and implemented. Current results (14/17 provisional role
agreement), GUI evidence and final verification are in the
[Phase 3 delivery record](phase-3-delivery.md). Figures below remain historical.

## Baseline and boundary

Preflight found a clean workspace on `codex/phase-2-midi` at
`c5977a91d22cdd9400c82f0df6b25470f7ebf274` (`assets: 欢乐颂 midi`). Parent
`aad681ac5b7aa389441df714e06aa2fb7b171e96` contains Phase 2 implementation.
Local/remote main were `6a91bef810eb7523db2b1899d61d28d86d1f53af`.
Read-only `git ls-remote` confirmed both branch heads. Phase 3 branched locally
from Phase 2; no merge, commit, push or PR creation was performed.

GitHub connector queries found no commit status/PR workflow runs. Actions REST
API independently reported zero runs for the Phase 2 branch. The repository
workflow triggers only on main pushes and PRs. Thus Phase 2 hosted CI remains
unverified, not passed or known failing. All existing 84 root tests passed locally
before implementation. Existing workspace CI commands include the new crate.

The initial task attachment ended in section IX after a partial `记录` bullet.
The complete task subsequently supplied by the owner explicitly includes desktop
analysis integration, which is still missing. Phase 4 remains excluded.

## Corpus and reference independence

The [event tables](../crates/skysheet-music/tests/fixtures/cases.json) contain 14
original synthetic cases with explicit integer events and manually specified
counts, polyphony, roles, acceptable melody tracks and uniqueness flags.
Expectations are not generated from analyzer outputs. Synthetic material is
repository-authored MIT content, not copied third-party music. There is no
train/test split or calibrated model.

**The implementation agent authored these design-intent labels; human review is
pending.** Measurements are agreement with provisional references, not human-
validated accuracy. The requested human reference results remain an acceptance
gap. Review should especially verify crossing voices, register alternation,
repeated accompaniment and percussion. No human reviewer is invented.

| Case | Reference roles by track | Melody reference | Observed roles |
|---|---|---|---|
| single_melody | Melody | unique track 0 | Melody |
| simple_chord | Harmony | none | Harmony |
| alternating_registers | Unknown | none | Melody (false positive) |
| melody_and_accompaniment | Melody / Accompaniment | unique track 0 | Melody / Unknown |
| synchronized_tracks | Melody / Melody | tracks 0 or 1; not unique | Melody / Melody |
| tempo_changes | Melody | unique track 0 | Melody |
| meter_changes | Melody | unique track 0 | Melody |
| repeated_notes | Accompaniment | none | Unknown |
| wide_range | Unknown | none | Melody (false positive) |
| irregular_rhythm | Melody | unique track 0 | Melody |
| no_clear_melody | Harmony / Harmony | none | Harmony / Harmony |
| ambiguous_key | Melody | unique track 0; uncertain key | Melody |
| percussion_hint | Rhythm | none | Unknown |
| crossing_voices | Harmony | none selected | Harmony |

Role agreement is **12/17 (70.6%)**. On six unique-melody reference cases,
thresholded top-1 and top-3 track hits are **6/6** each. A hit requires score >=0.65
and a reference track; this is track retrieval, not exact event-sequence accuracy
or proof of a recognizable melody. Ambiguous synchronization is excluded from
unique-reference accuracy. Among seven no-selected-melody cases, four produce
score >=0.65 candidates: register alternation, repeated notes, wide range and
crossing voices. Candidate generation has no abstaining selection API.

Rhythmic regularity can outweigh poor pitch continuity, causing false Melody
roles. Repetition and bass/rhythm rule ties yield Unknown. These failures were
retained instead of relabeling references or tuning for perfect test scores.
They limit suitability for automatic arrangement. Key tests retain uncertainty
and both C major/A minor on the ambiguous case; no unique-tonic accuracy is
claimed. Probabilities remain null throughout.

## Real MIDI preservation

`audios/ode-to-joy.mid` is the tracked owner-supplied 575-byte SMF in `c5977a9`.
SHA-256: `6e70886112270d2b68a826d8cee5283768766c97f95b54881cc29244f679dc2a`.
External author/download URL and redistribution license are unrecorded. It is
retained where the owner committed it, not copied into a newly licensed corpus.

The [MIDI test](../crates/skysheet-midi/tests/music.rs) contains an independent
reference transcribed from raw hex: all 52 pitches, start/end ticks, tracks,
channels, velocities, on/off event indices, and original tempo/meter locations
and values. VLQ values and track names are documented there. Three tracks hold
0/30/22 notes at PPQ 480. Expectations do not call the analyzer or MIDI writer.
Document equality before/after and report/source equality verify nonmutation;
original bytes remain in memory and the input file is unchanged.

All **22 notes outside sky-15 remain**, including pitches 43, 48, 50 and 55.
Tempo remains 600000 us/quarter at zero (display ratio 100 BPM), meter 4/4.
Provisional ranking places right-hand track 1 first; roles are Unknown/Melody/Bass.
Musical reference review is pending. Existing `import-midi` still rejects the
22 unsupported pitches; `analyze-midi` succeeds without creating a Score.

## Engineering and resource measurements

Observed on Windows 11 Pro 10.0.26200, Intel Core i7-13620H,
Rust 1.99.1, x86_64-pc-windows-msvc. No Linux run or new hosted CI is claimed.

| Check | Observed result |
|---|---|
| `cargo test --workspace --locked` | Passed; 96 tests, including the unchanged Score and MIDI suites |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| Isolated numeric workspace fmt/test/clippy | Passed; 20 tests |
| Full independent numeric gate | 267 fixed cases + 10000 samples passed; zero failures |
| Real CLI human/JSON and error/usage/I/O paths | Passed in integration tests; input and existing command behavior preserved |
| `cargo build -p skysheet-cli --release --locked` | Passed; Release human/JSON analysis of ode-to-joy also passed |

The full gate used
`cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked -- --report target/phase-03-numeric-20261101.json`.
The isolated workspace checks follow [numeric verification](numeric-verification.md).
Windows emitted incremental-cache access-denied notes during some successful
checks. An initial Clippy collapsible-if finding was fixed and Clippy rerun.
The local Release report is `target/ode-to-joy-analysis.json` (ignored generated
output, not a golden test expectation). Final diff checks found no whitespace
errors; the original MIDI SHA-256 is unchanged.

The deterministic Rust `analysis_bench` example builds four synchronized source
tracks and analyzes each size 20 times. `Instant` timing excludes serialization;
the process peak includes construction, analysis, report clones and serialization.
Times are min / median (sorted element 10) / max, with no warmup discarded.

| Notes | Analysis time, ms | Observed peak working set, bytes | Compact report bytes |
|---:|---:|---:|---:|
| 10000 | 11.909 / 12.545 / 14.853 | 12812288 | 2071603 |
| 100000 | 145.496 / 164.049 / 209.216 | 84160512 | 21644982 |

These are local measurements, not platform support or performance guarantees.
They measure the music analyzer on synthetic events, not MIDI parse/CLI startup
time, adversarial maximum polyphony or external musical accuracy. JSON duplicates
source references for auditability and can be much larger than the MIDI file.

Reproduce after the build command in [usage](music-analysis.md#verification):

```powershell
$analysisPeak = 0L
$analysisProcess = Start-Process -FilePath "$PWD/target/release/examples/analysis_bench.exe" -ArgumentList '10000' -WindowStyle Hidden -PassThru -RedirectStandardOutput "$PWD/target/analysis-bench.txt"
while (-not $analysisProcess.HasExited) {
    $analysisProcess.Refresh()
    $analysisPeak = [Math]::Max($analysisPeak, $analysisProcess.PeakWorkingSet64)
    Start-Sleep -Milliseconds 20
}
$analysisProcess.WaitForExit()
Get-Content target/analysis-bench.txt
$analysisPeak
```

Use `100000` for the second size. Peak working set is sampled from the OS
high-water property while the process is alive; it is not allocated-heap byte
accounting and can miss an increase during the final polling interval.

Source review confirms no Score/schema/example, D6 arithmetic, N2 parser feature,
frontend, desktop, isolated lockfile or third-party version change. Generated
reports and binaries stay under ignored `target/`. Human reference review,
real-file provenance and the concrete requirements identified in the full-prompt
audit are outstanding; no full Phase 3 acceptance or Phase 2 hosted-CI success is
declared. The complete prompt is now available.
