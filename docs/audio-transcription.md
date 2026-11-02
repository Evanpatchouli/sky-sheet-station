# Audio Transcription (AT-1 / AT-2 / AT-3)

Status: implemented candidate awaiting human acceptance. The baseline is main
`790c31e`, after owner-authorized PR #2 integrated the existing MIDI, Music and
Phase 4-A/B/C work. AT remains a separate feature branch and Draft PR.

## Use

```text
cargo build -p skysheet-cli --release --locked
skysheet transcribe input.mp3 -o transcription.mid --json --progress
skysheet analyze transcription.mid --json
skysheet arrange transcription.mid -o sky.json --auto --json
skysheet validate sky.json --json
skysheet export-midi sky.json -o sky.mid --json
```

The first command writes the unrestricted transcription MIDI and
`transcription.mid.transcription.json`. `--report <PATH>` overrides the report
destination. Both paths must be new, distinct files. Existing files, source aliases
and missing parents fail; failed pair writes clean up only newly reserved files.
Without `--progress`, JSON mode has one stdout response and no stderr progress.
With it, stderr contains JSON lines `{stage, completed, total}`; `total: null`
means indeterminate work, not an invented percentage. Exit codes remain 0 success,
1 invalid/untranscribable input, 2 usage, 3 I/O, 4 internal failure.

Optional controls, also available in the desktop dialog:

| Option | Default | Accepted range |
|---|---:|---:|
| `--onset-threshold` | 0.5 | 0.05–0.95 |
| `--frame-threshold` | 0.3 | 0.05–0.95 |
| `--minimum-note-ms` | 127.7 | 30–1000 ms |

Lower thresholds increase sensitivity and false positives. The minimum length
uses Basic Pitch's frame-based rule; this is an estimate, not exact sample-level
onset/offset detection. Build in release mode for interactive transcription.

Desktop: **导入音乐 → 选择音频 → 设置参数 → 生成乐谱 → 可视化编辑与试听**.
The single generation dialog runs transcription, analysis and arrangement
automatically. Thresholds are under advanced settings. Original reports and
transcription MIDI export are under **生成详情**; **另存乐谱** saves the current
edited score. See [editing controls](desktop-player.md#import-and-visual-editing-2026-11-03).
The MIDI save action preserves unrestricted notes and velocities; original
arrangement losses remain available in Report v2.

Cancellation kills the current sidecar and ignores late results. Picker
cancellation, failure and cancellation preserve the loaded Score and previous
successful score. Transcription snapshots survive original-file edits; save and
arrangement use backend-owned tokens. Generation completes directly in the editor;
manual edits do not change the original reports.

## AT-1: decoding and preprocessing

`skysheet-audio` owns decoding. Symphonia 0.5.5 is pinned with only MP3, WAV/PCM
and FLAC enabled. Files must be regular, nonempty, at most 128 MiB, with at most
600 seconds of decoded samples, 1–8 channels and 8–192 kHz source sample rate.
These are product resource limits, not Score/MIDI format rules. Unsupported
containers/codecs and corrupt packets fail explicitly. A declared frame count
more than 4096 samples beyond decoded output fails as truncated; not every
truncated MP3 without trustworthy length metadata is detectable.

Channels are averaged to mono. Rubato 0.16.2's FFT resampler provides anti-alias
filtering at 22050 Hz; its delay is removed and its tail flushed. The output length
is `ceil(source_frames * 22050 / source_rate)`. Leading silence is retained.
There is no peak normalization (the model depends on absolute signal level).
Opposite-phase stereo can cancel and is disclosed as a limitation.

## AT-2: actual local polyphonic inference

`basic-pitch-rs = 0.1.0` runs Spotify's unmodified Basic Pitch v0.4.0 `nmp.onnx`
using the Rust tract backend. The approximately 230 kB model is embedded in the
CLI by the dependency: no Python, C++ inference runtime, server, credentials,
runtime download or external decoder is needed. The fixed model SHA-256 is
`2c3c1d144bfa61ad236e92e169c13535c880469a12a047d4e73451f2c059a0ec`.

The graph supplies note, onset and contour activations in overlapping windows.
Up to four CPU workers infer independent windows. Upstream overlap stitching
and note decoding join sustained events across windows. `TrimMode::Main` retains
tail frames (the older release trimming convention can lose time near EOF).
Pitch bends are disabled, avoiding incompatible channel pitch-wheel semantics
in the existing MIDI reader. The supported model pitch range is A0–C8 (21–108).

Estimated velocity is upstream `round_ties_even(127 * mean_note_activation)`,
bounded to 1–127. Activation is recorded separately and is neither a calibrated
probability nor physical key force. Onset/offset estimates are clipped to the
decoded duration; out-of-range/zero-resolution events are counted in diagnostics.
Same-pitch overlaps are shortened at the next retrigger (and zero-length
duplicates removed), with a diagnostic count, so SMF note pairing is unambiguous.
No instrument separation, semantic instrument identity or percussion tracks are
claimed. Silence yields a valid empty MIDI and explicit no-notes diagnostic.

Choice and alternatives: Basic Pitch provides a documented multipitch neural
baseline, unlike a single-frequency tracker. The Rust port is new (0.1.0), so its
version and model are pinned and real inference is tested locally/in CI. tract
avoids an ONNX Runtime binary/distribution dependency; ONNX Runtime could offer
different performance but is not needed here. Symphonia/Rubato are established
Rust audio libraries; known 0.5/0.16 APIs are pinned instead of adopting major API
changes during integration. Core dependencies support native Windows/Linux; this
does not expand the desktop's existing Windows-only verification policy.

Primary references: [Spotify Basic Pitch](https://github.com/spotify/basic-pitch),
[paper](https://arxiv.org/abs/2203.09893),
[Rust port](https://github.com/AThevon/basic-pitch-rs),
[Symphonia](https://github.com/pdeljanov/Symphonia),
[Rubato](https://github.com/HEnquist/rubato).
Basic Pitch/port/tract use Apache-2.0 or MIT/Apache-2.0; Symphonia uses MPL-2.0;
Rubato 0.16.2 uses MIT. See [distribution notices](../third-party/audio/README.md).

## AT-3: MIDI and existing product flow

Audio timing has a documented independent grid: 960 PPQ at reference 120 BPM,
nominal 4/4. Neither is claimed to be an estimated musical beat/meter. Model
seconds are rounded to absolute 22050-Hz sample endpoints, then converted with
integer half-up rounding: `tick = (sample * 1920 + 11025) / 22050`. Endpoints are
rounded independently; durations are positive differences. This is not a change
to D6. The generated single-track SMF carries note-on velocities and note-offs,
with note-off before same-tick retrigger, tempo and nominal meter metadata.

The CLI validates those exact MIDI bytes with `skysheet_midi::read_music` before
writing. The desktop invokes the packaged CLI for both transcription and analysis;
it does not link the model or implement transcription logic. Arrangement continues
to use `project_music` and the existing D6 engine and Phase 4-B/C algorithms.
Score v1, Schema, fixtures, pitches, ordered keys, N2 parser/features and original
MIDI behavior are unchanged. Velocity/provenance remain outside exported Score.

## Evidence and limitations

Automated reference audio is synthesized in Rust (not mocked model output).
It checks A4 and a C5/E5 chord with leading silence, onset error <150 ms and
offset error <250 ms, velocity bounds, actual MIDI readback, four sample rates,
stereo folding, silence, damaged/truncated/missing inputs, cancellation and
invalid options. CLI integration exercises real inference through analysis,
arrangement, Score validation and MIDI export, with no partial-output/overwrite
regressions. Desktop protocol/save tests and frontend parameter/progress tests
complement native UI checks. These small references are regression gates, not
an accuracy claim over arbitrary recordings.

Local Windows Release observations (one run, compilation excluded):

| Owner-provided file | Decoded length | Candidate notes | Transcription wall time | Downstream |
|---|---:|---:|---:|---|
| `Scarlxrd-Ace - 小星星(钢琴版).mp3` | 103.741 s | 676 | 1.451 s | arrange + production validate passed |
| `Donk小曲.mp3` | 300.048 s | 2717 | 3.615 s | arrange + production validate passed |

These files are unchanged. They have no note-level ground truth; counts and
runtime demonstrate real execution, not precision/recall or perceptual quality.
Dense mixtures, drums, reverb, vocals, very quiet audio and unusual tuning can
cause false positives, omissions and octave errors. Stereo phase cancellation,
short-note filtering, estimated velocity and onset resolution remain limitations.
The default 120 BPM timeline is not beat-quantized notation. Human audition and
acceptance of representative music remain required; no general transcription
accuracy, source separation or finished Phase 1 contract freeze is claimed.

## Engineering and native verification

- Root `cargo fmt --all -- --check`, `cargo test --workspace --locked` (138 Windows
  tests) and `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Isolated desktop fmt, 14 Rust tests and Clippy passed. Frontend 11 tests,
  TypeScript/Vite production build, packaged sidecar and Windows Tauri Release
  build passed. The model checksum was independently read from the installed
  dependency and matches the report. License resources are present in the build.
- Isolated numeric fmt, 20 tests, Clippy and full gate passed: 267 fixed cases,
  10000 historical samples, zero failures. The focused 84 and research 3016
  cases also passed. Independent lockfile and parser-feature restrictions remain
  unchanged.
- Native Release at `http://tauri.localhost/`, approximately 1280×880 content:
  selected the Unicode/space piano MP3 in the Windows picker, observed real
  progress and 676 note rows, saved MIDI/report, entered the existing arrangement
  dialog, generated 635-note Score, saved Score/report and loaded the player.
  Playback advanced to 18 seconds with C4/A4/E5 highlights, then paused. The saved
  Score passed the production CLI validator. Saved MIDI SHA-256 matches the
  standalone CLI output: `edddda914322d1f2d52c75336bf38b4354ee5faba4cde8c0705a8fa82709ec37`.
- Native complex-MP3 transcription displayed 2717 notes. Invalid MP3 displayed a
  decoding error and retained that result and the previously loaded 635-note
  Score. A Rust-generated 590-second stereo WAV completed as one sustained A4
  (0.023–589.993 s), additionally checking window joining and the long-file tail.
- The native cancel click attempts occurred after these fast transcriptions had
  already completed; they are not claimed as mid-inference cancellation evidence.
  Actual cancellation during inference is covered by the Rust observer test;
  native mid-job cancellation timing remains a manual acceptance check.
- Native screenshots show a populated interface without a framework overlay,
  legible reports and working primary actions. Physical loudspeaker output,
  musical quality ratings, additional desktop sizes and console capture were not
  independently measured. The old keyboard-panel audio-placeholder caption found
  during QA was corrected.
- [Linux CI for implementation `4e702e4`](https://github.com/Evanpatchouli/sky-sheet-station/actions/runs/41326305238)
  passed core/CLI tests, fmt/Clippy, independent numeric checks and the added
  Node frontend test/build job. Final-head CI and acceptance status are maintained
  in [Draft PR #3](https://github.com/Evanpatchouli/sky-sheet-station/pull/3).
