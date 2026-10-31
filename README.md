# SkySheet Station

A Rust-first, CLI-first music analysis and arrangement tool for adapting audio and MIDI into scores playable on Sky: Children of the Light's 15-key instruments.

## Project status

Early development. The score format, architecture, and roadmap are documented proposals; check the docs before relying on any behavior as implemented.

## Project direction

- Rust core and standalone CLI.
- JSON as the default score format.
- MIDI import and export.
- Tauri GUI as a client of the CLI, planned after core workflows stabilize.
- No game input automation in the core.
- MCP deferred.

## Documentation

- [Agent instructions](./AGENTS.md)
- [Goals](./GOALS.md)
- [Development plan](./PLANS.md)
- [Documentation index](./docs/README.md)
- [Agent workflow harness](./harness/README.md)

## Current Rust workspace

The initial workspace contains the score model crate. Check the current workspace state before assuming other commands or crates exist.

From the repository root:

    cargo fmt --all -- --check
    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings

## Contributing with coding agents

Start with AGENTS.md. For a non-trivial task, use the plan-first workflow in PROMPTS.md and the phase template under harness/build/. Do not let an agent start an unrequested roadmap phase.
