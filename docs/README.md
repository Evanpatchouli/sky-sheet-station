# SkySheet Station Documentation

SkySheet Station is a Rust-first, CLI-first music analysis and arrangement tool. Its first target is converting audio or MIDI into arrangements adapted to Sky: Children of the Light's 15-key instruments.

## Agent workflow

Repository-wide instructions live in [AGENTS.md](../AGENTS.md). Product goals and roadmap live in [GOALS.md](../GOALS.md) and [PLANS.md](../PLANS.md). Reusable task prompts are in [PROMPTS.md](../PROMPTS.md), and the phase-based workflow is described in [harness/README.md](../harness/README.md).

## Documents

- [Architecture](./architecture.md): product boundaries, components, and repository direction.
- [Music data format](./music-data-format.md): v1 JSON score format, timing, pitch, instrument behavior, and validation rules.
- [Rust score crate](../crates/skysheet-score/): initial Rust types, Serde support, and semantic validation.
- [JSON Schema](./schemas/sky-score-v1.schema.json): machine-readable structural validation for score v1.
- [Example score](./examples/sky-15-example.json): valid JSON example for a Sky 15-key arrangement.
- [Phase 1 decisions](./phase-1-decisions.md): approved D1-D8 policies including D6-A through D6-F, calibration evidence checklist, and remaining verification/scope boundaries.
- [Conformance audit](../harness/build/phase-01-conformance-audit.md): rule coverage, raw-input observations, and verification results.
- [Conversion and arrangement](./conversion-and-arrangement.md): proposed audio/MIDI analysis and 15-key adaptation pipeline.
- [CLI specification](./cli.md): initial command-line interface proposal.

## Status

These documents capture early product decisions and proposals. Approved score
policies are identified separately from future features; the score contract is
not fully frozen and unresolved behavior is not silently assumed implemented.

Current priorities:

1. Rust for the core implementation.
2. CLI as the primary product interface.
3. JSON as the default score format, with MIDI import and export.
4. No playback automation in the core.
5. Tauri GUI as a client of the CLI.
6. MCP is explicitly deferred.

Changes to the score format follow the approved [version policy](music-data-format.md#15-serialization-and-format-compatibility-approved-d5).
Approval of individual policies does not complete the full freeze gate or authorize
MIDI implementation. Calibration and remaining verification are open; D6's approved
scope does not include Format 2/SMPTE support decisions.

On 2026-11-01, an approved pre-freeze revision resolved pitch range, integer
bounds including note ends, and meter denominators. See the
[format revision notes](music-data-format.md#14-pre-freeze-revision-2026-11-01)
and [verification record](../harness/build/phase-01-contract-boundaries.md).
The full v1 contract is not frozen; other Phase 1 questions remain open.

D2-D4 were subsequently approved: strict original integer spelling, unique decoded
JSON member names, and finite positive binary64 tempo. Direct-input regressions
now run in Cargo/CI. The numeric limits of the current Schema test tool remain
documented. The owner subsequently approved D1, D5, D7 and D8: preserve pitch/key
identity, use exact format-version support, retain embedded profiles without a
version field, and keep provenance outside v1. No external consumers currently
depend on draft 1.0, per the owner on 2026-11-01.

D6 was initially partially approved, then the owner approved A through F on
2026-11-01: reference/source tempo, absolute endpoint rounding, numeric failure,
nominal meter and PPQ-based Format 0/1 scope. The
[normative policy and boundary examples](music-data-format.md#16-approved-midi-timing-conversion-policy-d6)
distinguish mathematical expectations from future MIDI acceptance tests. No
importer/quantizer implementation, real-file verification, profile calibration
or full freeze is claimed; Phase 2 has not started.
