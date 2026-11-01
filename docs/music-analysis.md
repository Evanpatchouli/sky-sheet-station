# Music analysis primitives (Phase 3)

Phase 3 P3-A/B/C are implemented locally on `codex/phase-3-music-analysis`, based
on Phase 2 `c5977a9`. The full-prompt audit gaps have been addressed, including
optional MIDI provenance, track/register/overlap analysis, musical positions,
required CLI commands and the existing desktop's analysis view. See the
[delivery and verification record](phase-3-delivery.md). No Phase 4 arrangement,
audio transcription, playback-engine refactor or Score v1 format change is included
in that Phase 3 milestone. Phase 4-A subsequently adds a separate arrangement
consumer and an explicit MIDI D6 projection without changing source analysis;
see [Phase 4-A](phase-4a-arrangement.md).

## CLI and library entry points

```powershell
cargo run -p skysheet-cli --locked -- inspect-music audios/ode-to-joy.mid
cargo run -p skysheet-cli --locked -- analyze audios/ode-to-joy.mid
cargo run -p skysheet-cli --locked -- analyze audios/ode-to-joy.mid --json
```

JSON retains the existing CLI envelope (`output_version: 1`, `ok`, `command`,
`path`, `exit_code`, `result`, `error`). The new command's `result.report` is a
`MusicAnalysisReport` with `report_version: 2`, `analyzer_version: primitives-2`.
It contains `source_document_id`, an auditable `source` snapshot,
`note_statistics`, `pitch_statistics`, `tempo_analysis`, `rhythm_analysis`,
`key_candidates`, `melody_candidates`, `track_roles`, `track_ranking`,
`structure_analysis`, `confidence`, `diagnostics`. `analyze-midi` remains an alias
with its own command name in the envelope. `inspect-music` returns a version-2
`MusicInspectionReport` containing source/statistical/structural/tempo fields and
diagnostics, without key/melody/role inference. Version 2 replaces the initial
unreleased report draft because note provenance and score breakdowns changed;
Score v1 and Phase 2 import/export report versions are untouched.
Analysis accepts no `--output` or `--tempo`, writes no file and produces no Score.
Exit codes remain 0 success, 1 invalid/unsupported input or analysis limit,
2 usage, 3 I/O, 4 internal failure. Success can include uncertain inferences.

Dependency direction:

```text
CLI -> skysheet-midi::read_music -> existing Phase 2 inspect/scanner
                            |-> skysheet-music::MusicDocument
CLI -> skysheet-music::analyze -> MusicAnalysisReport
future arrangement -> source events + reviewed analysis -> skysheet-score
```

`skysheet-music` depends only on the already-used Serde. It has no MIDI parser,
Score, target instrument, GUI or playback dependency. MIDI owns the adapter;
CLI reads bytes and formats results. All third-party package versions and parser
features are unchanged. Cargo.lock adds one local crate and local edges, plus
the already-used serde_json as a MIDI test-only dependency.

`extract_melody(&document, &candidate)` returns references to original notes,
checking document/note IDs. It changes neither events nor other voices. Callers
retaining candidates must keep the matching document immutable; IDs are
provenance references, not tamper-proof attestations.

## Source representation

`MusicDocument` retains ID, source name/format, original positive PPQ, all tracks
(including empty ones), paired notes, original tempo/meter events and metadata.
`MusicNote` retains pitch 0..127, integer start/end ticks, track, event ID and
diagnostics. Its optional `midi` source object retains velocity 1..127, channel
and on/off event indices for MIDI inputs. Authored or future transcribed notes
can use `midi: null` without inventing those values. `MidiPitch` provides pitch
class, signed semitone distance and canonical sharp name/octave (C4 = 60).
Overlapping unisons, chords and synchronized tracks remain
separate events. `MidiPitch::new` checks range; document validation checks timing,
identity, track references, MIDI values and contradictory maps before analysis.

The adapter uses Phase 2 parsing and pairing, supporting SMF Format 0/1 with PPQ.
Format 2, SMPTE, unpaired notes, nonpositive duration/tempo, conflicting maps and
unsupported controller/sounding-event semantics fail with MIDI diagnostics.
Unlike `inspect-midi`, analysis does not certify partial interpretations when
the scanner reports semantic errors. This is the new command's policy; existing
import/inspect/export behavior is unchanged.

Exact MIDI bytes remain in `original_midi` in memory, including release velocity
and unmodeled event encoding. This field is omitted from report JSON, which is
an analysis snapshot rather than a byte-round-trip persistence format. Source
maps retain original order, duplicates and locations. Effective maps are separate
derived views. Names remain metadata hints; original metadata bytes are retained.
Source IDs use FNV-1a64 plus byte length for deterministic local references,
**not** collision-resistant security/content hashing. Durable callers can supply
their own document IDs.

Source facts occupy dedicated fields. User, inferred and future arrangement
annotations carry distinct `Origin` values and note references; analysis never
writes them into source facts. `analysis_refs` can associate external reports.
Inference evidence carries method, origin, document ID, track IDs and note IDs.
Default tempos carry `protocol_default`, explicit tempos `source`.

Raw ticks remain `u64`, independent of Score v1's narrower export bounds. JSON
consumers must preserve integer precision. Accumulated pitch-class durations use
`u128` and serialize as decimal strings, avoiding overflow through the CLI's JSON
value layer. Ratios/means are approximate descriptions, never event positions.
D6 conversion is neither invoked nor reimplemented. No rounded seconds accumulate.

## Analysis definitions

- Counts, ranges, per-track counts and duration histograms use all notes. A sweep
  of half-open `[start,end)` intervals gives overlap-pair counts and polyphony
  duration from tick zero to last note end, including silence. Adjacent off/on
  events do not overlap.
- Note density divides count by first-onset-to-last-end span in quarters. Sparse
  activity windows count onsets in `floor(tick/PPQ)` bins. Principal bins have at
  least the mean count over occupied bins; they are not phrase/bar segmentation.
- Effective tempo starts at 500000 us/quarter only without explicit tick-zero
  tempo. Identical simultaneous events keep all locations; conflicts fail. Changes
  count actual value changes, including from the default. Principal tempo has the
  largest accumulated source-tick span from zero to last note end, with ties
  choosing smaller us/quarter. It is not wall-clock weighting or measured BPM.
  Empty music has no principal tempo.
- Pitch classes count notes and source-tick durations. Means include every MIDI
  register. Principal classes rank duration, then class number.
- Key candidates measure duration coverage by 12 major and 12 natural-minor
  scales. Keep coverage >=0.75 and within 0.05 of best; fewer than three pitch
  classes yields Unknown. Relative major/minor remain ambiguous; `selected_key`
  stays null. There is no cadence, modulation, modal or unique-tonic claim.
- Rhythm uses distinct global onset ticks. Density is
  `(unique_onsets-1)/(last_onset-first_onset)` in quarters. Modal IOI fraction
  >=0.8 is regular; no IOIs is unknown. Repeated exact three-IOI windows count
  overlapping occurrences, retain top 32 and report truncation. Up to three modal
  IOIs become inferred period candidates with first-onset modulo period as phase.
  Source notes and PPQ are never quantized.
- Near-downbeat counts use unique onsets within PPQ/16 of a nominal bar boundary,
  default 4/4 and an analytical origin reset when numerator/denominator changes. Rational
  integer arithmetic preserves fractional beat lengths. Denominator exponent >31
  yields null. This does not recover pickups, performed accents or phrase boundaries.

`structure_analysis` additionally provides:

- A 128-bin MIDI pitch histogram and low/middle/high register counts. The two
  thresholds are centrally defined (`LOW_REGISTER_END=55`, `HIGH_REGISTER_START=79`)
  and exported in the report. These are descriptive heuristics, never Sky limits.
- Per-track counts, named ranges/spans, unioned activity intervals, active ticks
  and density over first onset through last end. Empty tracks retain null range/density.
- Pairwise pitch-range intersection, exact intersection of activity intervals,
  overlap ticks, shared distinct onsets and onset-set Jaccard ratio. Interval
  overlap and simultaneous onset are different measurements.
- Exact onset groups with source note IDs, half-open sounding intervals with
  active-note counts, and repeated-pitch groups by source track/channel/pitch.
  Repetition, interval overlap and same-tick groups are not labeled named chords.
- Meter segments and nominal one-based bar/beat positions for each onset group.
  Fractional beats are integer numerator/denominator ratios; bar numbers are
  decimal strings to support extreme source ticks. A mid-bar signature change
  starts the next nominal bar and emits `partial_bar_at_meter_change`. Repeated
  identical signatures/click-only changes do not shift the grid; all original
  events remain in `source.time_signature_map`. Unsupported exponents >31 give
  unknown beat positions and diagnostics; later absolute bar numbers remain
  unknown after such a segment. No audio beat detection or phrase segmentation.

Pairwise detail is omitted with `pairwise_complete: false` and an explicit
diagnostic above 128 nonempty tracks. Individual tracks and all source notes
remain complete. This prevents quadratic pair-report growth.

## Voices, roles and confidence

Within each source track/channel, deterministic greedy lanes select an available
nonoverlapping continuation with smallest pitch difference, then shortest gap,
then lane index. This does not identify original voices. At most 32 simultaneous
lanes per track/channel bound the search; excess emits `voice_search_limit` while
all notes remain in source and statistics.

Melody scores combine continuity (0.25), modal IOI fraction (0.15), duration
(0.15), density (0.15), short-gap connectivity (0.20), stepwise movement (0.10),
minus repeated-pitch fraction (0.20), clamped to [0,1]. Fewer than three notes
and channel 10 score zero. Short gaps mean <= one quarter, not detected phrases.
Features also expose onsets with another active track. Names, program numbers,
highest pitch and note count alone do not decide melody. All candidates remain
available, including zero-score lanes; there is no selected main melody.

Each candidate exposes raw component, weight, contribution, weighted sum,
eligibility and clipped total. `track_ranking` uses the maximum lane score per
track, includes empty tracks with 0, and breaks ties by original track index.

Role proposals use low register plus sequential connectivity (Bass), shared
onsets (Harmony), repetition plus regularity (Accompaniment, or Rhythm with
channel-10 hint), or melody score plus nonchordal activity/register (Melody).
Every track retains Unknown. Select a provisional role only when top rule score
>=0.65 and margin >=0.10; otherwise Unknown. Empty/sparse tracks remain Unknown.
Tracks can contain multiple roles; arpeggios, crossings, bass ostinatos, unusual
channels and large leaps can fool these rules. See actual evaluation failures.
The provisional Melody role also requires mean adjacent leap <=12 semitones;
regularity alone should not override missing pitch-continuity evidence. Rankings
remain visible even when role evidence is insufficient. Specific diagnostics
cover sparse tracks, >8 onsets/quarter, >=0.8 pairwise onset Jaccard, ambiguous
melody ranking and competing bass/accompaniment/rhythm evidence.

## Desktop

Choose **分析 MIDI** in the existing desktop window. The native file picker reads
a private snapshot and invokes the packaged `analyze --json` CLI. The analysis
dialog displays source counts, track ranges/roles, ranked score breakdowns,
rhythm/register statistics, Tempo Map, nominal Meter Map, original meter events
and diagnostics. Tables paginate at 20 rows; narrow tables scroll horizontally.
No frontend analysis algorithm or additional desktop program is introduced.

Analysis is independent of the loaded Score. Starting it pauses audition;
success, cancellation and failure preserve the Score. Cancellation/failure retain
the preceding analysis; errors are visible. **分析结果** reopens the report.
The existing MIDI import/player path keeps its original sky-15 restrictions.
Integers above JavaScript's exact range become decimal strings in the native
display projection only, so source ticks are not silently rounded by WebView.

All confidence probabilities are null with `uncalibrated_heuristic`. Scores and
coverage describe rules, not empirical correctness probabilities. Diagnostics
distinguish source info/warnings from analysis limitations. The 100000-note
analysis limit is explicit and never silently samples or truncates source data.

## Verification

See [Phase 3 evaluation](phase-3-evaluation.md) for references, measurements and
acceptance gaps. Reproduce quality and preservation checks:

```powershell
cargo test -p skysheet-music --locked --test corpus -- --nocapture
cargo test -p skysheet-midi --locked --test music -- --nocapture
cargo test -p skysheet-cli --locked --test cli
cargo build -p skysheet-music --example analysis_bench --release --locked
.\target\release\examples\analysis_bench.exe 10000
```
