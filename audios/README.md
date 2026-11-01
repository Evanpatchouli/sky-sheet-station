# Audio development samples

The project owner supplied these files on 2026-11-01 for development and testing.
The descriptions below are the owner's classifications, not measured analysis results.

| File | Description | Size (bytes) |
|---|---|---:|
| [Scarlxrd-Ace - 小星星(钢琴版).mp3](<Scarlxrd-Ace - 小星星(钢琴版).mp3>) | Relatively simple piano example | 4209680 |
| [Donk小曲.mp3](Donk小曲.mp3) | More complex modern-song example | 7201184 |

The original files are retained unchanged. No transcription, note/timing ground
truth, quality evaluation or automated audio test has been produced for them yet.
They are candidate material for later authorized audio work, not evidence that
audio conversion is implemented or accurate. They are ordinary source-audio samples;
game-instrument recording and acoustic calibration are not required under the
[logical-pitch design decision](../docs/phase-1-decisions.md#logical-sky-pitches-and-actual-game-sound).

Adding these samples does not start Phase 2 or the later audio-research phase.
Current score-contract tests and CI do not process these audio files.

## Owner-supplied MIDI development input

`ode-to-joy.mid` (575 bytes) was committed by the owner in `c5977a9` on 2026-11-01,
after Phase 2 implementation. SHA-256:
`6e70886112270d2b68a826d8cee5283768766c97f95b54881cc29244f679dc2a`.
External author, download source and redistribution license are unrecorded;
owner submission is not a license determination. The original file is retained
and used directly, not copied into newly licensed evaluation material.

It contains three tracks, PPQ 480 and 52 paired notes, including 22 unsupported
by sky-15. Phase 3 analyzes all notes and checks exact preservation against a
byte-derived reference. Musical reference labels still require human review.
See [analysis evaluation](../docs/phase-3-evaluation.md).
