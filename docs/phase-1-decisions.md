# Phase 1 decisions and remaining proposals

**Status: D1-D8 and N1-N4 approved on 2026-11-01. Independent numeric tooling is implemented; observed Rust/N2 conformance differences remain unresolved.**
After approving D2-D4, the owner approved preservation of pitch spellings and key
identity, the version policy, embedded profiles without a profile-version field,
and keeping provenance outside v1. The owner confirmed that no external consumers
currently depend on draft 1.0. D6's diagnostic flattening principle and default
short-note quantization error were approved first; the owner subsequently approved
A through F below. The latest follow-up authorizes documentation, boundary examples
and consistency checks only, not importer/quantizer implementation or Phase 2 work.
The subsequent N1-N4 approval authorizes independent numeric tooling and tests,
with parser differences reported before any parser-feature or behavior change.

The approved pitch range, integer limits, note end limit, and meter denominators
remain in force. See the [audit and verification record](../harness/build/phase-01-conformance-audit.md)
for observed implementation behavior. The full v1 contract is not frozen.

## Decision status

The table records approved rules; verification and excluded support decisions
remain separate. The
normative score rules are in the [format specification](music-data-format.md),
including sections 4, 5 and 15 for the newly approved compatibility policies.

| ID | Question | Approved rule or pending proposal | Compatibility consequence |
|---|---|---|---|
| D1 | What pitch spellings are permitted? | Approved: preserve validated strings, exact membership and string uniqueness, including separate enharmonic keys in custom profiles. Recommend canonical spelling for newly generated data, but use existing profile strings when referencing its keys. | No input normalization or tightened acceptance. Rejecting aliases or merging same-sounding keys would be a separate compatibility change. |
| D2 | Are integer-valued decimal/exponent tokens accepted in integer fields? | Preserve the current direct Rust reader: nonnegative integer tokens, without decimal point, exponent, or negative zero, for tick/PPQ/meter fields. Document the extra lexical restriction rather than silently coercing numbers. | `1.0`, `1e0`, and `-0` remain rejected by Rust even though Schema treats them as integer values. This is now an approved original-input rule; Schema alone cannot enforce original token spelling after parsing. |
| D3 | How are duplicate JSON members handled? | Explicitly reject repeated member names in every object, even when the values are equal. Validate original input before a generic object representation can discard duplicates. | Preserves the direct Rust behavior observed for the audited fields. Consumers that previously kept the last value would need to reject such files. Do not conflate repeated members with duplicate note/track IDs. |
| D4 | What numeric precision/range is promised for tempo? | Retain `f64` and require a finite positive decoded result, with ordinary binary64 precision. Reject overflow and underflow-to-zero; do not invent a musical BPM minimum/maximum to accommodate a test tool. | Rust already accepts 1e-300 and 1e300 but rejects 1e400 and rejects decoded zero from 1e-400. This makes the representation limit explicit. Tool limitations must not silently narrow the contract. |
| D5 | How is format compatibility versioned? | Approved: format and crate versions are independent; exact supported versions only. Freeze the observable contract. Additive extensions use a new minor version; tightened acceptance or changed meanings use a new major version after freeze. | A minor version does not promise old-reader support. No external consumers depend on draft 1.0, per the owner; the current version remains 1.0 and is not yet frozen. |
| D6 | Can v1 flatten a MIDI tempo map? | Approved: diagnostic flattening and default short-note errors, plus D6-A through D6-F: reference/source tempo, absolute rounding, reliable arithmetic/failure, nominal meter and PPQ Format 0/1 scope. | The policy is approved, not implemented or verified against MIDI files. Tempo/meter maps and some beat/bar relationships are lost. No lossless round trip or Format 2/SMPTE support is promised. |
| D7 | Add a profile-version field now? | Approved: do not add one; retain the embedded snapshot. Do not silently redefine a stable ID's ordered logical pitch mapping; prefer a new ID when that mapping changes. | No new field. Structural compatibility does not guarantee client/instrument support for a new ID. The owner removed acoustic calibration as a prerequisite; the default C4-C6 mapping and playback-mode acceptance are unchanged. |
| D8 | Add source-note provenance to the playable score? | Approved: keep provenance outside v1; consider an analysis report only in a separately scoped task. | No provenance fields or report schema. Loss diagnostics remain required; the score alone cannot reconstruct source history. |

These decisions were approved explicitly, not inferred from earlier test work.
The absence of external draft consumers is an owner statement as of 2026-11-01,
not a permanent assumption or authorization for unreviewed pre-freeze changes.

## D1: spellings and key identity

Current `Pitch` equality/hash use the stored string. `B#3` and `C4` map to the
same MIDI number but are distinct strings. Under the approved preservation policy,
a custom profile may list such strings separately, and a note must match the chosen
string exactly.
If unique *sounding pitches* are desired instead, that is a new uniqueness rule
requiring coordinated Schema/semantic-policy review and compatibility tests.
The existing `sky-15` keys remain exactly the current ordered natural-note list.

Input and serialization preserve the decoded pitch string, including accepted
octave spellings such as `C04` and `C-0`; this is not byte/JSON-escape preservation.
Flats, lowercase letters and Unicode sharp symbols are not newly accepted.
For newly generated pitches, canonical spelling is recommended: pitch classes
`C C# D D# E F F# G G# A A# B`, with no leading octave zeros or negative zero.
An existing profile takes precedence: its note references must use its exact key
strings. Do not normalize only notes or silently merge/reorder keys.

For example, normalizing custom keys `["B#3", "C4"]` would create duplicates;
deduplication would change derived key numbers. Canonical-only input or uniqueness
by sounding pitch would tighten the approved contract and require a version plan.

## D2-D4: approved rules and remaining verification limitations

The former direct-input audit cases are now normal regression tests with explicit
accept/reject expectations. Additional tests cover all five integer fields, duplicate
members at all seven object levels (including decoded escape equivalence and null
text), and positive subnormal/maximum finite tempo. The optional Schema audit and
the Value-first comparison remain diagnostic: they cannot certify original input.

D2 requires whole integer tokens matching `^(?:0|[1-9][0-9]*)$` plus existing field
bounds; decimal/exponent forms, negative zero, and numeric strings are rejected.
D3 rejects repeated decoded names within each object, even for equal/null values;
comparison is case-sensitive without additional Unicode normalization. D4 accepts
finite positive decoded binary64 values, including positive subnormals, with
rounding; N2 subsequently specifies nearest-value/ties-even decoding and bit-preserving
save/read cycles. Overflow and underflow-to-zero are errors. No musical BPM
bound, arbitrary precision promise, or new public parser API is introduced.

The observed `Test-Json` implementation rejects 1e-300 and 1e300 tempo values
that Rust accepts. This is a validator/representation limitation, not evidence
that JSON Schema itself specifies those exclusions. Keep this limitation visible;
N1-N4 now provide the independently checked numeric strategy and approved locked
development dependencies. Current Rust/N2 discrepancies must be resolved before
freeze; no parser fix was authorized as part of that tooling task. See the
[numeric verification record](numeric-verification.md).

Accepting integer-valued decimals/exponents was not selected for D2. Any future
proposal to broaden input would require a new compatibility decision and precise
handling that does not round integer tokens through `f64`.

Separate from these policy choices, the owner approved a focused follow-up fixing
the array/object deserialization mismatch. All seven score object types now require
objects, as the existing public contract and Schema already specify. The fix
rejects previously misaccepted arrays without changing the Schema. See the
[verification follow-up](../harness/build/phase-01-conformance-audit.md#approved-follow-up-object-only-deserialization).
That earlier repair did not itself adopt D1-D8. D2-D4 were approved subsequently;
the later approvals of D1/D5/D7/D8, partial D6 and then D6-A through D6-F are recorded above.

## D5: approved serialization compatibility policy

The policy is normative in format-specification section 15. It adds no parser,
migration API, future-version reader or writer:

- Read exactly the supported format versions; never infer future minor-version
  support from the major version alone.
- JSON object order and whitespace do not carry musical meaning. Round trips
  preserve the supported data model, not original bytes or number spellings.
- Array order is preserved. In particular, instrument key order defines key
  numbers and must not be sorted during serialization.
- `text` omission and null both mean no text; the current serializer emits null
  for `None`. Do not promise omission/byte-for-byte preservation.
- Deserialization is not full validation. A successfully parsed `Score` must
  still pass semantic validation before being treated as a playable arrangement.
- Format versions are independent of crate versions. Freeze field structure,
  meanings, accepted values and raw-input rules, pitch identity, key order,
  defaults and validation behavior for each published format version.
- Clarifications, optimizations and fixes to violations of an unambiguous
  contract do not require a format bump. A disputed behavior must first be
  resolved explicitly; calling it a bug does not settle compatibility.
- Additive fields or enum values preserving existing meanings use a new minor
  version (for example, 1.1). Tightened acceptance, changed meanings, identity or
  timing semantics use a new major version (for example, 2.0) after freeze.
- Old strict readers reject both unknown versions and unknown fields. A minor
  version is not a forward-readability promise. New readers must explicitly
  retain/test older versions to claim support. Writing an older target version
  requires explicit conversion, not merely relabeling the file or dropping fields.
- Compatibility changes require representative old-reader/new-writer tests.
  During pre-freeze, an owner-approved revision may retain 1.0, with its impacts
  documented. No external consumers currently depend on draft 1.0; this does
  not authorize future changes automatically or freeze the current contract.

## D7-D8: approved profile and provenance boundaries

Keep the embedded instrument snapshot and add no `profile_version` field.
Consumers must not overwrite its ordered keys with a newer local registry entry.
Once an ID's mapping is stable, a changed mapping should use a new ID, with client
support and any migration assessed separately. Custom-profile publishers are
responsible for cross-file ID consistency; no registry enforcement is introduced.
The `sky-15` mapping is the default logical C4-C6 key mapping, not an acoustic
measurement or a claim about fixed game tuning. Actual sounding-pitch changes do
not redefine this logical profile or require a new profile ID. Both currently
accepted playback modes remain accepted; this decision does not fix one per ID.

Source-note IDs, original pitches and transformation history stay outside the v1
score. A separate report can be proposed later; no schema or report system is
authorized now. Do not repurpose user-authored `text` for structured provenance.
Loss diagnostics are still required, but a score alone does not preserve enough
information to reconstruct its source. The absence of exported velocity remains.

## Logical Sky pitches and actual game sound

**Owner-approved design update, 2026-11-01.** Actual instrument sound may vary with
the game environment. SkySheet therefore uses standard logical score pitches
without requiring measurement of the game's acoustic output. This records the
owner's product boundary, not a new empirical claim about particular environments.

Keep the exact existing `sky-15` natural-note array C4 through C6, with C4 = MIDI
60 and array position plus one defining the game key number. These are logical
labels for score analysis, arrangement, validation and MIDI interpretation; they
are not a guarantee of the pitch actually sounded in a particular game context.
Neither acoustic variation nor removal of calibration changes the array or keys.

The previous recording procedure and calibration evidence checklist are withdrawn.
No game recording, measured frequency or physical octave-label verification is
required to define the default profile or finish Phase 1. Historical audit/build
references to calibration describe the former requirement and are superseded by
this decision; no measurement is claimed to have been completed.

Compatibility: JSON fields, format version, pitch strings/order, D1 identity and
D7 mapping policy, Rust data structures and score-reader behavior remain unchanged.
The full contract still requires its remaining verification, including independent
numeric validation. This update does not declare a full freeze, start MIDI
implementation or authorize Phase 2.

## D6: tempo-flattening decision and acceptance preparation

**Policy approved; implementation and MIDI acceptance verification not performed.**
The owner first approved flattening with loss diagnostics and default errors for
short-note quantization failure, then explicitly approved all six recommendations:

| Decision | Approved rule | Compatibility consequence |
|---|---|---|
| D6-A | Explicit valid reference tempo takes priority; otherwise use the effective source tempo at zero. Preserve origin/leading silence; do not average, use first-note tempo or automatically change the target. | Reference choice changes ticks/resolution, not intended elapsed timing. No CLI option is promised. |
| D6-B | Missing source tempo uses 500000 microseconds/quarter until an explicit event. Invalid tempo errors. Scan all tracks, diagnose off-first-track events, merge equal values at one absolute tick and reject differing values, even within one track. | Diagnosed tolerance for event placement; stricter than last-event-wins conflict handling. Implicit defaults are not competing events. |
| D6-C | Round absolute onset/end positions to nearest ticks, exact halves upward, then subtract. No extra error redistribution or cumulative rounded deltas; zero duration defaults to failure. | Simultaneous events stay aligned. Distinct events are not deduplicated; source duration below one tick alone is not grounds for failure. |
| D6-D | Exact mathematics defines rounding; use the actual output binary64 tempo. Validate rounded ranges; unreliable arithmetic, overflow or invalid output fails the whole conversion. | No clamping, silent note removal or new BPM range. Score validity and ability to complete a conversion remain distinct. |
| D6-E | Keep the initial nominal meter, default 4/4; diagnose discarded changes. Same-tick conflicts and unrepresentable initial meter interpretations fail. | Later source-valid, unexported meters may exceed v1 limits. Timing is not changed to preserve bars; metronome data is omitted with diagnostics and nonstandard notation-unit relationships are unsupported. |
| D6-F | Limit this policy to PPQ-based SMF Format 0/1. | Format 2 and SMPTE support require separate decisions; they are not implicitly supported. |

The normative details and boundary examples are centralized in
[format section 16](music-data-format.md#16-approved-midi-timing-conversion-policy-d6).
`timeline.tempo` means quarter notes per minute, regardless of meter. D6-C replaces
the earlier ambiguous recommendation to distribute rounding error with absolute-
position rounding without extra event shifting. No JSON fields, ranges, mappings,
dependencies or implemented reader behavior change in this documentation task.

Examples include default/late tempo, explicit target changes, notes spanning tempo
boundaries, same-tick conflicts, leading silence, half-tick ties, collapsed and
noncollapsed sub-tick notes, exact upper-bound rounding, missing/changing meters,
and out-of-scope time divisions. They are mathematical expectations, not a MIDI
fixture suite or evidence that an importer works.

Remaining verification: choose and validate a reliable numeric implementation;
exercise actual files, event merging, track-order independence, simultaneity,
diagnostics and whole-conversion errors in separately authorized Phase 2 work.
Library selection, other MIDI constructs, note pairing, export precision and
round-trip behavior are outside this timing-policy approval. Format 2/SMPTE
support stays undecided outside the approved scope. Independent score-numeric
verification and remaining contract checks still prevent the full freeze; the approval itself
does not supply that evidence.

## N1-N4: independent numeric validation

The owner-approved stack migration supersedes N1's former language/library choice:
use an isolated Rust workspace, locked Rust dependencies and an exact independent
Oracle (N1). Nearest binary64/ties-even decoding and bit-preserving save/read (N2),
the strict independent CI gate (N3), and reporting differences before any parser
feature or expectation adjustment (N4) remain unchanged. All validation now runs
on Rust; no auxiliary interpreter or shell validation script is required.

The implemented gate checks 120 existing structural cases, 114 original-text cases,
33 exact midpoint-neighbor cases and 10,000 deterministic in-memory samples.
Independent expectations and self-tests pass, but the Rust comparison currently
fails. Acceptance, midpoint and save/read differences have minimal reproductions
and compatibility analysis in [numeric-verification.md](numeric-verification.md).
No Rust production source, Cargo manifest/lock/feature, JSON field or Schema
assertion changed. N2 is an approved pre-freeze precision refinement, not a claim
of current implementation conformance or a change to D6 tick rounding.

## Freeze gate

Completion requires approved policies and corresponding contract verification.
The owner explicitly removed acoustic calibration and recording collection from
scope; neither is a freeze condition. The independent strategy now exists, but
its observed Rust/N2 differences and remaining consistency checks are unresolved.
Missing required numeric/contract evidence cannot
be replaced by passing software tests. D6-A through D6-F are no longer pending
policy choices, but their implementation and file-based verification are not done.
Documentation and mathematical examples do not authorize entry into Phase 2.
