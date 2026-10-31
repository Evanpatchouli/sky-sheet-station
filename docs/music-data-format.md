# Music Data Format

**Status:** initial proposal, subject to validation during implementation.

This document describes the native JSON score emitted by SkySheet. It focuses on the instrument-adapted score, not the full internal representation used during audio analysis.

## Design goals

- Readable and editable by humans.
- Straightforward to consume from scripts and other applications.
- Explicit absolute note positions and durations.
- Validatable against a target instrument's playable pitches.
- Independent of any particular playback automation implementation.
- Suitable for JSON as the default format and MIDI as an interchange format.

## Timing

Use **960 PPQ** (pulses/ticks per quarter note) for the initial format.

- A quarter note is 960 ticks.
- A 4/4 bar is 3840 ticks.
- An eighth note is 480 ticks.
- A sixteenth note is 240 ticks.
- A quarter-note triplet subdivision is 320 ticks per note.

The smallest representable duration is **1 tick**. Start positions and durations are integers. Do not use floating-point values for score positions or durations.

960 PPQ is a practical resolution, not a claim that every conceivable tuplet maps to an integer number of ticks. When a subdivision does not divide evenly, quantization may distribute rounding across adjacent events. The audio-analysis layer may retain higher-resolution timestamps internally before converting them to score ticks.

For v1, the score has one global tempo value and one time signature. Mid-song tempo-map support is not required by the initial product scope. This can be revisited if faithful MIDI round-tripping or source-tempo preservation becomes a requirement.

## Pitch naming

Use standard scientific pitch notation strings such as C4, F#4, and Bb5.

Do not store both a game key index and a MIDI pitch. The instrument's ordered keys array defines the mapping: array index plus one is the in-game key number.

The proposed Sky 15-key pitch set is:

    [
      "C4", "D4", "E4", "F4", "G4", "A4", "B4",
      "C5", "D5", "E5", "F5", "G5", "A5", "B5",
      "C6"
    ]

This pitch set is a working assumption based on the agreed two-octave-plus-high-C layout. Exact octave labels should be verified against measured or authoritative in-game pitches before being treated as a stable compatibility guarantee.

A note in a validated Sky 15-key arrangement must exactly match one entry in the selected instrument's playable pitch set. Chromatic pitches may exist in the unrestricted internal music representation, but must be adapted before appearing in a validated Sky 15-key arrangement.

## Velocity and dynamics

Do not include MIDI velocity in the Sky arrangement note schema. The target game instruments do not expose per-note velocity in the same way as a MIDI instrument.

If a MIDI source contains velocity, the importer may preserve it in an intermediate/source representation for diagnostics or future use, but it is not part of the v1 playable Sky score.

## Instrument playback behavior

The instrument profile distinguishes two game instrument behaviors:

- **trigger:** ordinary instruments; pressing a key triggers its sound.
- **sustain:** string-like instruments; holding a key sustains the sound, with the game's own fade-in/fade-out behavior.

This is descriptive instrument metadata. The core does not implement key presses or playback scheduling. A downstream playback client decides how to interpret note durations and key release behavior for its automation environment.

## Proposed JSON example

    {
      "format_version": "1.0",
      "metadata": {
        "id": "score-001",
        "name": "Example song",
        "description": null
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
        }
      ],
      "text": null
    }

## Field reference

### Root

| Field | Type | Required | Meaning |
|---|---|---:|---|
| format_version | string | yes | Version of this JSON format. |
| metadata | object | yes | Score identity and descriptive metadata. |
| instrument | object | yes | Target instrument definition/snapshot. |
| timeline | object | yes | PPQ, tempo, and time signature. |
| tracks | array | yes | Tracks containing independent note events. |
| text | string or null | no | Optional user-authored text, usable for lyrics, notes, or arbitrary remarks. No lyric recognition is implied. |

### Metadata

| Field | Type | Meaning |
|---|---|---|
| id | string | Stable score identifier. |
| name | string | Human-readable score name. |
| description | string or null | Optional description. |

### Instrument

| Field | Type | Meaning |
|---|---|---|
| id | string | Instrument profile identifier, e.g. sky-15. |
| name | string | Display name. |
| playback_mode | enum | trigger or sustain. |
| keys | array of pitch strings | Ordered playable pitches. Position + 1 is the game key index. |

### Timeline

| Field | Type | Meaning |
|---|---|---|
| ppq | integer | Ticks per quarter note; v1 uses 960. |
| tempo | number | Global tempo in quarter notes per minute (BPM). |
| time_signature.numerator | integer | Time-signature numerator. |
| time_signature.denominator | integer | Time-signature denominator. |

### Track

| Field | Type | Meaning |
|---|---|---|
| id | string | Stable track identifier. |
| name | string | Display name. |
| role | enum | Suggested role: melody, harmony, bass, accompaniment, percussion, or unknown. |
| notes | array | Independent note events. |

### Note

| Field | Type | Meaning |
|---|---|---|
| id | string | Stable note identifier. |
| pitch | string | Standard pitch name; must be playable on the target instrument. |
| start_tick | integer | Absolute start position from the beginning of the score. |
| duration_ticks | integer | Positive duration in ticks. |

Tracks are logical groupings, not separate clocks. Notes from different tracks can overlap. A chord is represented by multiple notes with the same start_tick; their durations may differ.

## Phrase and section policy

The v1 generated score is not automatically divided into phrases, lyrics, or sections. It represents the full piece as one continuous score. Users or downstream editors may split it later.

The optional root text field is generic user text, not a machine-generated lyric field. If later requirements need multiple user-defined text regions, a separately versioned annotation/section extension can be introduced.

## Validation rules

1. format_version must be supported by the reader.
2. ppq must be 960 in v1.
3. start_tick must be a non-negative integer.
4. duration_ticks must be a positive integer.
5. pitch must be present in instrument.keys.
6. Instrument keys must contain exactly 15 unique pitches for sky-15.
7. playback_mode must be trigger or sustain.
8. Note IDs must be unique within a score.
9. Track IDs must be unique within a score.

## Open questions before freezing v1

- Confirm the exact octave labels of the 15 in-game pitches with a reference recording or measurement.
- Decide whether instrument definitions are embedded snapshots, registry references, or both.
- Decide whether v1 needs source-note provenance in the exported score or only in an analysis report.
- Confirm how a single global tempo should be selected when importing a MIDI file containing tempo changes.
