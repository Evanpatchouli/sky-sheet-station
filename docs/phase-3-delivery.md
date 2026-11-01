# Phase 3 delivery and verification — 2026-11-01

P3-A, P3-B and P3-C are implemented and locally verified on Windows. This closes
the concrete omissions identified in the [full-prompt audit](phase-3-full-prompt-audit.md).
It does not certify a general-purpose melody classifier, external asset licensing,
Linux CI or Phase 4 arrangement. This record captures the implementation and local
verification before the owner's subsequent authorization to commit and push.
Commit identity and remote state are recorded by Git; no merge, release or deployment
is part of this delivery.

## Delivery summary

- **Implemented:** unrestricted source model, explainable analysis, required CLI
  commands and analysis inside the existing desktop application.
- **Source preservation:** ode-to-joy retains all 52 notes, including the 22
  pitches outside sky-15; exact source properties and bytes are checked.
- **Engineering evidence:** 103 root tests, 9 desktop Rust tests, 8 player tests,
  formatting/lint, Release builds and the independent numeric gate pass locally.
- **Heuristic evidence:** 14 of 17 track labels agree on 14 fixed synthetic cases.
  This is provisional synthetic-label agreement, not real-music identification
  accuracy; all three disagreements output Unknown.
- **Open evidence:** external MIDI provenance/license, independent human musical
  references and hosted Linux CI. These are distinct from implemented functionality.

## A. Model and staged implementation

**P3-A:** `skysheet-music` is independent of Score, MIDI parsing, GUI and Sky pitch
constraints. `MusicDocument` contains identity, source name/format, source PPQ,
tracks, notes, unmodified tempo/meter events, metadata, annotations, diagnostics
and optional report references. `MusicNote` holds ID, pitch 0..127, integer start/
end ticks and track. MIDI channel/velocity/on/off indices live in optional
`MidiNoteSource`; non-MIDI notes need no fabricated fields. Pitch helpers expose
canonical names/octaves (C4 = MIDI 60), classes and signed semitone distances.
The source MIDI bytes remain in memory, while analysis JSON omits that byte blob.

**P3-B:** exact note statistics now include a full MIDI-pitch histogram, central
register thresholds, track ranges/activity, track-pair range/time/onset overlap,
same-onset note-ID groups, repeated-pitch groups and sounding intervals. Tempo
and meter are retained separately from derived views. Nominal bar/beat positions
use integer rational arithmetic and explicitly diagnose mid-bar changes and
unsupported grids. Repeated identical signatures do not shift the grid.

Per-track ranking includes empty tracks, stable ties, selected lane reference,
weighted score components, eligibility and uncertainty. Role candidates retain
Unknown; provisional Melody requires continuity evidence as well as activity.
Specific diagnostics cover sparse tracks, excessive density, synchronization,
ambiguous rankings, competing roles and bounded search/detail limits. Original
notes are never deleted by these limits.

**P3-C:** `inspect-music` and `analyze` use reusable Rust APIs; `analyze-midi` remains
an alias. Internal report version 2 records the changed provenance and score
breakdown contract. The existing CLI envelope/exit codes, Score v1, import/export
reports, D6 conversion and N2 reader are unchanged. The existing Tauri client
invokes packaged `analyze --json`; React displays the returned facts and rankings.
No new third-party production dependency, Python, cloud/API service, audio
transcription, automatic transposition/arrangement or playback-engine rewrite.

Implementation proceeded through separately exercised model, analysis, CLI,
native-transport and rendered-UI checks. The Rust core was tested before GUI work;
the native bridge was tested against the packaged CLI before window validation.
Detailed definitions and limits are in [music analysis](music-analysis.md).

## B. Actual ode-to-joy results

All 52 original notes and all 22 notes outside sky-15 are retained. The file hash
remains `6e70886112270d2b68a826d8cee5283768766c97f95b54881cc29244f679dc2a`.
The independently byte-transcribed reference checks every original pitch,
endpoint, track, channel, velocity and source event index, plus tempo/meter values.

| Source track | Name | Notes | Range | Active source ticks | Provisional role | Melody score |
|---:|---|---:|---|---:|---|---:|
| 0 | no source name | 0 | unknown | 0 | Unknown | 0 |
| 1 | Mano destra | 30 | C5–G5 / MIDI 72–79 | 14140 | Melody candidate | 0.853679 |
| 2 | Mano sinistra | 22 | G2–G3 / MIDI 43–55 | 14134 | Bass candidate | 0.655166 |

Track ranking is 1, 2, 0. Global maximum polyphony is 2, distinct onset positions
32, overlapping note pairs 32, and density about 1.629030 note events/quarter over
first onset to last end. Low/middle/high counts are 15/33/4 using exported
thresholds 55 and 79. The source explicitly provides 600000 us/quarter at tick 0
(display ratio 100 BPM) and 4/4; no tempo changes. Key remains uncertain.

Diagnostics describe retained velocity/program/meter metadata, the empty
conductor track's insufficient evidence and uncalibrated inference. A high score
does not turn a candidate into source fact. Existing `import-midi` still rejects
unsupported Sky pitches; analysis succeeds without a fake Score.

The owner committed this real file in Phase 2. External author/download/license
remain undocumented. It was neither modified nor copied into another public
fixture. The 14 synthetic cases are original repository-authored MIT material.

## C. CLI executed

All three requested commands were executed successfully using the built Release
CLI (the target-suffixed packaged executable), from the repository root:

```text
skysheet inspect-music audios/ode-to-joy.mid
skysheet analyze audios/ode-to-joy.mid
skysheet analyze audios/ode-to-joy.mid --json
```

The local generated JSON report is `target/ode-to-joy-analysis-v2.json`; it is
ignored build output, not an expected test answer. CLI integration tests cover
human/JSON output, required versions, Unicode paths, unsupported output options,
usage/input/I/O failures, source preservation and continued Phase 2 behavior.

## D. Desktop and rendered verification

The existing toolbar now provides **分析 MIDI** and **分析结果**. A wide dialog
displays track statistics/ranges, role hypotheses and reasons, melody ranking and
components, rhythm/registers, Tempo Map, nominal Meter Map, original meter events
and diagnostics. Tables page at 20 rows and scroll horizontally on narrow screens.
The GUI does not reimplement musical analysis. Integers above JavaScript's exact
range become decimal strings in native display transport, without changing CLI
JSON or source ticks.

**Native evidence:** launched the Windows Release application at
`http://tauri.localhost/`, loaded the existing Score example through CLI validation,
opened **分析 MIDI**, selected the real `E:\sky-sheet-station\audios\ode-to-joy.mid`
in the Windows file picker, and observed 52 notes, all three tracks, right-/left-
hand ranges, rankings, roles, 600000 tempo and 4/4. Closing analysis retained the
6-note Score. Playback subsequently advanced the progress value to approximately
0.62 seconds, moved the playhead and highlighted G4; audition was then paused.
Physical speaker output was not independently measured.

**Browser QA:** the Browser runtime skill was not available; cached Playwright
with local Edge was used without installing dependencies. At
`http://127.0.0.1:1420/`, the fixture was an actual CLI-generated analysis report
with test-only mocked IPC. This checks rendering/state flows; native selection
and packaged-CLI tests separately establish the real bridge. Repeated after the
final frontend update at 1280×880, 800×880 and 390×880.

| UI check | Result |
|---|---|
| Page identity and nonblank content | Correct SkySheet title/URL and meaningful content |
| Framework overlay / script errors | No overlay or page exceptions; no relevant console errors (development favicon 404 excluded) |
| Analysis display | Real CLI data: 52 notes, Mano destra/sinistra, correct ranges/tempo |
| Interactions | Component details expand; Tempo/Meter sections visible |
| Cancel/error handling | Previous analysis and loaded Score retained; error visible |
| Player regression | Playback progresses before and after analysis; Score title retained |
| Responsive layout | Document width matches each viewport; wide tables scroll within dialog |
| Screenshots | Desktop overview, tempo section, 800 and 390 widths visually inspected |

QA script and PNGs are outside the repository under the session's visualization
directory, `phase3/`. No browser mock or fixture injection is in production.
The initial desktop Release rebuild encountered a file lock from the still-running
old application. Its empty window was closed, and the rebuild succeeded.

## E. Engineering and evaluation evidence

| Check actually run | Result |
|---|---|
| `cargo test --workspace --locked` | 103 passed: Score 51, Music 12, MIDI 22, CLI 18 |
| Focused Music/MIDI/CLI suites during implementation | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| Desktop manifest fmt/test/Clippy | Passed; 9 tests, including real packaged CLI analysis and existing N2/Score paths |
| `npm test` | 8 player timing/engine tests passed |
| `npm run build` | TypeScript and Vite production build passed |
| `npm run desktop:build` | CLI sidecar and native Windows Release executable built; no installer/release publication |
| Independent numeric workspace fmt/test/Clippy | Passed; 20 tests |
| Full independent numeric gate | 267 fixed + 10000 sampled cases passed, zero failures |
| Final diff/asset checks | No whitespace errors; original MIDI, Score, D6, player engine and isolated lockfiles unchanged |

Numeric evidence is `target/phase-03-complete-numeric-20261101.json`, produced with
the documented isolated tool's `--report` option. Windows incremental-cache
access-denied notes occurred in successful builds; they were not test failures.

All 14 fixed synthetic reference cases now also pass through an independently
encoded MIDI byte path before analysis. Additional cases cover empty tracks,
4096 notes spanning pitches 0..127, maximum legal MIDI deltas, u64 source tick
extremes, exact overlap oracles, component arithmetic (ascending scale = 44/47),
stable tied ranks, Unknown for sparse/unsupported continuity evidence and
redundant-meter grid stability. Expectations are not generated by the analyzer.

On 14 fixed synthetic cases containing 17 tracks, **14 track labels match the
provisional references**, versus 12 initially. This is synthetic-label agreement.
Top-1/top-3 track hits remain **6/6** on the six unique-melody reference cases.
The two register-leap examples now conservatively abstain from a Melody role.
The three remaining disagreements are:

| Synthetic reference case / track | Reference label | Output |
|---|---|---|
| melody_and_accompaniment / accompaniment track | Accompaniment | Unknown |
| repeated_notes | Accompaniment | Unknown |
| percussion_hint | Rhythm | Unknown |

Competing melody/accompaniment or bass/rhythm rules do not have sufficient score
separation, so the classifier abstains. Candidate generation can still rank
nonmelodic material highly.
This is the same small regression corpus, not held-out generalization accuracy.
Labels are agent-authored design intent, not human-validated musical truth;
probabilities remain null, and no claims of AI-level melody recognition are made.

Final Release resource observations on Windows 11 Pro, i7-13620H, Rust 1.99.1:

| Notes, four tracks | min / median / max analysis ms (20 iterations) | Peak working set bytes | Compact JSON bytes |
|---:|---:|---:|---:|
| 10000 | 14.796 / 15.291 / 17.611 | 17125376 | 3354711 |
| 100000 | 168.714 / 176.118 / 195.098 | 140226560 | 34945018 |

The Rust `analysis_bench` example uses generic events without MIDI provenance.
Analysis timing excludes serialization; process memory includes it and is sampled
from the OS high-water property every 20 ms. It is not exact heap accounting.
These observations describe this generated workload only; no performance SLA.

## F. Git baseline and remaining limits

The following is the local-verification snapshot, before the separately
authorized commit/push. Consult Git history and the remote branch for the final
commit identity and synchronization state.

- Branch: `codex/phase-3-music-analysis`.
- HEAD: `c5977a91d22cdd9400c82f0df6b25470f7ebf274`, unchanged Phase 2 asset commit.
- Phase 2 implementation parent: `aad681ac5b7aa389441df714e06aa2fb7b171e96`.
- Main remains `6a91bef810eb7523db2b1899d61d28d86d1f53af`; no branch merge.
- Working tree contains unstaged modified and untracked Phase 3 files; no commit/push.
- Changes cover the new music crate/tests/benchmark, MIDI adapter/tests, CLI
  command/report module, desktop transport/types/component/toolbar styles and
  documentation/manifests. Root lock changes are local dependency edges and an
  already-used test dependency; no third-party version change.
- A fresh Actions query still returned zero runs for Phase 2. No Phase 2 or
  Phase 3 Linux-CI success is claimed; the current local changes are not on GitHub.
- Heuristic quality, external asset provenance and human musical review remain
  explicit limitations. Format 2/SMPTE and controller-dependent interpretation
  keep existing unsupported-input diagnostics. Pairwise/voice search limits are
  diagnosed and preserve original data. No Phase 4 work was started.
