# Phase 4-B/C: context-aware polyphonic arrangement

Status: locally implemented candidate, **not fully accepted**. Human musical
review and hosted Linux CI are pending. Phase 5/audio transcription are not begun.

## Baseline

Clean starting HEAD: `c0f14e5405bdd106edfbf385fc3ce888df171b35`.
`git fetch origin` succeeded; main `6a91bef`, Phase 2 `c5977a9`, Phase 3 `bdfa0b6`,
Phase 4-A `c0f14e5` matched remotes. Branch:
`codex/phase-4bc-polyphonic-arrangement`. No reset, merge, commit or push.
GitHub PR-run query and `gh run list --commit c0f14e...` returned no runs.
Workflow triggers: main push and PR, not feature push. Hosted Linux is unverified.
No Music/MIDI/Score production source, Score v1, D6, N2, source MIDI, dependency
or parser-feature changes. Arrange owns algorithms; Tauri invokes CLI; React
only edits options and displays results. No new runtime, service or game input.

## P4-B1 implemented objective

Rust `arrange` preserves the Phase 4-A algorithm. `arrange_with_options` and CLI
default use context mapping. Disabling octave shifts restores global-only pitch
mapping/search; explicit simple reduction or user ceiling can still apply.

1. Analyze once with existing Phase 3 roles/evidence and melody selection margin.
   Unknown is not relabelled accompaniment. Mixed voices within one track remain
   a limitation: this is track/onset optimization, not full voice separation.
2. Auto searches -127..127 using octave-feasible retention upper bounds: melody,
   total, support counts, then smallest absolute and signed shift. Fixed means
   exactly the user integer. A globally shifted pitch outside MIDI 0..127 cannot
   be rescued with an octave. No nearest-key/chromatic substitution exists.
3. Group each track by source onset. State: shared integer octave -10..10,
   highest original pitch representative, lost count and local cost. Actual
   `instrument.keys` determines membership. Retained members of one group keep
   exact internal intervals. Independent events are never merged.
4. DP minimizes `(lost count, additive integer cost)` lexicographically. Melody
   is solved first; bass uses its sounding minimum at each source onset. This
   is conditional optimization, not a joint global optimum.
5. Local cost sums target distances to role center plus `3*abs(octave)`. Centers:
   melody 76, bass 64, harmony/accompaniment 67 (polyphonic 69), others 72.
   Bass at/above active melody adds 480 in all modes (revised after audition).
   Later overlap within a sustain is diagnosed, not fully optimized.
6. Transition: `w*abs(target_interval-source_interval)` + `6*w` for sign change
   + `4*max(0,abs(target_interval)-12)` + `8*abs(octave_change)`.
   Melody `w=4*(melody_priority+1)` (default 16); other tracks use 16.
   Representatives are not a full inner-voice model. Unmappable groups can still
   affect representative transitions; this is an explicit heuristic limitation.
7. Ascending states/predecessors; smaller predecessor then smaller final state
   breaks ties. Optimality is limited to this finite per-track additive objective.
   Independent enumeration of 27 paths finds cost 41; a separate exact-tie test
   confirms stable selection. No claim of best musical arrangement.

## P4-B2 modes, importance and reports

- melody-first retains mapped events and gives bass/melody separation more weight.
- simple-accompaniment uses that mapping, then keeps at most two distinct support
  pitches per onset, by importance/lower source pitch/event order. Melody/Unknown
  are exempt. Repeated events of admitted pitches remain distinct.
- polyphonic retains mapped support and uses a higher harmony center (69 versus
  67). Bass separation now has the same protection as the other modes; unresolved
  same-key collisions remain in the report.

Priorities are integer 0..5, defaults 3/2/1. Importance: melody `1000+100*priority`,
bass `300+100*priority`, other known support `100+100*priority`, Unknown 200;
+20 on an original quarter-note onset. These are engineering weights, not
probabilities, calibrated accuracy or inferred roots. Melody priority also
controls contour cost. No automatic generated harmony exists.

Optional max polyphony 1..128 constrains half-open interval overlap, not just
onsets. Absent means unlimited; this is a user rule, not Score v1. A higher-
importance incoming event can remove a whole weaker overlapping event; ties keep
ongoing events. No shortening or movement. Greedy reduction is not globally
optimal and cannot backfill gaps. Every removed event has its ID and reason.

Onset groups list all source IDs and retained counts. Major/minor/diminished
triad candidates use exact pitch-class sets and handle inversions; not every
group is called a chord. Reports distinguish starts/overlaps and same-key events,
check exact interval changes, diagnose bass/melody crossing and support onset loss.
Conflict records show the new event and one representative plus full active count,
avoiding quadratic pair expansion; all events remain in note mappings.

Report v2 retains v1 fields and adds options, per-track used octave sets, final
ranges/adjusted counts, per-note global/voice/final pitch, key index, neighboring
interval changes, importance/reasons, onset/chord/conflict records and playability.
Candidate counts are pre-DP upper bounds; final role/track/mapping counts are
actual. Legacy accompaniment includes bass/harmony/accompaniment/rhythm; new
role metrics separate them. Every ratio has a denominator; zero returns null.

## Time, safety and P4-C

Kept times/durations copy the immutable D6 projection. Score PPQ, tempo bits,
nominal meter and projected times are checked exactly. Source PPQ 480 becomes
Score PPQ 960; raw and projected ticks are both reported. No rhythm rewrite,
quantization, velocity export, lyric/phrase segmentation or source mutation.
Successful Score validates, raw-byte round-trips through production N2 and checks
per-note correspondence. CLI retains exit codes and one JSON stdout response.
Reports identify failure stage. New destinations use create_new, completed
buffers precede reservation, cleanup removes only invocation-created files and
reports errors. Two-file saving is not crash-atomic.

Desktop: analyze snapshot → mode/options → private preview → report → explicit
load into existing player → audition → save new Score + independent report.
Failed/canceled operations preserve loaded Score. Request IDs ignore old results;
backend generations/tokens protect pending previews. Cancel-wait ignores late
completion; it does not terminate CLI. No frontend/Tauri arrangement algorithms.

## Actual Ode to Joy results — initial candidate, retained as evidence

Unchanged source SHA-256:
`6e70886112270d2b68a826d8cee5283768766c97f95b54881cc29244f679dc2a`.
Actual files: `target/p4bc-results/{baseline,melody-first,simple-accompaniment,polyphonic}.json`,
adjacent reports and JSON responses. All passed packaged `validate --json`.
Baseline uses --no-octave-shifts; original Phase 4-A tests also remain.

| Metric | 4-A baseline | Melody first | Simple accompaniment | Polyphonic |
|---|---:|---:|---:|---:|
| Source events | 52 | 52 | 52 | 52 |
| Global semitones | 0 | 0 | 0 | 0 |
| Melody | 30/30 | 30/30 | 30/30 | 30/30 |
| Bass (also legacy support) | 0/22 | 22/22 | 22/22 | 22/22 |
| Other accompaniment | 0/0 (null) | 0/0 (null) | 0/0 (null) | 0/0 (null) |
| Total | 30/52 | 52/52 | 52/52 | 52/52 |
| Octave-adjusted | 0 | 22 | 22 | 22 |
| Dropped | 22 | 0 | 0 | 0 |
| Maximum overlap | 1 | 2 | 2 | 2 |
| Melody MIDI range | 72–79 | 72–79 | 72–79 | 72–79 |
| Bass MIDI range | absent | 60–67 | 60–67 | 60–79 |
| Melody interval changes | 0/29 | 0/29 | 0/29 | 0/29 |
| Cross-track same-key overlaps | 0 | 0 | 0 | 9 |
| Bass/melody crossing events | 0 | 0 | 0 | 17 |

Melody-first/simple pitches coincide here: sparse support never exceeds the
reduction threshold. Polyphonic trades overlap for more bass interval retention.
These metrics do not prove musical quality or actual playability.

## Engineering evidence

Observed Windows checks (final reruns appended below):

| Check | Actual result |
|---|---|
| Root fmt/test/Clippy, locked | Passed; 129 tests: Arrange 20, CLI 24, MIDI 22, Music 12, Score 51 |
| Desktop Rust | 12 tests passed, including save collision cleanup and mapping mismatch rejection |
| Frontend | 9 tests passed: player 8, parameter boundaries 1 |
| TypeScript/Vite | Production build passed |
| Windows Release CLI/Tauri | Built; final rebuild follows closing QA window |
| Independent numeric fmt/test/Clippy | Passed; 20 tests |
| Full N2 gate | 267 fixed + 10,000 samples, zero failures, target/p4bc-numeric-gate.json |
| Hosted Linux | No result for uncommitted work; Phase 4-A query also empty |

Tests cover octave/boundary/black-key mapping, ascending/descending/repeats,
independent DP enumeration, triads/inversions, overlap, simple/Unknown/repeats,
later protected melody, deterministic real-file output/time/source preservation,
CLI invalid flags/fixed transpose/output collisions and regressions.

Browser skill absent: cached Playwright + Edge used without new dependencies.
At http://127.0.0.1:1420/, test-only IPC invoked actual packaged CLI and loaded
actual validated files. All modes, pagination, playback advancement, fixed +127
failure retaining Score, delayed cancellation, picker cancellation, 1280/800/390
widths, no blank/framework overlay/runtime page error passed. Script/screenshots:
this chat's visualization directory, p4bc-qa.mjs and p4bc-aCAXmJ/.
Native analysis/preview also verified; the native player was observed with 52
notes, D4/D5 highlights and playback at 13.225 seconds during user interaction.
This observation does not identify which of all three modes the user auditioned.

Resource bounds: existing 100,000 notes/4,096 tracks; 21 states/onset, O(N*21²)
DP plus sorting/existing analysis/search. No Music copy per candidate. Conflict
records avoid quadratic pair expansion. CLI rejects encoded reports over 64 MiB
before writing; desktop Score 32 MiB/report 64 MiB. Encoding precedes cap check,
so transient memory can exceed report bytes. No silent truncation.

Release synthetic observation (context_bench): 200 source repetitions, 10,400
notes retained, 8,181,814 report bytes; 203 ms initial / 222 ms with polling;
observed peak working set 28,737,536 bytes. One local sample, not a guarantee.

## Human review and outstanding gates

| Mode | Recognizability | Accompaniment | Harsh intervals / jumps | Density/suitability |
|---|---|---|---|---|
| Melody first | Pending listener | Pending listener | Program: no new melody leap; human pending | Pending |
| Simple | Pending listener | Pending listener | Same sample pitches as melody-first | Pending |
| Polyphonic | Pending listener | Pending listener | 9 reported same-key overlaps; human pending | Pending |

The user was asked to audition actual files. Physical speakers are not independently
verified. Program metrics are not human evidence. Completion requires listener
feedback and PR/Linux CI. Repository instructions prohibit unrequested commit/push;
keep candidate local pending authorization. Mixed-role tracks, dense wide chords,
onset-local conditioning, limited triads, greedy reduction, uncalibrated weights,
cross-platform packaging and worst-case performance remain limitations.
No Phase 5/audio/release/deployment.

## Audition-driven revision 2

The owner auditioned `polyphonic.json` and reported: “怎么听着有点跑调”.
This is negative human feedback for the initial polyphonic candidate, not a
validated musical success. Analysis showed no chromatic pitch substitutions,
but a bass range extending to G5, 9 same-key overlaps and 17 register-crossing
events. Register confusion is a plausible explanation, not an established
psychoacoustic diagnosis of what the listener heard.

The implementation now applies the 480 bass/melody crossing penalty in all modes;
polyphonic retains its full-support policy and harmony center 69. The original
100-penalty rule and initial table are historical first-candidate evidence. The new
`target/p4bc-results/polyphonic-revision-2.json` is separate; no first candidate
or historical report was overwritten. Actual revised output: 52/52 retained,
30/30 melody, 22/22 bass, 22 octave-adjusted, zero discarded, maximum overlap 2,
bass C4–G4, zero cross-track same-key overlaps and zero bass/melody crossings.
All 29 melody intervals remain unchanged. Production validate passed.
Its note pitches/times equal melody-first on this sparse example. Different
mode labels do not require artificially different results for every input.
Revised subjective audition is still pending; the initial polyphonic result
must not be described as musically accepted. A regression asserts the explicit
bass range and zero register/key collisions for this source in every mode.

### Final local revision verification

After audition-driven pitch changes and cancellation handling, root fmt, 129
workspace tests and Clippy passed. Desktop fmt, 12 tests and Clippy passed.
TypeScript/Vite and Windows Release CLI/Tauri rebuilt successfully; executable:
`apps/desktop/src-tauri/target/release/skysheet-desktop.exe`.
The previous running audition executable was renamed within ignored release
output to `skysheet-desktop-audition-v1.exe`, preserving the user's live process.
Restart the normal executable for the latest UI/backend. No source was moved.
Browser QA repeated against the revision and passed, with evidence under
`p4bc-IMmjg0/` in the same external visualization directory. Root/desktop logs
are ignored local files `target/p4bc-*-revision-final.log`.
Source MIDI, Score/MIDI/Music production sources and lockfiles remained unchanged.
`git diff --check` passed. Worktree changes are limited to Arrange, CLI, desktop,
tests/benchmark and the requested docs. HEAD remains the Phase 4-A commit until
separate commit/push authorization. No PR/merge/release/deployment has occurred.

### Owner acceptance of revised sound

The owner listened to `polyphonic-revision-2.json` and answered:
“已改善，听感可接受”. This supersedes the initial polyphonic rejection for the
revised sample, not for arbitrary MIDI. Detailed recognizability, harsh-interval,
density and 15-key physical-playability ratings were not separately supplied;
no such ratings are invented. Melody-first/simple have identical note pitches
and times on this example, but that equality is a program fact, not separate
per-mode human testimony. Physical speaker output remains unmeasured.
The owner explicitly authorized committing, pushing this branch and opening a
draft PR against Phase 4-A for Linux CI; no merge authorization was given.
