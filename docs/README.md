# SkySheet Station Documentation

SkySheet Station is a Rust-first, CLI-first music analysis and arrangement tool. Its first target is converting audio or MIDI into arrangements adapted to Sky: Children of the Light's 15-key instruments.

## Documents

- [Architecture](./architecture.md): product boundaries, components, and repository direction.
- [Music data format](./music-data-format.md): v1 JSON score format, timing, pitch, instrument behavior, and validation rules.
- [Rust score crate](../crates/skysheet-score/): initial Rust types, Serde support, and semantic validation.
- [JSON Schema](./schemas/sky-score-v1.schema.json): machine-readable structural validation for score v1.
- [Example score](./examples/sky-15-example.json): valid JSON example for a Sky 15-key arrangement.
- [Conversion and arrangement](./conversion-and-arrangement.md): proposed audio/MIDI analysis and 15-key adaptation pipeline.
- [CLI specification](./cli.md): initial command-line interface proposal.

## Status

These documents capture decisions from early product design. They are proposals, not an implemented or frozen public API. Unresolved behavior is identified rather than silently assumed.

Current priorities:

1. Rust for the core implementation.
2. CLI as the primary product interface.
3. JSON as the default score format, with MIDI import and export.
4. No playback automation in the core.
5. Tauri GUI as a client of the CLI.
6. MCP is explicitly deferred.

Changes to the score format should be versioned and documented here. The v1 score specification is a proposal until its Rust implementation, validator, and import/export tests are complete.
