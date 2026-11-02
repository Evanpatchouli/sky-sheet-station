# SkySheet Desktop

Windows Tauri client for real SkySheet Score JSON, with a Canvas piano roll and
local synthesized audition. **音频导入** now transcribes MP3/WAV/FLAC locally into
MIDI, with progress, cancellation, candidate reports and MIDI save; continue into
Sky arrangement for Score preview, audition and save. See
[Audio Transcription](../../docs/audio-transcription.md) for limits and model notes.

```powershell
npm ci
npm run desktop
```

`npm run desktop:build` builds local release executables. Keep the desktop
executable and its CLI sibling together. For architecture, verification commands,
dependencies and known limits, see [Desktop player](../../docs/desktop-player.md).

Use **导入音乐** to choose audio or MIDI, set options and click **生成乐谱**.
The result opens directly in the grid editor. Click a cell then an instrument
key to add a note, or remove the existing note in that cell of the edit track;
drag, resize, copy/paste and delete notes; use **另存乐谱** to
validate and save edits. See the desktop guide for shortcuts and limits.

Selectors share a dark popover surface across generation settings, track/grid
editing and playback speed. Use Enter/Space or arrow keys to open, arrows or
Home/End to navigate, Enter/Space to confirm, and Escape to cancel. Tab moves to
the next control. Menus adapt to viewport edges and respect reduced-motion settings.

Generation progress uses the shared `MusicProgress` component: a decorative,
looping SVG staff with faded edges and a separate measured current-stage indicator.
Unknown totals stay indeterminate; the notation is not a transcription preview.
Motion uses CSS transforms without per-frame React updates and stops under the
system's reduced-motion preference. Inputs share the button/select surface tokens.
