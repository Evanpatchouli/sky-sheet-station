# Phase 4-A: global transposition and Sky 15-key adaptation

Status: Phase 4-A implemented on the Phase 3 baseline, with the owner's approved
D6 projection boundary. Global transposition, exact-key mapping, separate loss
reports, CLI and existing desktop integration are included. Phase 4-B/C remain
deferred. Human musical quality assessment and hosted Linux CI remain unverified.

## Preflight

- Clean starting worktree; `git fetch origin` completed.
- `main` / `origin/main`: `6a91bef`.
- Phase 2 local/remote: `c5977a9`.
- Phase 3 local/remote: `bdfa0b62e6602cd41e494a42053e5b93bb8043d5`;
  its parent is `c5977a91d22cdd9400c82f0df6b25470f7ebf274`.
- Created `codex/phase-4a-sky-arrangement` from Phase 3, without reset,
  commit, push, or modification to the source MIDI.
- GitHub commit workflow query (PR events) returned no runs. A separate
  repository Actions query with `head_sha=bdfa0b62e6602cd41e494a42053e5b93bb8043d5`
  returned `total_count: 0`. Linux Phase 3 CI is **unverified**, not passing.
- The supplied task attachment ends mid-list at section XI.B, after the Chinese
  character `正`. The owner subsequently supplied the engineering/test requirements
  through section XIV and explicitly approved the projection boundary below.

## Approved time-boundary decision

The task requests unchanged source tick numbers and PPQ while generating Score
v1. The checked-in `read_music` retains original MIDI ticks without D6 conversion.
The actual ode-to-joy source is **480 PPQ**, 100 BPM, initial 4/4. Score v1 requires
**960 PPQ**. These numerical requirements cannot all hold simultaneously.

The owner explicitly selected **采用 D6 投影边界**:

1. Keep the original MusicDocument, source MIDI, and source tempo/meter maps intact.
2. In the MIDI layer, reuse the existing D6 implementation to make a separate
   Score-time projection before arranging. Do not change D6 arithmetic or old
   `import-midi` acceptance policy.
3. Arrange changes pitch only. Its output endpoints/durations, PPQ, reference
   tempo bits and nominal meter must equal the projection exactly.
4. Keep original ticks and projected ticks together in the separate arrangement
   report, including D6 rounding/flattening diagnostics. Do not claim that a
   converted tick value equals its original MIDI tick value.

`skysheet_midi::project_music` creates a privately constructed, immutable
`ScoreTimeProjection`. It shares `prepare_timing` with the original importer,
including defaults, conflicts, nominal meter, exact D6 arithmetic and diagnostics.
Arrange checks source identity, PPQ, tempo/meter maps and every source note against
the projection; stale or mismatched projections fail. Retained output notes copy
projected start/duration directly, without quantization. All source notes undergo
projection before pitch filtering: an invalid time is never hidden by dropping
an unplayable pitch. Existing `import-midi` still refuses unsupported pitches.

## Implemented pitch search

`skysheet-arrange` depends on existing workspace Music, MIDI, Score, Serde and
serde_json; no new third-party package or parser feature is introduced. It borrows
the source document and runs the existing Phase 3 analysis. Its target profile is
constructed from `skysheet_score::SKY_15_KEYS`, not another hand-maintained table.

Auto evaluates all 255 integer shifts in `-127..=127`, ascending. Every shift that
can map any legal MIDI pitch to another legal MIDI pitch lies in this interval.
Larger shifts cannot retain any note. Fixed mode evaluates exactly its supplied
integer, rejects values outside this interval and never falls back to auto.
Each note uses `source_midi + shift`; only membership in the actual ordered target
keys retains it. Shifted values outside MIDI 0..127 are unplayable. No clamps,
nearest-key substitutions, octave folding, merging or polyphony limits exist.

### Melody uncertainty policy

Select the first Phase 3 ranked track only if its best lane is eligible, its score
is at least 0.65, the Phase 3 track role is Melody and its lead over the next ranked
track is at least 0.10 (or there is no next track). A smaller margin explicitly
reports competing candidates. Otherwise select Unknown and prioritize total
retention. These are transparent engineering thresholds, not calibrated musical
confidence or accuracy probabilities. Preserve all Phase 3 rankings/reasons.

Accompaniment counts use Phase 3 Bass, Harmony, Accompaniment and Rhythm roles,
excluding the selected melody track. Unknown tracks are not silently reclassified
as accompaniment; they still count in total retention. A melody selection applies
to every note of that source track, not just the best candidate lane.

### Comparison and ties

Maximize the following lexicographic tuple; no combined weighted objective:

1. Selected melody track's retained count (omitted when Unknown).
2. Total retained count.
3. Accompaniment retained count.
4. Negative absolute shift (prefer the smallest pitch displacement).
5. Count of adjacent source melody-note pairs both retained.
6. Negative sum of absolute pitch intervals between consecutive retained melody
   notes (prefer smaller total movement after omissions).
7. Negative signed shift (negative shift wins an otherwise exact +/- tie).

Continuity uses original per-track source vector order; it never reorders notes
or creates phrases. Constant transposition alone preserves intervals; omissions
can change the retained succession. Melody retention ratios and dropped counts
are reported; their fixed denominators make them equivalent to already-compared
counts, so they are not weighted twice. Empty denominators are null, not 100%.
Unknown melody has null melody metrics and zero continuity metrics. The final
signed tie rule, explicit order and integer tuple avoid HashMap-order choices.

### Actual first sample observation

For `audios/ode-to-joy.mid`, the source remains 52 notes: track 1 has 30 melody
notes and track 2 has 22 bass/accompaniment notes. The top Phase 3 scores are
approximately 0.853679 and 0.655166, so track 1 is a provisional melody candidate.
Auto selects **0 semitones**: **30/30 melody**, **0/22 accompaniment**, **30/52
overall (57.6923%)**, **22 dropped**. A real Score and full report were generated
and passed the existing raw-byte `validate` command. The 30 retained notes have
exactly twice their raw 480-PPQ onset and duration numbers, equal to their D6
projection at 960 PPQ; tempo remains exactly 100 BPM and meter 4/4. Fixed +12
remains +12 with losses. The source file SHA-256 remains
`6e70886112270d2b68a826d8cee5283768766c97f95b54881cc29244f679dc2a`.

## CLI and output safety

```text
skysheet arrange audios/ode-to-joy.mid -o target/ode-to-joy-sky.json
skysheet arrange input.mid -o result.json --auto --report result-report.json --json
skysheet arrange input.mid -o fixed.json --transpose -12 --json
skysheet validate result.json --json
```

Auto is the default; `--auto` and `--transpose` are mutually exclusive. Fixed
offsets are signed integers in -127..127. `--report` defaults to the full output
filename plus `.arrangement.json`. `--` supports option-looking filenames.
Exit codes retain the existing contract: 0 success (including diagnosed note
loss), 1 invalid/unconvertible input or zero retained notes, 2 usage, 3 I/O,
4 internal failure. JSON mode emits exactly one CLI response on stdout.

The report has `arrangement_report_version: 1`, source identity/name, mode,
selected offset, all candidates, source track roles/rankings and scoring reasons,
per-track ranges/counts/ratios, timing diagnostics, and one `note_mappings` entry
for **every** source note. Entries retain source note/event identity, pitch name
and MIDI number, global shift, target MIDI/pitch, zero-based key index, raw and
projected timing, success and specific failure reason. Key index + 1 derives the
game key number. MIDI overflow, below/above range and missing actual key are
distinct reasons. No such fields enter Score v1. Ratios with no denominator are
null. Source track indices are zero-based; source IDs and per-track vector order
survive, including duplicate pitches, repeats and simultaneous notes.

Every successful Score validates, serializes, reads back directly through the
production reader and validates again; tempo bits, time and key correspondence
are checked. An empty result fails with a complete loss report and no Score.
Input/preparation failure produces a separate versioned report with
`stage: preparation_failed`, unknown counts as null and original MIDI diagnostics;
the CLI error includes it as `arrangement_report`. Invalid destination/usage
cannot guarantee a report file, but still returns the normal explicit error.

Output and report paths must be distinct and unused. Parent aliases are resolved;
`create_new` is the final no-overwrite guard against existing files, symlinks,
hardlinks and races. Both successful output buffers are complete before either
file is reserved. If the second reservation or any write/sync fails, only this
invocation's created files are removed; cleanup failures are reported. No promise
of a two-file crash-atomic transaction or recovery after external process kill is
made. Failed writes do not advertise a usable Score path. Source files are never
opened for writing.

## Desktop flow and boundaries

Analyze MIDI → **适配为 Sky 乐谱** (or **Sky 编曲**) → automatic/fixed offset →
**生成并保存 Score** → review the report → **加载生成乐谱并预览试听**.
The native save dialog selects a new Score path; CLI also saves its adjacent
report. The desktop retains the successfully analyzed byte snapshot, so later
source-file edits cannot silently change arrangement input. Source identity,
command/version/exit code, output path and validation flags are checked on the
CLI response. Score loading still uses the packaged CLI and raw Score reader.
Only explicit successful loading replaces the previous player Score; failure
and picker cancellation preserve it. A new analysis resets the pending result.

The frontend only accepts parameters and displays reports, with a 20-row loss
table and all-candidate details. It reports provisional melody/unknown roles,
melody and accompaniment counts/ratios, total retention, and saved paths.
Large source integers use the existing exact decimal-string display projection.
Local synthesized audition remains a desktop feature; no game input exists.

## Resource and maintenance limits

Arrangement/projection reject more than 100,000 notes or 4,096 tracks, without
partial conversion. The note cap matches existing analysis; the track cap bounds
track-indexed allocations and the analyzer's per-track passes for sparse files.
The fixed 15-key profile is constructed once per search, not per note/candidate.
Search costs O(255 × N × 15); fixed mode O(N × 15). It stores candidate summaries
and only maps the selected candidate into Score. It reuses one existing Phase 3
analysis call (whose current API includes one source snapshot copy), rather than
cloning notes for each candidate. A bounded sorted note-ID map checks output
correspondence. No new third-party dependency, numeric implementation or parser
feature is introduced, and no throughput guarantee is claimed.

## Initial verification checkpoint (historical)

Five focused search tests passed, including actual-key versus range checks,
positive/negative and non-octave shifts, MIDI boundaries, deterministic tie and
lexicographic comparison, sparse/bass/ambiguous Unknown behavior, and real-file
source preservation. Root workspace tests, formatting and Clippy with `-D warnings`
passed. The isolated numeric workspace's formatting, 20 tests and Clippy passed;
its full gate passed **267 fixed cases + 10,000 samples, zero failing records**
(`target/phase4a-numeric-checkpoint.json`, ignored local evidence). The isolated
test suite also exercised the existing research/focused N2 regressions. Final
`git diff --check` passed. Rust emitted nonfatal Windows incremental-cache
finalization access-denied notes; all reported commands completed successfully.
No GUI audition or full Phase 4-A success is claimed.

## Final local verification

The initial checkpoint above was superseded after the owner approved D6 projection.
The final implementation was verified on Windows with these observed results:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --locked` | 121 tests passed: Arrange 13, CLI 23, MIDI 22, Music 12, Score 51 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| Focused Arrange, MIDI and CLI tests during implementation | Passed |
| Desktop isolated Rust fmt/test/Clippy | Passed; 11 tests, including real packaged-CLI arrangement/load and failed fixed offset |
| `npm run test` | 8 existing timing/playback tests passed |
| `npm run build` | TypeScript and Vite production build passed |
| `npm run desktop:build` | Final Windows Release CLI sidecar and desktop executable built; no installer/publication |
| Independent numeric workspace fmt/test/Clippy | Passed; 20 tests including research/focused N2 regressions |
| Complete independent numeric gate | 267 fixed + 10,000 historical samples, zero failures; `target/phase4a-numeric-final.json` |
| Generated CLI and native Score files, using final packaged `validate --json` | Both passed |
| `git diff --check` | Passed |

The original Score crate, JSON Schema, format specification, example fixture,
source MIDI, N2 parser, Oracle and historical expectations are unchanged. The root
lockfile adds only the internal Arrange crate and dependency edges; no external
package/version/feature changes. This local verification record was completed
before the owner subsequently authorized committing and pushing Phase 4-A.

### Native and browser evidence

Native Windows Release, at `http://tauri.localhost/`: loaded the existing six-note
example, selected the actual source MIDI through the file picker, observed all 52
source notes, selected automatic adaptation, and saved
`target/欢乐颂-native-sky.json` plus its adjacent `.arrangement.json` report.
The report showed 30 retained, 22 discarded, 255 candidates and all loss details.
The prior Score stayed loaded until the explicit preview/load action. Loading
showed 3 tracks, 30 notes, 100 BPM, 4/4 and 960 PPQ. Playback advanced to about
6.28 seconds with scrolling playhead and D5 key highlight, and was then paused.
This verifies the application audition path; physical speaker output and human
musical recognizability were not independently measured. The verification window
was closed before the final Release rebuild to avoid Windows executable locks.

Rendered QA used existing cached Playwright 1.63.0 and local Edge because the
Browser runtime skill was unavailable; no dependencies were installed. At
`http://127.0.0.1:1420/`, test-only IPC invoked the **actual packaged CLI** and
loaded its actual output files. It checked automatic/fixed +12/failed +127,
noninteger input rejection, 22-loss pagination, preserving the old Score before
explicit load, successful playback and key activation, failure/cancellation
retention, and 1280/800/390 widths without page overflow or framework overlay.
Native file dialogs were independently checked above. Test hooks are outside
production source. The only browser console messages were the existing missing
`/favicon.ico` 404s; there were no application runtime errors.

The final QA script, JSON evidence and PNGs are outside the repository in this
chat's visualization directory, under `phase4a-qa.mjs` and `phase4a-AsHIfg/`.
The last UI adjustment moved the arrangement and preview actions above long
reports and added explicit melody/accompaniment counts; browser QA and the
Release build were repeated afterwards. Final desktop tests used the rebuilt
sidecar. Local generated CLI artifacts are `target/ode-to-joy-sky.json` and
`target/ode-to-joy-arrangement.json`, intentionally excluded from Git.

### Remaining boundaries

- Phase 3 and Phase 4-A had no hosted Linux CI result at local acceptance. The
  current workflow runs on pushes to `main` and pull requests; pushing the Phase
  4-A feature branch alone does not trigger it. Commit/push is not CI acceptance.
- Heuristic melody roles remain uncalibrated; no human musical quality claim.
- This sample's accompaniment is completely lost under the permitted global
  transposition strategy. The report makes that loss explicit; it is not a
  complete two-hand arrangement.
- No Phase 4-B/C octave placement, context-aware substitution, rhythm rewriting,
  voice reduction or generated harmony was implemented.
- Source asset provenance remains the previously documented Phase 3 open item;
  no external MIDI was downloaded or published.
