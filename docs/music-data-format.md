# Music Data Format

**Status:** v1.0 pre-freeze contract. D1-D8 are approved, including D6-A through D6-F on 2026-11-01. D6 is an approved conversion policy, not an implemented or verified importer. This does not freeze the full v1 contract or authorize Phase 2.

The [Phase 1 decision record](phase-1-decisions.md) distinguishes approved rules
from remaining implementation, verification and out-of-scope decisions. The owner confirmed no external consumers currently
depend on draft 1.0. Pending proposals do not change the accepted format.

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

### 2.1 Original JSON input rules (approved D2-D3)

Integer fields (`start_tick`, `duration_ticks`, `ppq`, and the time-signature
numerator/denominator) must use plain unsigned integer tokens. The **whole token**
must match `^(?:0|[1-9][0-9]*)$`; it is a JSON number, not a quoted string. Each
field must additionally satisfy its existing value constraints. Reject decimal
points, exponent notation, negative zero, and numeric strings even if a coercion
could produce an integer. For example, `1` has valid integer spelling, while
`1.0`, `1e0`, `-0`, and `"1"` do not. Do not round or coerce these inputs.
This rule does not apply to `tempo`, which permits decimal/exponent notation.

Within every JSON object, member names must be unique **after JSON string escape
decoding**, even when the repeated values are identical or null. For example,
`"name"` and `"\u006eame"` denote the same member name. Comparison is case-sensitive;
no additional Unicode normalization is introduced. The same name may occur in
different objects. This is separate from track/note ID uniqueness.

Both rules concern the original file. Parsing into a generic object can discard
duplicate members and original number spellings, so validating that intermediate
object cannot certify the original input. Input consumers must preserve/check
this information while reading, not silently keep the first/last duplicate.

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

The smallest representable time unit is **1 tick**. start_tick and duration_ticks are integers; floating-point timing values are forbidden in the exported score.

The maximum tick value is **9007199254740991 (2^53 - 1)**:

- start_tick must be in 0..=9007199254740991.
- duration_ticks must be in 1..=9007199254740991.
- The derived end position, start_tick + duration_ticks, must also be at most 9007199254740991. It is not a new JSON field.

This bound supports exact integer exchange with JSON consumers using binary64
numbers. Rust retains `u64` storage but validates the narrower public range.
The sum must be checked without integer overflow. For example, a note starting
at 9007199254740990 with duration 1 is valid; starting at 9007199254740991 with
duration 1 is invalid even though both fields individually satisfy their bounds.
Out-of-range values must produce errors, not truncation, clamping, or conversion
to floating-point tick values.

960 PPQ represents many common subdivisions exactly, including triplets and
quintuplets. Not every possible tuplet divides evenly into 960. For the approved
D6 conversion scope, round absolute onset/end positions to the nearest tick,
with exact half ticks upward. Do not accumulate rounded intervals or additionally
redistribute rounding error per track or note sequence. This clarifies the earlier
recommendation to distribute error: absolute-position rounding avoids cumulative
drift while retaining simultaneous events. See section 16.3. The analysis layer
may retain higher-resolution timestamps internally before score quantization;
this decision does not define an audio-transcription quantizer.

At 120 BPM, one tick is approximately 0.521 ms. The chosen resolution is intended to be comfortably fine for a game-instrument score without creating unnecessarily large time values.

### 3.2 Absolute positions

Every note's start_tick is measured from the beginning of the entire score, not from the previous note and not from the beginning of its track.

Notes in different tracks share the same timeline. Simultaneous notes have the same start_tick. Their durations may differ.

A chord is represented by multiple independent note objects with the same start position. There is no special chord string or chord container in v1.

### 3.3 Tempo and time signature

timeline.tempo counts **quarter notes per minute**, independent of the time-signature
denominator or metronome beat. This clarifies the unit in the existing time formula;
it does not change that formula. The BPM number is decoded as binary64 (`f64`), with a **finite,
strictly positive decoded value** (approved D4). Positive subnormal values are
allowed; `f64::MIN_POSITIVE` is the smallest positive *normal* value, not the
minimum permitted tempo. Decimal and exponent notation are allowed. Reject
numeric overflow, underflow that decodes to zero, zero (including negative zero),
and negative values. Numeric strings are not converted. NaN and infinity are not
JSON number literals, and in-memory nonfinite tempo values fail validation.

No additional musical BPM minimum/maximum is imposed. Under approved N2, decimal
JSON numbers decode to the nearest binary64 value, with exact midpoints rounded
to the even significand. Accepted tempo values must preserve their binary64 bits
through serialization and reading back. This does not promise arbitrary decimal
precision or original numeric-text preservation. It is distinct from D6's half-up
tick quantization. Examples: `120`, `120.5`, `1.2e2`, `1e-300`, and `1e300` are valid;
`1e400` and `1e-400` fail the current decoder/validation path. Do not clamp invalid
values to an endpoint or replace them with a fallback.

**Approved N2 reader correction:** the `tempo` field now captures the original
JSON value as `Box<serde_json::value::RawValue>` and converts its complete numeric
text with Rust's `str::parse::<f64>()`. Nonfinite results are parsing errors;
zero and negative values remain semantic validation errors. Numeric strings and
nonstandard JSON number syntax remain rejected. The field remains `f64`, with
unchanged serialization and no token-length restriction. This field adapter is
JSON-specific; arbitrary non-JSON Serde deserializers are not promised compatible.
Read original JSON directly: an intermediate `Value` cannot restore discarded
numeric or duplicate-member information.

`float_roundtrip` is not the fix: it still misrounds certain long integer/exponent
spellings of exact midpoints, including the zero boundary. Windows verification
passes the unchanged 267 fixed cases, 10,000 historical samples, 3,016 research
cases and 84 focused regressions. Linux CI for `5cee8ef` also passes these gates,
completing N2 repair acceptance without freezing the remaining v1 contract. See
[independent numeric verification](numeric-verification.md) for preserved old
failures, current evidence and compatibility details.

The reference tempo is used to convert ticks to playback time. At constant tempo:

    seconds = ticks / ppq * 60 / tempo

A valid tempo does not guarantee that every later floating-point time calculation
is representable; operations must check their own numeric results. This statement
does not introduce a playback implementation or a new tempo bound.

time_signature records the score's nominal meter for display and interpretation. It does not change the meaning of absolute note positions.

time_signature.numerator must be an integer in **1..=65535**. Its Rust storage
remains `u16`. time_signature.denominator must be one of **1, 2, 4, 8, 16, 32, 64**.
This is the supported v1 meter set, not a limitation imposed by 960 PPQ or a claim
about all possible musical meters. It does not forbid shorter note durations;
for example, a one-hundred-twenty-eighth note still occupies 30 ticks.

The exported v1 format stores one global tempo, not a tempo map. **D6-A through
D6-F are approved; no importer or quantizer is implemented.** Flattening requires
a loss diagnostic, preserving absolute onset/end times to the target tick grid.
The original tempo map and some beat/bar relationships are lost; this is not a
lossless MIDI round trip. A note that quantizes to zero duration causes a default
conversion error, not silent dropping or a one-tick extension. The complete policy,
scope, boundary examples and remaining verification are in
[section 16](#16-approved-midi-timing-conversion-policy-d6).
Real MIDI implementation and file-based validation remain separately authorized
Phase 2 work. Existing score bounds and D4 tempo acceptance are unchanged.

For audio transcription, the estimated tempo may be used as the reference tempo. If tempo estimation is unreliable, the converter may choose a documented fallback reference tempo and place note events according to measured elapsed times.

A consuming application may apply a global playback-speed adjustment. That does not require the score to contain playback automation instructions.

## 4. Pitch model

### 4.1 Pitch names

The exported score stores pitch as a string in scientific pitch notation, for example C4, F#4, or A5. SkySheet defines its octave convention explicitly: **C4 is MIDI note number 60**. This avoids relying on octave labels used by individual MIDI applications, some of which differ.

For canonical spelling, use sharps for chromatic pitch classes (for example, C#4, not Db4). The Sky 15-key playable set contains natural notes only.

Every pitch in both instrument.keys and note.pitch must map to a MIDI note number
in **0..=127**, using the C4 = 60 convention. The endpoints in canonical spelling
are C-1 and G9; B-2, G#9, and C10 are outside the range. This also applies to custom
instrument profiles. The Schema pitch pattern checks syntax only; the MIDI range
is enforced by Rust's validated `Pitch` parser. Passing Schema validation alone
does not establish a valid score.

Under approved D1, accepted pitch strings are preserved without normalization.
The input syntax is an uppercase ASCII letter A-G, optionally one ASCII `#`, then
an optional minus and ASCII octave digits, subject to the MIDI range above.
Leading octave zeros, negative zero and enharmonic sharp spellings remain accepted:
`C04`, `B#3` and `C4` map to MIDI 60 but are distinct strings; `E#4` and `F4` map
to 65 but are distinct strings. `C-0` maps to 12 and remains spelled `C-0`.
This does not permit flats, lowercase letters, Unicode sharp signs or plus signs.
Preservation concerns the decoded string, not its JSON escape spelling.

Membership and key uniqueness use exact string equality, not MIDI-number equality.
A custom profile may contain separate same-sounding keys such as `["B#3", "C4"]`;
they retain separate derived key numbers. A note `B#3` is not playable when the
profile contains only `C4`. Do not normalize, merge or reorder keys on read/write.
The special `sky-15` profile still requires the exact array in section 4.2.

For newly generated data, canonical spelling is recommended: use pitch classes
`C C# D D# E F F# G G# A A# B` and octave integers without leading zeros or
negative zero. This is not a canonical-only input restriction or a serializer
rewrite rule. When generating notes for an existing profile, use that profile's
exact key strings, even if noncanonical. No pitch generator is introduced here.

Do not store both a game key index and a MIDI pitch number. The instrument's ordered keys array defines the mapping: array index + 1 is the in-game key number.

### 4.2 Sky 15-key pitch set

The default logical profile is:

    [
      "C4", "D4", "E4", "F4", "G4", "A4", "B4",
      "C5", "D5", "E5", "F5", "G5", "A5", "B5",
      "C6"
    ]

This is the default **logical score-pitch mapping**: two complete natural-note
octaves plus the upper C, retaining exactly the existing 15 strings and order.
SkySheet uses the standard logical convention C4 = MIDI 60. Key 1 is logically
C4, key 8 C5 and key 15 C6; these labels do not assert the physical pitch sounded
by the game. Actual sounding pitches may vary with the game environment, as
clarified by the product owner; SkySheet does not measure or verify that behavior.

**Game recordings and acoustic pitch calibration are not required**, either to
define this mapping or to complete Phase 1. This supersedes the previous
calibration-assumption wording. Musical analysis, score validation and MIDI pitch
interpretation use the logical pitches; acoustic variation does not rewrite the
profile, note strings or derived key numbers. No JSON fields, pitch values, key
ordering or Rust data structures change under this decision.

### 4.3 Playability invariant

For every note in a validated instrument-specific score, note.pitch must exactly equal one item in instrument.keys. This is logical key membership, not verification that the game sounds a fixed physical pitch.

A chromatic or out-of-range pitch may exist in the unrestricted internal music/transcription model. It must be adapted before it is written into a validated Sky 15-key arrangement.

The key number is derived at runtime by finding the pitch's zero-based index in instrument.keys and adding one. It is not duplicated in the JSON note.

## 5. Instrument model

The score embeds an instrument snapshot so a file remains understandable without depending on a locally installed instrument registry.

Approved D7 adds no `profile_version` field. Consumers must preserve the embedded
ordered keys rather than overwrite them from a newer local registry. Once an ID's
mapping is stable, it must not be silently redefined; prefer a new profile ID for
a changed mapping and assess client support and migration separately. A new ID
may be structurally valid without being supported by a particular client.
Custom-profile publishers are responsible for cross-file ID consistency; no
registry mechanism is introduced. The `sky-15` C4-C6 mapping is defined logically,
independently of actual in-game sound; it has no acoustic-calibration prerequisite.
The overall v1 contract still awaits its remaining verification. Both `trigger` and `sustain` remain accepted
for this ID; D7 does not impose a new per-ID playback-mode restriction.

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
| pitch | string | MIDI number 0..=127; must be present in the selected instrument's keys. |
| start_tick | integer | Absolute start position in 0..=9007199254740991. |
| duration_ticks | integer | Duration in 1..=9007199254740991; start_tick + duration_ticks must also be <= 9007199254740991. |

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
| keys | array of pitch strings | yes | Ordered playable pitches, each mapping to MIDI 0..=127. Position + 1 is the game key index. |

### Timeline

| Field | Type | Required | Description |
|---|---|---:|---|
| ppq | integer | yes | Ticks per quarter note; exactly 960 in v1. |
| tempo | number | yes | Reference BPM decoded to finite positive binary64, including positive subnormals; decimal/exponent notation allowed. |
| time_signature.numerator | integer | yes | Time-signature numerator in 1..=65535. |
| time_signature.denominator | integer | yes | One of 1, 2, 4, 8, 16, 32, 64. |

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
| pitch | string | yes | Pitch name mapping to MIDI 0..=127; must be playable on the selected instrument. |
| start_tick | integer | yes | Absolute start position in 0..=9007199254740991. |
| duration_ticks | integer | yes | Duration in 1..=9007199254740991; the derived end position must also fit that upper bound. |

## 11. Validation rules

A conforming v1 score must satisfy all of the following:

1. format_version is exactly 1.0.
2. All required objects and fields are present.
3. ppq is exactly 960.
4. tempo decodes to finite positive binary64, including positive subnormals; overflow and underflow-to-zero are rejected.
5. Time-signature numerator is an integer in 1..=65535; denominator is one of 1, 2, 4, 8, 16, 32, 64.
6. start_tick is an integer in 0..=9007199254740991.
7. duration_ticks is an integer in 1..=9007199254740991.
8. Every note pitch exactly matches one entry in instrument.keys.
9. Instrument keys are unique by exact string and retain their order; custom profiles may contain distinct strings with the same MIDI pitch.
10. For instrument id sky-15, keys matches the exact 15 pitch strings and order in section 4.2.
11. playback_mode is trigger or sustain.
12. Track IDs are unique within the score.
13. Note IDs are unique within the score.
14. Notes may overlap; overlapping notes are not a validation error.
15. Unknown fields are rejected by the strict v1 Schema and reader rather than silently interpreted.
16. Every instrument key and note pitch maps to MIDI 0..=127.
17. For each note, start_tick + duration_ticks <= 9007199254740991, computed without overflow.
18. Integer fields use plain unsigned integer tokens, not decimal/exponent notation, negative zero, or numeric strings.
19. Every object has unique member names after JSON string escape decoding, regardless of the member values.

Some constraints, especially uniqueness across nested arrays and checking each note pitch against the instrument's keys array, require semantic validation in Rust in addition to JSON Schema validation.

## 12. Companion files

- JSON Schema: [schemas/sky-score-v1.schema.json](./schemas/sky-score-v1.schema.json)
- Valid example: [examples/sky-15-example.json](./examples/sky-15-example.json)

The JSON Schema declares field shapes, individual numeric bounds, and the meter
denominator enum. It does not enforce original integer token spelling, duplicate
members already discarded by a parser, or binary64 decoding behavior. Its
descriptions record these additional requirements; they are not numeric/string
assertions that a generic Schema engine can enforce on parsed values.

The current Rust input-validation path directly deserializes original JSON
text/bytes into `Score`, then calls `Score::validate`. Field deserialization checks
object shapes, integer tokens, duplicate members, and the validated `Pitch` range;
`Score::validate` checks positive finite tempo and score-wide semantic constraints
including note ends, IDs, membership, and the Sky profile. Neither deserialization
alone nor `Score::validate` on a preconstructed value certifies an original file.
`from_value` remains useful for structured data but cannot restore lost source
information. No new public parser API or production dependency is introduced.

The existing example remains valid and unchanged. Shared positive and negative
boundary cases are in [contract-boundaries.json](../crates/skysheet-score/tests/fixtures/contract-boundaries.json).
They are applied to that example by both Rust integration tests and
[the isolated Rust verification tool](../tools/numeric-validation/Cargo.toml). Cases explicitly record
where structural validation succeeds but semantic validation must fail.

Approved D2-D4 raw-input regressions run as normal Cargo tests in `input_rules.rs`
and `raw_input_audit.rs`. They preserve original tokens/duplicate members rather
than constructing those inputs through a generic JSON object.

Approved D1/D5/D7/D8 regressions are in `serialization_compatibility.rs` and the
shared `contract-conformance.json` cases. They cover spelling/key preservation,
exact version checks, model round trips, and rejection of unintroduced fields.
They do not implement or validate MIDI conversion, migration, or future versions.

**Independent verification:** the retired Schema tool's numeric limitations remain
historical findings, not exclusions from the format. N1-N4 now use an isolated
Rust Schema/raw/numeric gate with a separate Cargo.lock. It reuses the unchanged
Schema and original tokens/members, and uses exact rational interval search as the
independent N2 Oracle. Schema engine features cannot unify with the root reader.
It is not a complete independent score-semantic validator. All former cases and
10,000 samples are preserved. The RawValue reader passes local Windows gates;
Linux CI for `5cee8ef` also passes. See [numeric verification](numeric-verification.md). No interpreter or auxiliary
shell validation script is required.

## 13. Open questions before freezing v1

The owner removed acoustic calibration and game-recording collection from the
freeze requirements; section 4.2 defines the unchanged logical mapping. Remaining
verification is not replaced by this scope decision:

- Verify the approved D6 mathematical boundaries and diagnostic requirements against a future implementation; no MIDI/quantizer implementation has been validated. Format 2 and SMPTE support remain outside this policy and require separate decisions.
- N2 RawValue repair acceptance is complete on Windows and Linux CI for `5cee8ef`; preserve the independent gate and historical expectations during remaining contract work.
- Complete remaining contract verification. Real MIDI implementation and file-based acceptance cases in section 16.7 require separately authorized Phase 2 work; they are not completed Phase 1 tests.

The [audit record](../harness/build/phase-01-conformance-audit.md) records the
approved resolutions and remaining validator limitations. D1-D8 are approved;
D6 approval is not implementation or verification. Profile-version and provenance fields are not pending v1
additions. A future analysis report requires separate scope; no report schema is
defined here. Real MIDI implementation remains outside the current task.

The same audit identified array representations incorrectly accepted where this
specification requires objects. The approved follow-up now rejects those arrays
for all seven score object types. This is a reader correction to the existing
contract, not a new score encoding or adoption of the other draft input policies.

## 14. Pre-freeze revision: 2026-11-01

The product owner approved resolving only three boundaries: MIDI pitch range
0..=127, explicit integer bounds including the derived note end, and the existing
1..64 meter denominator set. The format_version remains **1.0** as an explicitly
approved pre-freeze contract revision, not a claim of backward compatibility.

- MIDI range and meter denominators preserve current Rust acceptance. The range
  now forms an explicit semantic requirement for Schema-only consumers too.
- The numerator maximum narrows Schema acceptance to match existing Rust `u16`.
- Tick field limits and the end-position limit reject some values previously
  accepted by Rust. Existing draft consumers must handle these validation errors;
  values must not be silently rounded or rewritten.
- JSON field names/types, PPQ, the existing example, and the Sky key mapping are
  unchanged. No wider pitch-spelling policy, MIDI importer policy, or general
  format-version policy is decided by this revision.

The safe-integer bound follows the interoperability guidance in
[RFC 8259 section 6](https://www.rfc-editor.org/rfc/rfc8259.html#section-6).

### Additional approved input decisions: D2-D4

On 2026-11-01 the owner approved strict integer token spelling, rejection of
duplicate decoded member names, and finite positive binary64 tempo semantics.
These preserve the tested direct Rust reader's behavior while making the public
input contract explicit. Schema-only, duplicate-overwriting, or arbitrary-precision
consumers may need additional checks; this is not a claim of universal backward
compatibility. JSON field shapes, field ranges, instrument mapping and dependencies
are unchanged. That approval did not adopt D1 or D5-D8; the subsequent approval
below is separate.

### Additional approved compatibility decisions: D1, D5, D7, D8 and partial D6

On 2026-11-01 the owner approved the preservation, version, profile and provenance
policies in sections 4, 5 and 15, and only the two D6 principles in section 3.3.
The owner confirmed no external consumers currently depend on draft 1.0. The
format version remains 1.0; current reader acceptance, fields, ranges, mappings
and example are unchanged. New regressions lock existing behavior. This neither
froze the contract nor authorized the then-pending D6 choices or Phase 2 implementation.

### Additional approved D6 details: A through F

The owner subsequently approved D6-A through D6-F on 2026-11-01. Section 16 records
the policy for PPQ-based SMF Format 0/1, with analytical examples and explicit
remaining verification. This follow-up changes documentation only: no importer,
quantizer, CLI flag, dependency or test algorithm is added. JSON structure, Schema,
format version 1.0, score acceptance, PPQ and instrument mappings are unchanged.

## 15. Serialization and format compatibility (approved D5)

Format versions are independent of crate/package versions. Readers accept only
explicitly supported exact version strings; the current reader supports only
`"1.0"`. It does not infer support for future minor versions, alternate spellings
such as `"1.00"`, or a patch component such as `"1.0.0"`. Unknown fields remain
errors, including hypothetical optional fields.

Round trips preserve the supported data model, not original bytes, object member
order, whitespace, JSON escapes or numeric token spelling. Array order is retained,
especially instrument keys, whose positions define game key numbers. D1 pitch
strings are retained. Omitted `text` and `null` both mean no text; the current
serializer emits `null` for either. Empty text is a distinct present string.
Successful deserialization must be followed by semantic validation.
N2 additionally requires preservation of accepted tempo's binary64 bits through
save/read cycles. This is an approved requirement with known current implementation
failures, not a claim that the existing reader already satisfies it for every value.

Each published format version freezes its observable contract: field structure
and meanings, accepted values and raw-input rules, pitch/key identity and order,
defaults and validation rules. Apply this policy after freeze:

| Change | Format-version treatment |
|---|---|
| Clarification, optimization or correction of an implementation violating an unambiguous contract | No format bump; crate versions may change independently. |
| Additive optional fields or enum values that preserve existing data meanings | New minor version, for example 1.1; readers must explicitly add support. |
| Tightened input acceptance or changed meanings, identity or timing semantics | New major version, for example 2.0. |
| Disputed implementation/specification behavior | Resolve explicitly before classifying it; calling it a bug does not bypass compatibility review. |

A new minor version is not a promise that old readers can read it. An old strict
reader rejects a new version, or new fields even if a writer incorrectly retains
the old version label. A newer reader must explicitly retain and test old-version
rules before claiming backward readability. Writing an older target requires an
explicit conversion policy and validation; relabeling or silently dropping fields
does not establish compatibility. No migrations or multi-version reader are added.
Future changes require representative old-reader/new-writer compatibility tests.

Before freeze, an owner-approved revision may retain 1.0 with documented impacts.
The absence of external consumers, confirmed on 2026-11-01, does not authorize
unreviewed changes. Remaining contract verification still
prevent declaring the full contract frozen.

### 15.1 Provenance boundary (approved D8)

Source-note IDs, original pitches and transformation history are outside the v1
score. No provenance fields, report schema or report system are added. A separate
analysis report may be proposed in a later scoped task; do not encode structured
provenance in user-authored `text`. Loss diagnostics remain required for future
conversions, but a score alone cannot reconstruct its source history. Exported
notes continue to have no velocity field.

## 16. Approved MIDI timing conversion policy (D6)

**Approved on 2026-11-01; not implemented or verified against MIDI files.** These
rules govern future conversion within D6-F's scope. They define neither a MIDI
parser nor a numeric implementation, and do not authorize Phase 2. Loss diagnostics
remain outside the score JSON, consistent with D8; no report schema is introduced.

### 16.1 Reference tempo and origin (D6-A)

Use an explicitly selected valid target reference tempo when supplied; otherwise
use the effective source tempo at source tick zero, including the default in
section 16.2. Convert source microseconds per quarter note to quarter notes per
minute, and use the actual finite positive binary64 value that will be written as
`timeline.tempo`. Do not compute ticks using a different, unrounded target value.
No CLI option syntax is specified by this policy.

Preserve source time zero and leading silence. Do not move the first note to zero,
choose the tempo at the first sounding note, average the tempo map, or silently
change the reference tempo to rescue short notes or range failures. Source tempo
determines elapsed time; target tempo determines its representation on the output
grid. Changing the target tempo requires recalculating ticks and does not request
a playback-speed change. Higher target tempos give finer time resolution but use
more ticks. No extra musical BPM bounds are added to D4.

### 16.2 Source tempo and conflicts (D6-B)

For PPQ timing, the default source tempo is **500000 microseconds per quarter note**
(120 quarter notes per minute). Apply it until the first explicit tempo event,
or throughout a file with none, and diagnose its use. An explicit tick-zero event
replaces the implicit default; the default is not a competing event. Never apply
a later event retroactively or substitute the target reference tempo for source
timing. Zero tempo and malformed/unparseable tempo events are errors, not occasions
to fall back to the default.

In Format 1, scan all tracks and merge tempo events by absolute source tick into
one global map. Diagnose events outside the first track. This is an approved
tolerance for noncanonical placement, not a claim to recover an author's intent
or to support independent per-track tempos. At the same absolute tick, identical
tempo values merge; different values cause an error identifying tracks, event
locations and values. This also applies within a single track. Neither track
priority nor traversal order nor last-event-wins resolves a conflict. Different
values at different ticks are ordinary tempo changes, not same-tick conflicts.

A change at tick `p` uses the old tempo to calculate elapsed time up to `p`; the
new tempo applies to subsequent intervals. Events exactly at `p` share that
boundary time. Compute both endpoints of a note spanning changes from the map;
its onset tempo alone does not determine its duration.

### 16.3 Absolute-position rounding (D6-C)

The following is a mathematical contract, not executable algorithm code. For an
absolute source time `t` in seconds and the actual output binary64 tempo `B`, define:

    x = t * 960 * B / 60
    Q(x) = floor(x + 1/2), for x >= 0
    start_tick = Q(x_start)
    duration_ticks = Q(x_end) - Q(x_start)

All tracks use the same absolute-time mapping. Round endpoints independently;
do not separately round duration, sum rounded intervals, or shift events afterward
to distribute error. Equal source times retain equal ticks. Distinct events that
land on the same tick remain independent notes, not deduplicated events; merged
time gaps may be reported. A zero output duration causes a default error without
deletion or forced extension. A source duration below one output tick is not by
itself an error if endpoint rounding gives a positive duration.

With correct arithmetic, each endpoint's rounding error is at most half a target
tick and the duration error is at most one tick. Half a tick is `30 / (960 * B)`
seconds, approximately 0.2604167 ms at `B = 120`. These bounds cover quantization
only, not source accuracy, and do not promise a fixed audible precision at every
D4-valid tempo.

### 16.4 Numeric reliability and failure (D6-D)

Use the exact mathematical result as the reference for rounding. Preserve source
integer/rational timing where possible rather than accumulating rounded binary64
seconds per segment. Numeric representation, integer width, algorithms and any
dependency remain implementation decisions requiring their normal review; this
policy selects none of them and adds no arbitrary-precision score representation.

Do not implement the mathematical expression above as an unchecked binary64
`floor(x + 0.5)`: near the tick bound, the addition itself can round incorrectly.
If intermediate arithmetic overflows, becomes nonfinite, or cannot reliably
determine the prescribed rounded result, fail explicitly. Do not silently underflow
to zero, saturate, truncate or clamp. Distinguish calculation/precision failure
from output-range failure and from invalid source metadata.

After determining the rounded endpoints, enforce the existing start, positive
duration and derived-end limits, including overflow-safe arithmetic. With
`M = 2^53 - 1`, continuous position `M + 0.49` rounds to `M`, whereas `M + 0.5`
rounds to `M + 1` and fails. This is rounding before validation, not clamping.
Any such failure defaults to failure of the whole conversion; do not silently
remove offending notes and present the remaining score as a successful conversion.
A D4-valid tempo can still fail a particular conversion. Do not redefine score
validity or add BPM limits to conceal calculation limitations.

### 16.5 Nominal meter and discarded changes (D6-E)

Use the effective source time signature at tick zero as the output nominal meter;
if absent, use **4/4** and diagnose the default. An explicit tick-zero signature
overrides the implicit default. Retain that nominal meter when later signatures
change, and report that the changes were discarded. Identical same-tick signatures
merge; different same-tick signatures are errors, including across tracks, rather
than arbitrary first/last selection. A later signature is not applied retroactively.

If the selected nominal signature is outside v1's numerator/denominator limits,
fail instead of replacing it with 4/4. A source-valid later signature that exceeds
the v1 limits may be discarded with the rest of the meter map and diagnosed; it
is not a value written to the output score. Malformed source metadata is not
equivalent to a valid but unrepresented change.

Do not change tick-to-time conversion, move notes, pad bars or segment the score
because of meter changes. Even a constant source meter does not guarantee original
bar boundaries after tempo flattening. MIDI metronome-click parameters are omitted
with a diagnostic. Nonstandard notation-unit relationships (a notated-32nd-note
count per MIDI quarter note other than the conventional 8) are unsupported under
this policy; merely copying numerator/denominator must not imply preservation of
that interpretation. No new score fields are added.

### 16.6 Scope and compatibility (D6-F)

This policy covers **SMF Format 0 and Format 1 with a PPQ time division** only.
Format 2's independent sequences and SMPTE-based timing need separate support
decisions; do not claim support or reinterpret them using the PPQ tempo formula.
Audio transcription, MIDI library choice, other MIDI constructs, note pairing,
export encoding and round-trip behavior are not settled by this timing policy.

The rules preserve v1 JSON structure, field meanings and value ranges. They define
future conversion output and failure conditions, so later changes require explicit
compatibility review; a change of importer policy does not automatically require
a JSON format-version bump. Same-tick conflict rejection is deliberately stricter
than last-event-wins readers. Off-first-track tempo merging is a diagnosed
tolerance. Flattening preserves elapsed timing only to the selected grid and
does not promise the original tempo map, meter map, tick spacing or bar structure.

Protocol references distinguish source rules from SkySheet's chosen policies:
[SMF description based on the IMA text](https://midimusic.github.io/tech/midispec.html)
documents defaults and event structure;
[Mido's tempo explanation](https://mido.github.io/mido/files/midi.html#midi-tempo-vs-bpm)
and [meta-event definitions](https://mido.github.io/mido/meta_message_types.html)
describe tempo and meter fields;
[Microsoft's timing reference](https://learn.microsoft.com/en-us/windows/win32/multimedia/timing-information)
distinguishes PPQ and SMPTE timing. These references do not select a library or
authorize a dependency, and they do not prescribe SkySheet's conflict/rounding policy.

### 16.7 Boundary examples and remaining verification

These are analytical expectations, **not parsed MIDI fixtures or importer tests**.
Fractional positions below are intermediate mathematical values, never JSON ticks.

| Case | Expected result under the approved policy |
|---|---|
| No source tempo; one quarter note of source time | Default 500000 microseconds/quarter, elapsed 0.5 s, target default 120, output 960 ticks; diagnose the default. |
| Source PPQ 960; first tempo is 60 at tick 960 | Default 120 applies up to 960. Source ticks 0, 960, 1920 occur at 0, 0.5, 1.5 s. Default target remains 120, giving ticks 0, 960, 2880. |
| Same example, explicitly selected target 60 | Output ticks 0, 480, 1440; elapsed times unchanged. A source note 960 to 1920 has output duration 960 rather than 1920 at target 120. |
| Source 120 at zero, 60 at 960; note from 480 to 1440 | Endpoints are 0.25 and 1 s; at target 120 output start 480, end 1920, duration 1440. Both tempo segments contribute. |
| Explicit source 60 at tick zero | Overrides implicit 120 without conflict; default target is 60. One source quarter maps to 960 output ticks. |
| Two tempo events at one tick, both 500000 | Merge; location outside the first track still requires a diagnostic. |
| Two tempo events at one tick, 500000 and 1000000 | Error even within one track; changing track order cannot choose a winner. |
| Source tempo zero | Error, not substitution with 500000. |
| Leading silence of 0.5 s at target 120 | First note stays at tick 960, not zero. |
| Continuous positions 0, 200/3, 400/3, 200 | Rounded positions 0, 67, 133, 200; intervals 67, 66, 67 without extra error redistribution. |
| Continuous positions 0.5 and 1.5 | Round upward to 1 and 2; the same time in different tracks gives the same tick. |
| Note 100.10 to 100.40 | Both endpoints round to 100; fail for zero duration. |
| Note 100.49 to 100.51 | Endpoints round to 100 and 101; duration 1 is valid rounding, not forced extension. |
| End at M + 0.49, with a start rounding to M - 1 | End rounds to M and duration is 1; valid at the limit. |
| End at M + 0.5 | Rounds to M + 1; conversion fails the end bound. |
| Exact position M, evaluated using naive binary64 M + 0.5 | The addition may become M + 1; it is not evidence that exact M is invalid. Reliable rounding must yield M. |
| D4-valid extreme target tempo, but unreliable arithmetic or zero-duration result | Conversion fails with the corresponding reason; the score tempo domain is unchanged. |
| Initial 3/4, later 5/8 | Keep nominal 3/4; diagnose lost meter change; note times do not change. |
| No initial signature, later 3/4 | Keep nominal default 4/4; diagnose the default and discarded change. |
| Initial source-valid 3/128 | Cannot be the v1 nominal meter; fail, do not replace with 4/4. |
| Initial 4/4, later source-valid 3/128 | Keep 4/4 and diagnose discarded later meter; do not reject solely for its unexported denominator. |
| Conflicting same-tick 3/4 and 4/4, or a nonstandard notation-unit relationship | Error/unsupported interpretation respectively; do not silently select or relabel. |
| Format 2 or SMPTE time division | Outside D6-F; support remains undecided, not implicitly covered by these examples. |

Remaining work is verification and separately scoped implementation, not unapproved
D6-A through D6-F rules:

- Select/evaluate numeric techniques capable of the prescribed rounding, including
  half-tick ties, extreme binary64 tempos, long timelines, intermediate overflow
  and the difference between output-range and calculation failures. No production
  dependency is selected or added here.
- In separately authorized Phase 2 work, implement and test actual MIDI parsing,
  tempo/meter event merging, boundary semantics, cross-track simultaneity,
  track-order independence, held notes, leading silence, diagnostics and whole-
  conversion failure behavior against licensed/owned reference files.
- Assess MIDI export and its precision/representation limits separately; this is
  not a lossless round-trip claim or an export implementation specification.
- Decide Format 2/SMPTE support separately if requested. Do not treat these
  excluded cases as unresolved choices inside the approved PPQ Format 0/1 policy.
- N2 repair acceptance now passes Windows and Linux CI; complete remaining
  contract checks before the Phase 1 freeze. Acoustic calibration
  is not required under the owner's logical-pitch decision. Passing document
  arithmetic or existing score tests does not satisfy those requirements or enter
  Phase 2.
