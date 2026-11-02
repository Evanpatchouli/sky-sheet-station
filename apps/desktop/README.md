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
