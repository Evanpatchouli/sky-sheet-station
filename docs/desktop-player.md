# SkySheet Desktop Player (Phase 6)

The owner authorized Phase 6 on 2026-11-01 before audio/MIDI conversion was
implemented. This is a Windows desktop **Score visualization and local audition
client**, not a transcription engine or game-input player. Phase 2 subsequently
adds MIDI import through the packaged CLI; see [MIDI usage](midi.md).
Score format and pitch mapping remain unchanged. AT-1/2/3 now adds offline audio
transcription through the CLI; see [Audio Transcription](audio-transcription.md).
Phase 4-A/B/C provides the arrangement flow below.

## Import and visual editing (2026-11-03)

The desktop entry is now **导入音乐** for both audio and MIDI:

1. Select a MIDI, MP3, WAV or FLAC file in the single generation dialog.
2. Choose melody-first (default), simple accompaniment or polyphonic arrangement.
   Transposition, priorities, polyphony and audio thresholds are in **高级设置**.
3. Click **生成乐谱**. The desktop invokes the existing CLI transcription (audio
   only), analysis and arrangement pipeline, then installs the validated result
   directly in the editor. There is no intermediate MIDI, preview or load button.
4. Edit and audition the score. **另存乐谱** saves the current document to a new
   JSON file after the existing raw-byte reader and packaged CLI validate it.

**生成详情** retains source analysis, transcription diagnostics and the original
arrangement report. Those reports describe generation, not subsequent manual
edits. Saving the original score/report pair is disabled after manual editing;
use **另存乐谱** for the edited version. Original transcription MIDI/report export
remains available in the details view.

Cancellation and failures preserve the current document. Request identities
reject late generation results. Audio cancellation retains the existing process
termination behavior; MIDI analysis/arrangement cancellation invalidates results
while the temporary CLI computation may finish. Native selection tokens must
match the selected file and input kind; the frontend cannot choose arbitrary
paths through these generation parameters. File contents are snapshotted when
processing begins, using the existing bounded readers.

### Editing controls

- The smallest editing cell defaults to a sixteenth note (240 ticks), with
  quarter/eighth/thirty-second/sixty-fourth alternatives. This is an editing
  preference, not a change to Score's one-tick resolution or MIDI conversion.
- Click a blank grid cell or ruler to position the insertion line, then click an
  instrument key to toggle that pitch in the selected track. An empty cell gets
  a one-cell note; an occupied cell removes the entire intersecting note, even
  if it sustains across cells or starts off-grid. Other tracks are unchanged;
  legacy overlaps in the same track are removed one note per click. Outlined
  keys indicate pitches occupying the current edit cell. Both actions can be undone.
- Select a note, drag its body to move its time/pitch, or drag either edge to
  resize it. Wheel up/down over the selected note lengthens/shortens it by a
  cell. Resize durations are positive cell multiples; the opposite edge stays
  anchored. A sub-cell imported note at the beginning may have no room to resize
  its left edge; resize its right edge instead.
- Del deletes the selected note. The shared icon/text context menu supplies
  Copy/Delete on notes and Paste on blank cells. Ctrl+C copies the selected note;
  Ctrl+V over the roll pastes at the hovered cell start, retaining pitch and
  using the current edit track. Clipboard state is internal to this document.
- New or edited notes cannot overlap another note of the exact same key/pitch,
  including other tracks. Touching endpoints and different-pitch chords are
  allowed. Invalid drags are translucent with a not-allowed cursor and revert
  on release. Existing imported overlaps remain intact until edited.
- Imported timing is unchanged on load. Moving/pasting snaps starts and lengths;
  resizing snaps duration while retaining the opposite edge. Pitch spellings,
  ordered keys, metadata, tempo and user text are preserved.
- Undo/redo retains up to 100 recent edits (Ctrl+Z / Ctrl+Y or Ctrl+Shift+Z).
  Editing pauses audition and rebuilds its note index. Scrolling and zoom remain
  available, with trailing space for adding notes after the original end.
- Opening another document prompts to save/discard unsaved work. Saving always
  uses a new file and never overwrites an existing destination. Browser reload
  protection uses beforeunload; native window-close behavior still needs manual
  verification. No autosave or persistent undo/clipboard is implemented.

Score v1, CLI commands, MIDI compatibility, instrument mapping and production
parser/dependency features are unchanged. Editing guards constrain editor
operations; Rust remains the authority for file validation. No transcription or
arrangement algorithm is reimplemented in React or Tauri.

## Run on Windows

Prerequisites: Node.js 22.18+ (Node's built-in TypeScript stripping is used for
tests), npm, edition-2024 Rust, MSVC build tools, and the Microsoft Edge WebView2
runtime. The build helper currently targets `x86_64-pc-windows-msvc` only.

From the repository root:

```powershell
cd apps/desktop
npm ci
npm run desktop
```

This builds the existing CLI in release mode, copies it to Tauri's target-suffixed
sidecar path, starts Vite on `127.0.0.1:1420`, and launches the native application.
Vite excludes `src-tauri` from its watcher so Windows cannot fail watching locked
Cargo executables. `npm run dev` alone is a browser UI preview; it does not bypass
CLI validation or load unvalidated files.

For an offline local executable (no installer or publication):

```powershell
npm run desktop:build
```

Keep `apps/desktop/src-tauri/target/release/skysheet-desktop.exe` and its sibling
`skysheet.exe` together. The frontend is embedded in the desktop executable; the
release application does not require Node or a development server. These build
outputs are ignored. Installer distribution, signing and publishing are outside
this delivery.

## Playback and existing files

- **打开乐谱** opens Score JSON through native selection and CLI validation.
  **打开内置示例** loads the checked-in six-note example through the same path.
- Notes are colored by track; pitch rows, nominal bar/beat guides, zoom,
  horizontal scrolling and follow-playhead remain supported.
- Keyboard order derives from instrument.keys. Play, pause, stop, progress,
  speed, loop and volume controls provide local synthesized audition.
- Main-toolbar **打开 MIDI**, **分析 MIDI** and **Sky 编曲** have been replaced by
  the unified generation flow. Low-level import/analysis CLI behavior is unchanged.

## Boundaries and architecture

`React → narrow Tauri commands → native file selection / CLI sidecar → Score`.
The Rust backend reads original bytes into a private temporary snapshot, invokes
the official Tauri shell plugin with an argument array (`validate --json -- PATH`),
checks the structured result and exit code, and reads those same bytes directly
with `skysheet_score::Score`. It does not duplicate validation or tempo parsing.
The snapshot is removed on success/error; the source is never rewritten. A crash
can leave a temporary snapshot in the OS temporary directory.

The IPC projection adds source display text and MIDI numbers derived from the
existing `Pitch::midi_number()` API. These are not exported Score fields. The
frontend never parses pitch strings, raw JSON, or original tempo tokens. The
existing N2 raw-byte adapter remains in use. All 84 frozen tempo fixtures are
also checked through the real sidecar and desktop projection.

Application commands handle Score opening/saving, native music selection,
MIDI analysis/arrangement, audio transcription and report exports. Selection and
save destinations use native dialogs. No executable names or command strings
come from JavaScript. Shell/dialog plugins have no frontend permissions.
The CSP permits local application resources and IPC, with no remote application
content. All processing is local; no cloud, credentials, telemetry or service is
introduced. See Tauri's [sidecar](https://v2.tauri.app/develop/sidecar/) and
[dialog](https://v2.tauri.app/plugin/dialog/) documentation.

The desktop Rust package is an **isolated workspace** with its own lockfile.
This prevents Tauri dependencies from unifying parser features with the CLI and
the independent numeric verification gate. The root workspace contains CLI/Score/MIDI.
Both workspaces use the same checked-in Score source.

### Local playback policy

Web Audio's audio-context clock is the single playback clock. A 25 ms scheduler
queues oscillators up to 120 ms ahead; the pointer, progress and active keys read
that same clock. Pause, stop, seek and speed changes cancel queued voices and
re-anchor timing. Browser/audio suspension pauses playback. Seeking into a note
restarts its local audition voice for the remaining musical duration. Speed
changes preserve frequency. Looping resumes at the elapsed overshoot after the
last note end; it is not a sample-perfect audio loop editor.

Triangle-wave synthesis is an intentional local preview voice, not a sampled Sky
instrument or a claim about actual game tuning. `trigger` uses a decaying
envelope and `sustain` holds until note end. Visual highlights always describe
the full note interval `[start, end)`. No MIDI velocity is introduced. No OS key
events, game automation or scheduling logic is added to the Rust core.

Nominal beat ticks are `PPQ * 4 / denominator`; nominal bar ticks multiply that
by the numerator. This does not recover original bars from tempo-flattened MIDI
or introduce automatic segmentation.

### Limits

- Desktop loading has an explicit 32 MiB file limit; this is not a Score-format
  restriction. All parsed notes remain in memory. Canvas queries a tick interval
  index and draws only the visible region rather than creating a DOM node per note.
- Unrepresentable/nonfinite playback seconds, collapsed note intervals and
  durations beyond exact whole-second precision disable audition with an explicit
  message. Valid Scores can still be viewed in ticks; BPM is never clamped.
- Empty tracks/scores are displayable; playback is disabled without notes.
- More than three simultaneous tracks at the same pitch can visually overlap;
  each note is still drawn and scheduled. Very long scores use a bounded scroll
  surface mapped to the full tick range. No arbitrary note truncation is used.
- Timing is subject to WebView/audio-device scheduling. No latency, large-score
  throughput or acoustic fidelity guarantee is claimed. Key flashes shorter
  than a display refresh can be visually missed, though audio is scheduled.
- No editing, saving, MIDI, audio transcription or conversion is added here.

## Dependencies

The owner's requested stack authorizes Tauri, React, TypeScript and Vite. The
only added native integration plugins are the maintained official Tauri 2 shell
and dialog plugins (MIT/Apache-2.0); they supply the supported sidecar/file-picker
APIs instead of handwritten Windows FFI or a general frontend shell. React is
MIT; TypeScript is Apache-2.0; Vite and its React plugin are MIT. Exact resolved
versions are pinned in the desktop npm and Cargo lockfiles. Windows x64 is the
verified target; upstream cross-platform support is not a claim of project support.
Web Audio and Canvas add no runtime packages.

## Verification commands

```powershell
# Repository root, independently from the desktop workspace
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings

# Native desktop checks require the prepared CLI binary
npm --prefix apps/desktop run sidecar
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- -D warnings
npm --prefix apps/desktop test
npm --prefix apps/desktop run build
```

Run the separate tool checks and full gate in
[numeric verification](numeric-verification.md#commands-and-ci). Root checks do
not cover either isolated workspace. Native UI checks must exercise actual
sidecar invocation; browser tests with mocked IPC only verify frontend behavior.

## Local verification (2026-11-01)

Observed on Windows x64, Rust 1.99.1, Node 26.3.0, npm 11.16.0, Tauri 2.12.5:

| Check | Observed result |
| --- | --- |
| Root format / tests / Clippy | Passed; 64 tests; production source and lockfile unchanged |
| Desktop Rust format / tests / Clippy | Passed; 5 tests, including actual CLI execution, original-token rejection, Unicode/spaces/metacharacters, snapshot cleanup and all 84 frozen N2 cases |
| Independent numeric tool format / tests / Clippy | Passed; 20 tests including research and focused N2 regressions |
| Full independent gate | Passed; 267 fixed cases, 10,000 samples, zero failing case records |
| Frontend typecheck / Vite build / Node tests | Passed; 8 tests covering clocks, cancellation, pause, seek, speed, loop, overlap, chords, meter and custom keys |
| Windows release build | `npm run desktop:build` passed; desktop and CLI executables exist together |
| Native Release workflow | Developer server stopped; embedded `http://tauri.localhost/` UI opened, native picker loaded a real fixture copy named `乐谱 空格.json`, metadata preserved its source, actual sidecar validated it, playback advanced progress/pointer and lit the active G4 key |
| Browser UI workflow | Local Playwright/Edge: title, nonblank UI, no page exceptions, sample keys, chords, pause freeze, stop, seek, speed, loop, metadata, volume, unavailable audio conversion and CLI-error retention passed |
| Larger test fixture | 600 synthetic repetitions of the repository events, explicitly test data and validated by the CLI: all events retained; horizontal scroll, zoom and follow on/off passed |
| Layout | 1280×880 desktop, 1536×1024 concept size and 390×844 narrow view inspected; fixed keyboard clipping and narrow-view transport overlap |

The remote Browser Use tool returned connection refused; a local browser was
used instead. The first local Vite process had exited on a locked Cargo output;
excluding `src-tauri` resolved that failure. Native Windows UI automation verified
the real bridge separately from the browser's test-only mocked IPC. No mock
bridge or test-generated score is included in production. Physical speaker
output and game-acoustic similarity were not measured.

Visual review compared the generated concept and rendered screenshots directly:

| Element | Implementation / intentional adjustment |
| --- | --- |
| Layout | Header, roll controls, full-width Canvas, centered 5×3 keys and bottom transport retained |
| Palette | Slate surfaces, gray/cyan tracks, coral red playhead and cyan key state retained |
| Typography / controls | Segoe UI-compatible type, restrained button borders, named accessible controls; compacted for smaller Windows screens |
| Notes / ruler | Actual fixture positions and track names replace illustrative concept notes; correct nominal 4/4 bars instead of image-generated inaccurate markings |
| Keyboard | All labels and order come from the Score; geometric outlines retained; added derived key numbers |
| Copy | Main required toolbar labels retained; actual `Example song`, track names, explicit example provenance, validation and audition limitations replace concept placeholders |
| Responsive behavior | Native desktop fits the keyboard; narrow view uses document scrolling and does not cover keys with a sticky transport |

There are no known blocking visual defects in the checked viewports. This is
faithful to the selected layout/style, with the above intentional, data-driven
deviations; it is not a pixel copy of illustrative notes. Temporary screenshots,
browser test dependencies and scripts live outside tracked source. An automatic
approval check rejected deletion of surplus generated platform icons with only
`blocked by policy`; they remain ignored locally. Only the icon source and
Windows-consumed ICO/PNG are included in the change. Signing, installer testing,
hosted CI and other OS platforms were not verified.

## Interaction revision verification (2026-11-03)

- Frontend: 17 Node tests and TypeScript/Vite build passed. Editing tests cover
  quantization, safe integer bounds, exact-key overlap across tracks, adjacency,
  endpoint resizing, nonmutating updates and refreshed playback indexes.
- Desktop: 16 Rust tests, formatting and Clippy with warnings denied passed.
  The new save test reads the edited file through the real CLI and verifies that
  existing destinations cannot be overwritten. Windows release build passed.
- Root workspace formatting/tests/Clippy passed. Independent numeric workspace
  formatting, 20 tests, Clippy and full gate passed: 267 fixed cases, 10,000
  historical samples, zero failures; its research/focused regression test passed.
  Windows reported some incremental-cache finalization notes; commands exited 0.
- Rendered QA used existing Playwright/Edge because the Browser skill/runtime was
  unavailable. At localhost:1420, injected IPC fixtures exercised parameter
  errors, audio/MIDI orchestration, cancellation/late completion, failure retention,
  unsaved replacement protection, note insertion, both endpoint resizes, wheel,
  context and keyboard copy/paste/delete, valid/invalid dragging, undo and playback.
  1280x880, 800x640 and 390x844 viewports had no horizontal overflow. The desktop
  viewport exposed all 15 keys and transport; narrow windows scroll vertically.
  There was no framework overlay or runtime exception. Localhost favicon requests
  returned 404; native console capture had no errors or warnings.
- Native release at tauri.localhost used the actual Windows picker and packaged
  CLI: the checked-in piano MP3 generated 635 notes directly into the editor with
  one Generate action. Adding A4 produced 636 notes; saving to a new temporary file
  preserved A4 at start_tick 0 / duration_ticks 240 and CLI validation passed.
  The real Ode MIDI generated 52 notes directly; playback advanced and paused;
  the existing arrangement report rendered. No inference was mocked in these
  native checks. Mocked failure/cancellation UI checks are distinct from them.
- No new production dependencies, public format, pitch mapping, timing policy,
  commits, release publication or deployment. Human musical acceptance, physical
  speaker output and native close-with-unsaved-edits remain unverified.

Keyboard-toggle follow-up (2026-11-03): 20 frontend tests and the production
frontend build passed. Rendered checks confirmed add → second-click delete,
occupied-key indication, undo/redo, and preservation of the other track's notes.
The running desktop executable was locked by Windows; the follow-up executable
is built separately as `skysheet-desktop-key-toggle.exe` beside the existing
`skysheet.exe` sidecar, without closing the active application.
Native follow-up executable also passed the actual example flow: first A4 click changed 6 notes to 7; second click returned to 6, with the key action changing back to Add.
