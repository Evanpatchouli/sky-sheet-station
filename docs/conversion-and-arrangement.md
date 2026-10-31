# Conversion and Arrangement Pipeline

**Status:** research-backed design proposal; algorithm choices require prototyping and evaluation.

## Inputs

SkySheet accepts two broad input classes:

1. Audio files such as MP3, WAV, and FLAC. These require decoding and automatic transcription.
2. MIDI files. These already contain note events and should bypass audio transcription.

MIDI import is a first-class workflow because note events, tracks, timing, and instrument/channel metadata are usually more directly available than in a mixed audio recording.

## High-level pipeline

    Audio input
      -> decode and normalize
      -> optional source separation
      -> multi-pitch transcription
      -> note-event cleanup
      -> music analysis
      -> target-instrument arrangement
      -> validation
      -> JSON / MIDI export

    MIDI input
      -> parse tracks, notes, tempo and meter metadata
      -> normalize into internal music representation
      -> music analysis
      -> target-instrument arrangement
      -> validation
      -> JSON / MIDI export

## Audio transcription research

Candidate projects and approaches to evaluate:

- **Basic Pitch:** audio-to-MIDI transcription with polyphonic note estimation. It is a useful baseline, but mixed recordings can produce false positives, missed notes, and octave errors. It should not be assumed to identify semantic instrument roles reliably.
- **basic-pitch-rs:** a Rust-oriented implementation worth benchmarking for integration and deployment constraints. Verify its current maintenance, model compatibility, licensing, and output quality before adopting it.
- **Melodia / predominant melody extraction:** useful research for estimating a dominant melodic line. A predominant melody is not necessarily the same as the song's intended lead instrument, especially in dense arrangements.
- **Source separation:** may help isolate vocals or instruments before transcription, but adds model size, runtime, and artifacts. Treat it as an optional later stage rather than a mandatory dependency.

The first prototype should compare at least one polyphonic transcription baseline and one predominant-melody approach on a small, representative evaluation set.

## MIDI import

MIDI import should preserve, where supported:

- Track boundaries and track names.
- Note pitch.
- Note-on and note-off timing.
- Overlapping notes and chords.
- Tempo and time-signature metadata.
- Source velocity in the unrestricted intermediate representation, if useful.

The exported Sky arrangement omits velocity because the target game instruments do not expose MIDI-style per-note dynamics.

MIDI files can contain tempo changes. The v1 score proposal has one global tempo and no tempo map. Before implementing MIDI import, define a deterministic policy for files with tempo changes: either normalize note positions to a chosen constant-tempo score timeline while preserving real-time offsets, or defer such files with a clear diagnostic. Do not silently discard tempo changes.

## Music analysis

Analysis should provide estimates and alternatives, not claim certainty beyond the source.

Candidate outputs:

- Estimated global tempo and meter.
- Candidate key/scale, with confidence or alternatives.
- Note-event confidence for audio transcription.
- Candidate melody line(s).
- Candidate bass line and harmonic accompaniment.
- Repeated motifs or repeated passages, if reliably detected.

Track role labels such as melody, harmony, and bass are inferred metadata. They are not guaranteed by the original audio and should be open to user correction in a future editor.

## Arrangement for Sky 15-key

The target pitch set is the 15 pitches declared by the Sky 15-key instrument profile. The exact octave labels must be verified before release.

The arrangement algorithm should optimize musical preservation subject to instrument constraints. It should not simply delete every unsupported note.

### Proposed stages

1. **Select a candidate lead line.** Prefer coherent melodic contours and salient notes. Keep alternative candidates when confidence is low.
2. **Estimate tonal context.** Estimate key/scale and local harmonic context where possible.
3. **Try global transposition.** Search transpositions that maximize the number of supported pitches while preserving interval relationships. Penalize transpositions that create awkward register or excessive octave changes.
4. **Map remaining unsupported pitches.** Use scale-aware and context-aware mapping rather than always selecting the nearest semitone. Preserve contour and local melodic intervals where possible.
5. **Fit the playable register.** Use octave displacement when it reduces range violations, but penalize large leaps and frequent octave switching.
6. **Reduce polyphony.** Preserve the lead line first. Add selected chord tones or bass notes when they contribute meaningful harmony and do not overwhelm the target instrument's practical playability.
7. **Simplify low-value events.** Remove low-confidence detections, redundant doublings, and ornamental events when necessary. Avoid deleting notes solely because they are short; short notes may be rhythmically or melodically important.
8. **Validate.** Every output pitch must belong to the target instrument profile. Report all lossy operations.

### Objective function

A future optimizer can score candidate arrangements using weighted terms:

    score =
      melody_preservation
      + harmonic_coherence
      + rhythmic_preservation
      + contour_preservation
      - unsupported_pitch_penalty
      - excessive_leap_penalty
      - octave_switch_penalty
      - unnecessary_note_density_penalty

Weights must be tuned against human-reviewed examples. Do not treat an arbitrary set of weights as established music theory.

## Two output modes

The initial product should support:

- **Melody mode:** prioritize a coherent, recognizable single-note lead line.
- **Polyphonic mode:** retain selected simultaneous notes to suggest harmony while respecting the instrument's playable pitch set.

An easier/simplified mode can be added after the first two modes have measurable quality.

## Evaluation plan

Build a small test corpus containing:

- Solo melody recordings.
- Piano recordings with melody and accompaniment.
- Dense mixed recordings with vocals and drums.
- MIDI files with clean tracks and overlapping notes.
- Examples containing chromatic notes and notes outside the target register.

Evaluate:

- Pitch transcription precision/recall.
- Note onset and offset accuracy.
- Melody continuity.
- Percentage of notes playable on the target instrument.
- Human-rated recognizability.
- Human-rated musical coherence.
- Processing time and memory use on supported hardware.

## Open research questions

- Which Rust-compatible transcription implementation provides acceptable accuracy and licensing?
- Is source separation worth its additional cost for the first release?
- Which melody extraction method works best for mixed recordings?
- How should the algorithm rank melody notes against chord tones and bass notes?
- What is the best policy for MIDI files containing tempo changes when the v1 score format stores one global tempo?
- What octave convention matches the in-game 15-key instrument?
