# Phase 2: MIDI interoperability preparation

Status: the owner approved all three dependencies and the manifest diff on
2026-11-01. Implementation is now present; see [current behavior](midi.md).
The original proposal and preflight record below are retained as decision context;
the usage document takes precedence for implementation details. D6-A through D6-F remain normative in
[the score specification](music-data-format.md#16-approved-midi-timing-conversion-policy-d6).
Phase 3/4, audio transcription and automatic pitch adaptation are outside scope.

## Baseline

After `git fetch origin`, both `main` and `origin/main` were
`6a91bef810eb7523db2b1899d61d28d86d1f53af` (`feat(desktop): player`).
Created `codex/phase-2-midi` from that commit without waiting for its CI.
The only pre-existing working-tree item was untracked `audios/ode-to-joy.mid`;
it is retained without staging or modifying it. No nested AGENTS.md was found.
Historical statements that Phase 2 is unauthorized describe earlier milestones;
this task supersedes that authorization status, not the unfinished Phase 1 gates.

The local MIDI header declares Format 1, three tracks and PPQ 480. Header
inspection is not full parsing or proof that its notes fit sky-15. SHA-256:
`6e70886112270d2b68a826d8cee5283768766c97f95b54881cc29244f679dc2a`.
Use it for local acceptance; do not add it as a redistributed fixture without
establishing provenance. Generate focused fixtures in Rust for repository tests.

## Approved dependency proposal

Recommend `midly = 0.5.3`, with default features disabled and `std, strict` enabled.
It provides typed SMF headers, PPQ/SMPTE divisions, per-track delta events, tempo,
meter, channel note messages and SMF encoding. Our wrapper must still reject
Format 2/SMPTE, zero PPQ and unsupported/malformed structures, and enforce D6.
`strict` prevents the library's permissive recovery mode; it is not a substitute
for testing chunk lengths, event payload lengths, track counts, EndOfTrack,
running status and trailing data. Disable `parallel` to avoid Rayon and parallel
parser behavior. With these features midly has no external runtime dependencies.

Maintenance evidence checked on 2026-11-01: crates.io's newest non-yanked release
is 0.5.3 (2023-01-01); the upstream repository redirects from `negamartin/midly`
to `kovaxis/midly`. Its latest commit is `0f652b5` (2024-06-15). This is a mature,
slow-moving dependency, not an actively releasing project. License: Unlicense.
It is portable Rust and introduces no native library; actual platform checks
remain required. Sources: [versioned documentation](https://docs.rs/midly/0.5.3/midly/),
[registry metadata](https://crates.io/api/v1/crates/midly),
[upstream](https://github.com/kovaxis/midly).

Alternatives considered briefly:

- `rimd 0.0.1`: SMF read/write, MIT; registry release is from 2016-02-09.
  Its old published API/release baseline offers no advantage over midly here.
  [Upstream](https://github.com/RustAudio/rimd),
  [registry](https://crates.io/api/v1/crates/rimd).
- `midi-msg 0.9.0`: MIT, more recently released (2026-05-08), with SMF support
  and a broader MIDI message/extension surface. Its mandatory micromath and
  file/SysEx dependency surface are unnecessary for this focused SMF boundary.
  Prefer midly's smaller configuration; this is a scope choice, not a claim
  that midi-msg is incapable or unreliable.
  [Upstream](https://github.com/AlexCharlton/midi-msg),
  [registry](https://crates.io/api/v1/crates/midi-msg).

Also request production approval for `num-bigint = 0.4.8` and
`num-traits = 0.2.19`, both MIT OR Apache-2.0, maintained under rust-num.
Their latest non-yanked registry releases are 2026-07-05 and 2024-05-03,
respectively. Both versions already exist in the isolated verification workspace;
that does **not** authorize production use. BigUint supports exact D6 rational
arithmetic across the full binary64 exponent range without implementing our own
big-integer library. Use std only: no serde, random generation, libm or native
numeric library. Expected additional transitive packages are num-integer and
autocfg; verify and review the resolved root lockfile after approval.
Sources: [num-bigint](https://github.com/rust-num/num-bigint),
[num-bigint registry](https://crates.io/api/v1/crates/num-bigint),
[num-traits](https://github.com/rust-num/num-traits),
[num-traits registry](https://crates.io/api/v1/crates/num-traits).

Checked u128 alone can reject intermediate overflow but cannot directly represent
all D4-valid binary64 values. Big integers avoid needless arithmetic failures and
make the rounding argument auditable. Do not add num-rational: positive ratios
only need integer multiplication, division, remainder and comparison here.

Exact approved manifest changes (now applied):

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@
-members = ["crates/skysheet-score", "crates/skysheet-cli"]
+members = ["crates/skysheet-score", "crates/skysheet-cli", "crates/skysheet-midi"]
@@
 [workspace.dependencies]
 serde = { version = "1", features = ["derive"] }
 serde_json = { version = "1", features = ["raw_value"] }
+midly = { version = "=0.5.3", default-features = false, features = ["std", "strict"] }
+num-bigint = { version = "=0.4.8", default-features = false, features = ["std"] }
+num-traits = { version = "=0.2.19", default-features = false, features = ["std"] }
--- /dev/null
+++ b/crates/skysheet-midi/Cargo.toml
@@
+[package]
+name = "skysheet-midi"
+version = "0.1.0"
+edition.workspace = true
+license.workspace = true
+description = "MIDI interoperability and conversion diagnostics for SkySheet"
+
+[dependencies]
+skysheet-score = { path = "../skysheet-score" }
+serde.workspace = true
+midly.workspace = true
+num-bigint.workspace = true
+num-traits.workspace = true
--- a/crates/skysheet-cli/Cargo.toml
+++ b/crates/skysheet-cli/Cargo.toml
@@
 [dependencies]
 skysheet-score = { path = "../skysheet-score" }
+skysheet-midi = { path = "../skysheet-midi" }
```

Only the root Cargo.lock should need regeneration. Keep skysheet-score's
dependencies and N2 reader unchanged. Neither new numeric dependency enables
serde_json parser features. Keep the independent tool in its separate workspace,
with its own unchanged manifest, lockfile, Oracle, expectations and subprocess
observation boundary; sharing a numeric package version does not link the tool
to the production implementation. Add independent boundary expectations for D6,
not a call from the Oracle to the MIDI quantizer. Desktop invokes the packaged
CLI, so it needs no MIDI/numeric dependency or lockfile change.

## Planned implementation and policy

Add one reusable crate with focused source parsing, timing, import, export and
report modules. An internal MIDI source model retains source PPQ, integer absolute
ticks, track/event locations, channels, MIDI pitches, velocity and metadata.
It is distinct from Score and is not a Phase 3 music-analysis framework.
The independent report retains source context, including failed notes; Score v1,
its Schema, velocity-free notes and ordered sky-15 mapping remain unchanged.

Import policy proposal:

- Format 0 must contain exactly one track; Format 1 has a common timeline.
  Reject Format 2, SMPTE, zero PPQ and structurally unreliable files explicitly.
- Keep each SMF track as one Score track, including empty tracks, with stable
  source-order IDs. Keep channel on source events/report, not new Score fields.
  Do not infer melody/bass roles. Track names use valid UTF-8 where available;
  diagnose decoding loss and retain raw bytes in the report.
- Pair notes by `(source track, channel, MIDI key)` in source event order.
  Use FIFO for overlapping same-key onsets, diagnose the ambiguity, and retain
  each event separately. Note On velocity zero is Note Off. Report repeated
  Note On, unmatched Note Off, pending Note On at track end and zero-length
  pairs; unmatched events and nonpositive durations fail the whole conversion.
  Do not invent note ends at EndOfTrack or pair across tracks/channels.
- Source pitch remains MIDI 0..127 plus canonical SkySheet spelling. Every note
  must be in sky-15 to produce its Score; any unsupported pitch fails with its
  original pitch/location. Do not substitute a custom full-range profile,
  transpose, delete notes or select only convenient tracks.
- Merge tempo and meter maps exactly as D6 prescribes, retaining event provenance
  for conflicts and off-first-track tempo diagnostics. Compare the complete
  signature interpretation, including its notation-unit parameter. Report
  omitted click parameters; reject nonstandard 32nd-note relationships.
- Program/velocity and non-timing text metadata can be retained in the report
  with explicit loss diagnostics. Define a tested event classification table:
  controller/SysEx/tuning/pitch-bend constructs whose sounding interpretation
  cannot be preserved must fail explicitly, rather than silently claim faithful
  conversion. Do not extend note lengths by implementing a sustain-pedal engine.

Timing algorithm proposal:

For source PPQ `P`, accumulate exact weighted time
`W(s) = sum(delta_source_ticks * microseconds_per_quarter)` through source tick s.
Elapsed seconds are `W(s)/(P*1_000_000)`. Decode the actual output binary64 BPM
bits into an exact nonnegative dyadic ratio `B = bn/bd`. Then compute
`x(s) = W(s)*960*bn / (P*60_000_000*bd)` using BigUint integers.
For `n/d`, obtain quotient `q` and remainder `r`; return `q + (2*r >= d)`.
This is exact half-up rounding with no floating-point addition, segment rounding,
duration rounding or saturation. Round each endpoint, then check MAX_TICK and
positive difference before checked narrowing. A fractional endpoint produces a
quantization diagnostic, including when it rounds to zero.

Use the explicit finite positive target BPM when supplied; otherwise round
`60_000_000 / initial_source_microseconds` to binary64, then use those exact bits
everywhere. Validate that initial conversion separately. No extra BPM bounds.
Diagnose tempo flattening and discarded meter changes. Tests must distinguish
invalid metadata, arithmetic/resource failure, output-range failure and a duration
collapsed by rounding; all fail the entire conversion.

Export policy proposal:

- Validate Score first. Write Format 1 with a conductor track plus one track per
  Score track, PPQ 960, original integer note start/end ticks and track names.
- Emit nominal meter and constant tempo at tick zero. MIDI tempo is an integer
  in 1..16777215 microseconds/quarter; select nearest integer, halves upward,
  using the exact Score binary64 ratio, and report any tempo approximation.
  Reject an unrepresentable result, never clamp. MIDI numerator is one byte;
  reject Score numerators above 255. These are export limitations, not changes
  to Score validation or D6 import policy.
- Default Note On velocity 64 and Note Off velocity 0 are output policy only.
  Same-tick Note Off precedes Note On; stable source note order breaks other ties.
  EndOfTrack follows all events. Do not deduplicate identical notes.
- Allocate deterministic melodic channels, avoiding channel 10. Same-key
  overlapping voices must use distinct channels so crossing end orders remain
  unambiguous; check channel capacity across tracks and fail if exhausted.
  Do not change Score tracks to conceal protocol limits. Report channel assignment.
- Check every u7/u15/u24/u28 field before library constructors, track-count u16,
  and chunk-length u32. Reject delta gaps above the SMF four-byte VLQ maximum;
  no truncation, wrapping or silent insertion of sounding events.
- MIDI cannot preserve arbitrary binary64 tempo exactly, source tempo/meter
  maps, Score IDs, pitch spelling aliases or instrument behavior. Document and
  test these limits; do not promise byte-identical or unrestricted round trips.

## CLI and desktop integration

Extend existing CLI argument parsing and result handling with `import-midi` and
`export-midi`, required `-o/--output`, and machine-readable diagnostics. Include
an explicit target-reference-tempo option for D6-A. Keep old inspect/validate
results unchanged and retain exit categories: 0 success, 1 invalid/unconvertible
input, 2 usage, 3 I/O, 4 internal error. The conversion report is separate from
Score JSON and available on failure as well as success. An inspect-midi command
is secondary, only if it reuses the source scanner without substantial new scope.

Complete conversion, Score validation and serialization before creating the final
output. Reject existing destinations by default, including source aliases; test
same-path/hard-link cases, write failures and cleanup. Do not truncate inputs or
leave a failed conversion looking successful. Use structured CLI responses for
the GUI; diagnostic text must never be mixed into a saved Score.

Extend `apps/desktop/src-tauri/src/main.rs` and snapshot helpers to select mid/midi,
invoke the packaged CLI via argument arrays into a unique temporary directory,
inspect status/report, and send generated bytes through existing `load_bytes`.
Always clean up task-owned temporary files. Return cancellation without replacing
the Score. Add service/types and an Open MIDI action/report view in `App.tsx`.
Its existing load transaction calls `prepareScore` before replacing engine/song;
preserve this success-only replacement and stale-request protection. Reuse the
existing piano roll, keyboard, synthesizer and transport. Conversion failures must
leave the old Score visible and provide a readable error and expandable report.

## Verification plan after approval

Use Rust-generated binary fixtures and explicit expected outcomes, including:

- Format 0/1, running status, channels, simultaneous chords, duplicate notes,
  FIFO overlaps, retriggers, missing/on-zero/off events, track names and metadata.
- Malformed chunks/events, Format 2/SMPTE/zero PPQ, trailing garbage, missing EOT,
  unsupported events and pitches; no partial successful Score or output files.
- Every section 16.7 D6 example, exact ties and neighbors, M/M+0.49/M+0.5,
  extreme binary64 BPMs, collapsed durations, leading silence, crossing tempo
  changes, event-order/track-order independence and equal-time cross-track notes.
- Export byte structure and reparse, tempo approximation/range, meter range,
  multiple tracks/channels, same-tick ordering, crossing same-pitch note ends,
  channel exhaustion, delta/chunk limits and retained input files.
- CLI raw JSON/N2 validation, Unicode/space paths, stable JSON responses and exit
  codes, failed conversion/write cleanup. Local ode-to-joy import must either
  validate/play successfully or report its real unsupported content without repair.
- Desktop native file selection, cancel, success, failure preserving the old Score,
  visible diagnostics, temporary cleanup and existing playback/seek/highlights.

Run root fmt/tests/Clippy; MIDI timing tests in debug and release; independent
workspace fmt/tests/Clippy and full numeric gate; frontend tests/build; desktop
Rust checks and rebuilt Windows sidecar/application. Load relevant frontend and
computer-use skills when implementation/visual verification starts. Record actual
results and limitations; do not mark Phase 2 complete until acceptance is observed.

## Preflight verification observed

Before production edits, on the branch above:

- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace --locked`: 64 tests passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --manifest-path tools/numeric-validation/Cargo.toml --all -- --check`: passed.
- `cargo test --manifest-path tools/numeric-validation/Cargo.toml --workspace --locked`:
  20 tests passed, including the separate N2 regression corpus.
- `cargo clippy --manifest-path tools/numeric-validation/Cargo.toml --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked -- --report target/phase-02-preflight-numeric-20261101.json`:
  267 fixed cases and 10,000 samples passed, zero failing records, production Rust checked.

Clippy emitted incremental-cache finalization notes (Windows access denied) in
both workspaces but exited successfully; this may prevent cache reuse on the next
build. No manifest, lockfile, Score implementation or desktop source was changed
for this preparation. No MIDI implementation, desktop retest, commit or push is
claimed at this approval checkpoint.
