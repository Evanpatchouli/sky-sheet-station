# Music Data Format

**Status:** v1.0 implementation proposal. This is the current design target, not a frozen public API until the Rust model, validator, and round-trip tests are implemented.

This document defines the **exported, instrument-adapted score**. It is not the complete internal representation used during audio analysis. Internal transcription data may contain chromatic pitches, confidence values, source provenance, and other analysis metadata that do not belong in a playable Sky 15-key score.

## 1. Design goals and boundaries

- JSON is the default native score format.
- MIDI import and export are supported.
- The score is readable and editable by humans and easy for external programs to consume.
- Notes use absolute positions on one shared timeline.
- The selected instrument declares the exact playable pitch set.
- The score describes music; it does not implement playback or game input.
- No automatic lyric recognition, phrase segmentation, or bar segmentation is required.
- A user-authored text field is available for lyrics, notes, or arbitrary remarks.

The core does not define how a client presses keys, schedules simultaneous inputs, releases keys, retriggers repeated notes, or handles game input. Those are responsibilities of software consuming the score.

A note's duration is musical timing data. A downstream player interprets that duration according to its target instrument and playback strategy.

## 2. Top-level structure

A score contains:

1. format_version: format version.
2. metadata: score identity and name.
3. instrument: an embedded instrument profile snapshot.
4. timeline: tick resolution, reference tempo, and time signature.
5. tracks: logical collections of independent note events.
6. text: optional user-authored free text.

The v1 score is a complete piece. It does not require phrase, lyric, section, or bar objects. A user or downstream editor can split the score into parts if desired.

## 3. Timing model

### 3.1 Resolution: 960 PPQ

Version 1 uses **960 ticks per quarter note (960 PPQ)**.

| Musical value | Ticks |
|---|---:|
| Whole note | 3840 |
| Half note | 1920 |
| Quarter note | 960 |
| Eighth note | 480 |
| Sixteenth note | 240 |
| Thirty-second note | 120 |
| Sixty-fourth note | 60 |
| One-hundred-twenty-eighth note | 30 |
| Quarter-note triplet subdivision | 320 |

The smallest representable time unit is **1 tick**. start_tick and duration_ticks are integers; floating-point timing values are forbidden in the exported score. duration_ticks must be at least 1.

960 PPQ represents many common subdivisions exactly, including triplets and quintuplets. Not every possible tuplet divides evenly into 960. When a source subdivision cannot be represented exactly, conversion rounds event positions to the nearest tick and should distribute rounding error across a run of events where possible. The analysis layer may retain higher-resolution timestamps internally before score quantization.

At 120 BPM, one tick is approximately 0.521 ms. The chosen resolution is intended to be comfortably fine for a game-instrument score without creating unnecessarily large time values.

### 3.2 Absolute positions

Every note's start_tick is measured from the beginning of the entire score, not from the previous note and not from the beginning of its track.

Notes in different tracks share the same timeline. Simultaneous notes have the same start_tick. Their durations may differ.

A chord is represented by multiple independent note objects with the same start position. There is no special chord string or chord container in v1.

### 3.3 Tempo and time signature

timeline.tempo is a positive BPM value used as the score's reference tempo for converting ticks to playback time. At constant tempo:

    seconds = ticks / ppq * 60 / tempo

time_signature records the score's nominal meter for display and interpretation. It does not change the meaning of absolute note positions.

The exported v1 format stores one global tempo, not a tempo map. If an imported MIDI contains tempo changes, the importer should first convert its note events to absolute elapsed time using the source tempo map, then map those times onto the v1 score timeline using one selected reference tempo. This preserves note timing to the nearest tick but flattens the original tempo-map metadata. The importer must report that flattening occurred; it must not silently pretend that the original tempo map was retained.

For audio transcription, the estimated tempo may be used as the reference tempo. If tempo estimation is unreliable, the converter may choose a documented fallback reference tempo and place note events according to measured elapsed times.

A consuming application may apply a global playback-speed adjustment. That does not require the score to contain playback automation instructions.

## 4. Pitch model

### 4.1 Pitch names

The exported score stores pitch as a string in scientific pitch notation, for example C4, F#4, or A5. SkySheet defines its octave convention explicitly: **C4 is MIDI note number 60**. This avoids relying on octave labels used by individual MIDI applications, some of which differ.

For canonical spelling, use sharps for chromatic pitch classes (for example, C#4, not Db4). The Sky 15-key playable set contains natural notes only.

Do not store both a game key index and a MIDI pitch number. The instrument's ordered keys array defines the mapping: array index + 1 is the in-game key number.

### 4.2 Sky 15-key pitch set

The current working profile is:

    [
      "C4", "D4", "E4", "F4", "G4", "A4", "B4",
      "C5", "D5", "E5", "F5", "G5", "A5", "B5",
      "C6"
    ]

This corresponds to two complete natural-note octaves plus the upper C. **The exact octave labels and acoustic pitches must be verified against the actual in-game instrument before the profile is declared stable.** The ordering and 15-key count follow the product requirement; the C4/C5/C6 labels remain a calibration assumption.

### 4.3 Playability invariant

For every note in a validated instrument-specific score, note.pitch must exactly equal one item in instrument.keys.

A chromatic or out-of-range pitch may exist in the unrestricted internal music/transcription model. It must be adapted before it is written into a validated Sky 15-key arrangement.

The key number is derived at runtime by finding the pitch's zero-based index in instrument.keys and adding one. It is not duplicated in the JSON note.

## 5. Instrument model

The score embeds an instrument snapshot so a file remains understandable without depending on a locally installed instrument registry.

| Field | Meaning |
|---|---|
| id | Stable instrument identifier, e.g. sky-15. |
| name | Human-readable instrument name. |
| playback_mode | trigger for ordinary one-shot instruments or sustain for string-like instruments. |
| keys | Ordered list of playable pitch names. Array position + 1 is the game key number. |

### Playback modes

- **trigger:** pressing a key triggers its sound. The game plays the instrument's sample; the score's duration remains useful for musical spacing and downstream scheduling, but the core does not claim that holding the key changes the sample duration.
- **sustain:** holding a key sustains the sound; the game instrument itself provides its fade-in/fade-out behavior. A downstream playback client may use the note duration to determine how long it holds the key.

The distinction is descriptive metadata for consumers. It does not turn SkySheet into a player.

## 6. Velocity and dynamics

There is no velocity field in the exported Sky arrangement. The target game instruments do not expose MIDI-style per-note velocity.

A MIDI importer may retain source velocity in an internal source/transcription representation if useful, but it is not part of the v1 playable score and must not be presented as an expressive control supported by the game.

## 7. Tracks and notes

A track is a logical grouping of notes, commonly used for roles such as melody or harmony. Track role is descriptive metadata, not a guarantee that automatic analysis classified the source correctly.

Supported roles: melody, harmony, bass, accompaniment, percussion, unknown.

Each note is independent:

| Field | Type | Rule |
|---|---|---|
| id | string | Unique within the score. |
| pitch | string | Must be present in the selected instrument's keys. |
| start_tick | integer | Absolute start position; must be >= 0. |
| duration_ticks | integer | Duration; must be >= 1. |

No velocity, key index, chord string, lyric, or playback instruction is stored on a note.

## 8. Text field

The optional root-level text field is user-authored free text. It may contain lyrics, a note to self, a performance reminder, or any other text.

SkySheet does not infer that this field is lyrics and does not run lyric recognition. The field is not automatically split or aligned to notes. It may be null or omitted.

The v1 format deliberately has no automatically generated Phrase or Section objects. If a future editor needs multiple text annotations at different positions, that should be introduced as an explicit versioned extension rather than assumed in v1.

## 9. JSON example

The following is valid JSON (without comments). It demonstrates a melody track and a simultaneous three-note chord in a second track.

    {
      "format_version": "1.0",
      "metadata": {
        "id": "score-001",
        "name": "Example song"
      },
      "instrument": {
        "id": "sky-15",
        "name": "Sky 15-key instrument",
        "playback_mode": "trigger",
        "keys": [
          "C4", "D4", "E4", "F4", "G4", "A4", "B4",
          "C5", "D5", "E5", "F5", "G5", "A5", "B5",
          "C6"
        ]
      },
      "timeline": {
        "ppq": 960,
        "tempo": 120,
        "time_signature": {
          "numerator": 4,
          "denominator": 4
        }
      },
      "tracks": [
        {
          "id": "track-001",
          "name": "Melody",
          "role": "melody",
          "notes": [
            {
              "id": "note-001",
              "pitch": "C4",
              "start_tick": 0,
              "duration_ticks": 480
            },
            {
              "id": "note-002",
              "pitch": "E4",
              "start_tick": 480,
              "duration_ticks": 480
            },
            {
              "id": "note-003",
              "pitch": "G4",
              "start_tick": 960,
              "duration_ticks": 960
            }
          ]
        },
        {
          "id": "track-002",
          "name": "Harmony",
          "role": "harmony",
          "notes": [
            {
              "id": "note-004",
              "pitch": "C4",
              "start_tick": 0,
              "duration_ticks": 960
            },
            {
              "id": "note-005",
              "pitch": "E4",
              "start_tick": 0,
              "duration_ticks": 960
            },
            {
              "id": "note-006",
              "pitch": "G4",
              "start_tick": 0,
              "duration_ticks": 960
            }
          ]
        }
      ],
      "text": "Optional user note, lyric, or reminder"
    }

A string-like instrument changes playback_mode to sustain.

## 10. Field reference

### Root

| Field | Type | Required | Description |
|---|---|---:|---|
| format_version | string | yes | Score format version. v1 is 1.0. |
| metadata | object | yes | Score identity and display name. |
| instrument | object | yes | Embedded target-instrument snapshot. |
| timeline | object | yes | PPQ, reference tempo, and nominal time signature. |
| tracks | array | yes | Logical tracks containing independent notes. |
| text | string or null | no | Optional user-authored free text; not automatically interpreted as lyrics. |

### Metadata

| Field | Type | Required | Description |
|---|---|---:|---|
| id | string | yes | Stable score identifier. |
| name | string | yes | Human-readable name. |

### Instrument

| Field | Type | Required | Description |
|---|---|---:|---|
| id | string | yes | Instrument profile identifier, e.g. sky-15. |
| name | string | yes | Display name. |
| playback_mode | enum | yes | trigger or sustain. |
| keys | array of pitch strings | yes | Ordered playable pitches. Position + 1 is the game key index. |

### Timeline

| Field | Type | Required | Description |
|---|---|---:|---|
| ppq | integer | yes | Ticks per quarter note; exactly 960 in v1. |
| tempo | number | yes | Positive reference BPM. |
| time_signature.numerator | integer | yes | Positive time-signature numerator. |
| time_signature.denominator | integer | yes | Positive power-of-two denominator. |

### Track

| Field | Type | Required | Description |
|---|---|---:|---|
| id | string | yes | Unique track identifier. |
| name | string | yes | Display name. |
| role | enum | yes | melody, harmony, bass, accompaniment, percussion, or unknown. |
| notes | array | yes | Independent note events. |

### Note

| Field | Type | Required | Description |
|---|---|---:|---|
| id | string | yes | Unique note identifier within the score. |
| pitch | string | yes | Pitch name; must be playable on the selected instrument. |
| start_tick | integer | yes | Absolute start position from score start. |
| duration_ticks | integer | yes | Positive duration in ticks. |

## 11. Validation rules

A conforming v1 score must satisfy all of the following:

1. format_version is exactly 1.0.
2. All required objects and fields are present.
3. ppq is exactly 960.
4. tempo is finite and greater than zero.
5. Time-signature numerator and denominator are positive; denominator is a power of two.
6. start_tick is an integer >= 0.
7. duration_ticks is an integer >= 1.
8. Every note pitch exactly matches one entry in instrument.keys.
9. Instrument keys are unique and ordered.
10. For instrument id sky-15, keys contains exactly 15 pitches.
11. playback_mode is trigger or sustain.
12. Track IDs are unique within the score.
13. Note IDs are unique within the score.
14. Notes may overlap; overlapping notes are not a validation error.
15. Unknown fields should be rejected by the strict v1 schema rather than silently interpreted.

Some constraints, especially uniqueness across nested arrays and checking each note pitch against the instrument's keys array, require semantic validation in Rust in addition to JSON Schema validation.

## 12. Companion files

- JSON Schema: [schemas/sky-score-v1.schema.json](./schemas/sky-score-v1.schema.json)
- Valid example: [examples/sky-15-example.json](./examples/sky-15-example.json)

The JSON Schema validates field shapes and basic ranges. Rust semantic validation must additionally enforce unique IDs, pitch membership in the instrument key list, and the exact Sky 15-key profile constraints.

## 13. Open questions before freezing v1

- Confirm the exact octave labels and acoustic pitches of the 15 in-game keys with a reference recording or measurement.
- Confirm whether the embedded instrument profile needs a profile-version field.
- Decide whether source-note provenance belongs in a separate analysis report or a future score extension.
- Validate the single-reference-tempo MIDI flattening policy with real MIDI files.
