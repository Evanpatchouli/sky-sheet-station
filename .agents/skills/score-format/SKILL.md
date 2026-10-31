---
name: score-format
description: Use when changing SkySheet score JSON, its schema, Rust score types, MIDI mapping, or compatibility rules.
---

# Score Format Maintenance

1. Read AGENTS.md and docs/music-data-format.md.
2. Inspect docs/schemas/sky-score-v1.schema.json, docs/examples/sky-15-example.json, and crates/skysheet-score/.
3. Identify whether the request changes the public contract or only an implementation detail.
4. Before a public contract change, explain compatibility impact and obtain approval as required by AGENTS.md.
5. Keep these artifacts synchronized when the contract changes:
   - normative format documentation;
   - JSON Schema;
   - example fixtures;
   - Rust serialization types;
   - semantic validation;
   - positive and negative tests.
6. Check that pitch names, key ordering, PPQ, absolute start ticks, positive durations, playback modes, and absence of exported velocity remain consistent.
7. Validate JSON syntax and run relevant Rust tests.
8. Report compatibility implications and any unresolved assumptions.

Do not add playback scheduling, game input automation, lyric recognition, or automatic phrase segmentation to the score contract.
