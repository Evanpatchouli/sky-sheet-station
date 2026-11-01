# MIDI interoperability (Phase 2)

Phase 2 adds a reusable `skysheet-midi` crate, CLI conversion and MIDI opening in
the existing Windows desktop player. It does not add audio transcription, pitch
adaptation, music analysis or a new Score format. See the
[dependency/design record](phase-2-midi.md) and [D6 policy](music-data-format.md#16-approved-midi-timing-conversion-policy-d6).

## Commands

```powershell
cargo build -p skysheet-cli --release --locked
.\target\release\skysheet.exe import-midi input.mid -o output.json
.\target\release\skysheet.exe import-midi input.mid -o output.json --tempo 60 --json
.\target\release\skysheet.exe export-midi input.json -o output.mid --json
.\target\release\skysheet.exe inspect-midi input.mid --json
```

`-o`/`--output` is mandatory for conversion. Output must be a new file: existing
destinations, including source aliases and hard links, are refused. Input files
are never rewritten. Conversion and encoding finish before output creation;
failed writes attempt removal of their own partial output. A cleanup failure is
reported. A process/OS crash can still leave a partial output file.

`--tempo` is a finite positive binary64 reference BPM, only for import. It changes
the output timing grid, not source elapsed time. Omission uses effective source
tempo at tick zero. No additional musical BPM bounds are imposed.

Exit codes retain the Alpha categories: 0 success; 1 invalid/unconvertible input;
2 usage; 3 I/O; 4 internal error. JSON mode emits one result document and no
ordinary stderr output. Existing inspect/validate output fields are unchanged.
Conversion success adds `result.output_path` and `result.report`; conversion
failure includes `error.report` when available. Score JSON contains no report.
Human mode prints diagnostics with severity, code and source location.

`inspect-midi` parses reliable SMF framing, pairs source notes and returns its
report without constructing a constrained Score. Semantic diagnostics do not
make structural inspection fail; exit 0 does not certify importability. Full D6
map conflict and target-pitch checks run during `import-midi`.

## Supported inputs and deterministic pairing

- Standard MIDI Files Format 0 (one track) and Format 1, with positive PPQ.
  Format 2, SMPTE division, RIFF/container input and unreliable framing are
  rejected explicitly. A framing precheck complements midly's strict parser:
  bounded VLQs, exact known metadata lengths, declared track counts, data-byte
  ranges, one terminal EndOfTrack and no trailing bytes.
- Each source track remains one Score track, including empty tracks. Channel
  is retained in source notes/report, not added to Score. Track role is unknown;
  no melody/bass inference is performed. Generated IDs follow source order.
- Pair by `(track, channel, key)` in event order; same-key overlaps use FIFO
  with a warning. Note On velocity zero is Note Off. Retriggers/chords/repeated
  notes remain independent. Unmatched off, missing off and nonpositive source
  duration fail the entire import. No cross-track pairing or invented endings.
- Source MIDI key, canonical pitch, velocity, event indices and source endpoint
  ticks remain in the report. Notes must all belong to sky-15's unchanged logical
  C4–C6 natural-note mapping. Unsupported pitches fail, with no deletion,
  transposition, filtering or full-range profile substitution.
- Track names use the first nonempty name event; invalid UTF-8 is displayed with
  replacement characters and diagnosed, with original bytes retained. Text,
  copyright, instrument names, lyrics, markers and program changes are preserved
  in the report; lyrics are not recognized, segmented or inserted into Score text.
  Ordinary informational metadata is diagnosed as report-only.
- Controllers (including sustain), pitch bend, aftertouch, SysEx/escape,
  sequencer-specific/unknown metadata, port routing and SMPTE offset are rejected
  because this implementation cannot preserve their interpretation. There is no
  controller/synthesizer emulation in the core. Velocity and source program timbre
  are diagnosed losses; preview uses the target instrument's local synthesis.

## D6 timing

Scan all tracks for global tempo/meter maps. Identical same-tick events merge;
conflicts fail with both locations. Signature comparison includes click and
notation-unit parameters. Tempo outside track zero is retained and diagnosed.
Default source tempo is 500000 microseconds/quarter until the first explicit
event; explicit tick-zero metadata overrides the implicit default. Zero tempo
fails. Default meter is 4/4, overridden only by an explicit initial signature.
Retain the initial nominal meter; diagnose discarded later signatures and click
parameters. Later source-valid large denominators are report-only; an initial
denominator above 64 fails. Nonstandard notation units and zero numerators fail.

For source tick `s`, accumulate `W(s) = sum(delta_ticks * source_us_per_quarter)`
as BigUint. If actual written binary64 BPM is `bn/bd`, the exact target position is
`W(s)*960*bn / (source_ppq*60_000_000*bd)`. Binary64 is decomposed from its bits,
including subnormals. Round by integer quotient plus one when twice the remainder
is at least the denominator. No rounded segment accumulation, floating seconds,
duration rounding or leading-silence removal occurs. Round start/end separately,
then require positive difference and the existing MAX_TICK bounds.

The default BPM calculation divides two exactly representable integers; tests
certify sampled results against exact neighboring-binary64 midpoints. Quantizer
tests include independent bounded-u128 expectations, half ties/neighbors, the
MAX_TICK edge, extreme tempos and full MIDI map fixtures. This is separate from
the unchanged independent N2 JSON gate. Any failed note fails the entire import.

## MIDI output policy and limits

Validated Score is encoded as Format 1, PPQ 960, with a conductor track followed
by one track per Score track. Integer note endpoints are preserved. Tempo/meter
occur at conductor tick zero. Tempo becomes nearest integer microseconds/quarter,
half up, using exact arithmetic on the actual Score BPM; approximation is reported.
Reject results outside 1..16777215 rather than clamp. MIDI meter numerator is
limited to 255, even though larger numerators can be valid Score values.

Note On velocity is 64; Note Off velocity is 0. These values are MIDI output
policy, not Score fields. Same-tick off precedes on within a track, with stable
note order for ties; EOT is last. Deterministic melodic channels avoid channel 10.
A channel/key pair belongs to one Score track for the entire file, and overlapping
unisons use distinct channels. This prevents both crossing-note ambiguity and
cross-track same-tick off/on merge hazards. Exhausting the 15 melodic channels
for a key fails explicitly, even for sequential notes on too many different tracks.

Check all protocol widths before constructing midly numeric types. Reject any
delta above 268435455 ticks, too many tracks (including conductor), and oversized
encoding. No truncation, saturation or filler events. The converter currently
limits input to 32 MiB and conservatively budgets output at 32 MiB; these are
implementation resource limits, not new Score validity rules. Within that budget,
chunk lengths fit u32 and metadata lengths fit u28.

Round trips preserve representable pitches and tick endpoints, not original MIDI
bytes, tempo/meter maps, Score IDs, pitch spelling aliases, text, track roles or
instrument behavior. An exported file has one additional conductor track on
reimport. Tempo approximation can slightly change elapsed seconds. Do not claim
lossless MIDI reconstruction.

## Desktop

Use **打开 MIDI**, select `.mid`/`.midi`, then play with the existing controls.
The backend snapshots the source, runs the packaged `import-midi --json` command
into a private temporary output, checks the response and passes generated bytes
through the existing raw-Score/CLI validation loader. It does not parse MIDI.
Success replaces the player contents only after preparation; cancel or failure
preserves the previous Score, paused. **导入报告** displays source counts, tempo
and meter events, quantization and rejection reasons, including full source notes.
Temporary files are removed on normal success/failure; crashes can leave snapshots.

`audios/ode-to-joy.mid` is useful as a real rejection case: 3 tracks, PPQ 480,
52 paired notes, of which 22 contain unsupported C3/G3/G2/D3 accompaniment pitches.
It must fail intact. A supported MIDI fixture is used separately to verify the
successful native load/play path; the supplied file is never modified or filtered.

## Verification observed on Windows, 2026-11-01

| Check | Result |
|---|---|
| Root `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed; 84 tests |
| `cargo test -p skysheet-midi --release --locked` | Passed; 17 tests (4 numerical unit tests and 13 SMF integration tests) |
| Desktop manifest fmt, tests and Clippy with warnings denied | Passed; 7 tests, including real packaged CLI conversion and N2 cases |
| Independent tool manifest fmt/tests/Clippy and full `cargo run ... --locked -- --report target/phase-02-midi-numeric-20261101.json` | Passed; 20 tests, 267 fixed + 10,000 sampled cases, zero failing records |
| `npm test`, `npm run build`, `npm run desktop:build` from apps/desktop | Passed; 8 playback tests, TypeScript/Vite and Windows local executable |
| Native release UI at `http://tauri.localhost/` | Actual picker loaded `可演奏 & 测试.mid`; 3 tracks/6 notes, 15 keys, pointer advancement and sounding-key highlights observed |
| Supplied MIDI | CLI exit 1, 22 unsupported-pitch errors, no output file; native UI retained the prior 6-note Score and exposed the report |
| Local Edge/Playwright at `http://127.0.0.1:1420/` | Real CLI-produced Score/report with test-only mocked IPC: import, play/highlights, pause, report, cancel/error retention and 22 error entries passed |
| Layout | 1280x880, 800x880 and 390x880 inspected; new narrow-toolbar overflow corrected; document widths match viewport widths |
| Browser health | Correct title/nonblank page, no framework overlay or script exceptions; only pre-existing missing `/favicon.ico` 404 in development preview |

The Browser runtime skill was not available; cached local Playwright 1.63/Edge
was used without adding project dependencies. The browser mock tests rendering;
native picker/sidecar verification establishes the real bridge separately.
Temporary QA script/screenshots are outside tracked source. No hosted CI run,
Linux MIDI validation or physical speaker/acoustic measurement is claimed.
Windows emitted incremental-cache access-denied notes during checks, which still
passed. The first release rebuild hit the old running executable's file lock;
closing that window and rebuilding succeeded.

The production Score source, Schema/example and N2 parser features are unchanged.
Both isolated workspaces' lockfiles and the frontend package lock are unchanged.
Root Cargo.lock adds only the approved dependencies and num-integer 0.1.47 /
autocfg 1.5.1 transitives. No Phase 3/4 work, commit, push or deployment occurred.
