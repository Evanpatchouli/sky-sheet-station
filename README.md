# SkySheet Station

A Rust-first, CLI-first music analysis and arrangement tool for adapting audio and MIDI into scores playable on Sky: Children of the Light's 15-key instruments.

## Project status

Early development. The score contract is not fully frozen: D1-D8 are approved,
including D6-A through D6-F for PPQ-based SMF Format 0/1. This is policy approval,
not an implemented or MIDI-file-verified importer/quantizer. Architecture and roadmap features are
not all implemented; see the [decision record](docs/phase-1-decisions.md) and
[development plan](PLANS.md) before relying on support. MIDI implementation
remains Phase 2.

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
- [Audio development samples](./audios/README.md)
- [Agent workflow harness](./harness/README.md)

## Current Rust workspace

The initial workspace contains the score model crate. Check the current workspace state before assuming other commands or crates exist.

Use a Rust toolchain supporting edition 2024 with the `rustfmt` and `clippy`
components, plus the native linker/build tools for your Rust target. The Phase 0
baseline was verified with Rust/Cargo 1.99.1 on Windows MSVC; this is an observed
environment, not a declared minimum Rust version or platform support policy.

From the repository root:

    cargo fmt --all -- --check
    cargo test --workspace --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings

Keep `Cargo.lock` in version control to reproduce the dependency resolution.
The first build needs registry access unless dependencies are already cached.
Build output under `target/` is ignored. CI runs the same three checks on stable
Rust plus the shared Schema checks below; it does not freeze the compiler version.

If Rust is installed but `cargo` is unavailable in a PowerShell session, reopen
the terminal after installation or add its existing bin directory for that session:

    $env:PATH = (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + $env:PATH

To check the example against the JSON Schema in PowerShell 7 with `Test-Json`:

    Get-Content -Raw docs/examples/sky-15-example.json | Test-Json -SchemaFile docs/schemas/sky-score-v1.schema.json

For the shared contract boundary and conformance cases, run:

    pwsh -NoProfile -File scripts/test-score-schema.ps1

This script requires PowerShell 7.4 or later. Cargo integration tests apply Rust
validation to the same cases. Schema validation covers structure; Rust additionally
checks MIDI pitch range, note end limits, pitch membership, and ID uniqueness.
The approved limits and compatibility impact are recorded in the
[contract revision record](harness/build/phase-01-contract-boundaries.md).

The [Phase 1 audit](harness/build/phase-01-conformance-audit.md) records remaining
validator differences. Approved D2-D4 rules are tested against original JSON
text during normal Cargo tests; the optional Schema `-AuditRaw` output is diagnostic.
`Test-Json` is a structural fixture checker, not a full-range validity authority:
it rejects some extreme tempo values permitted by the binary64 contract.

## Contributing with coding agents

Start with AGENTS.md. For a non-trivial task, use the plan-first workflow in PROMPTS.md and the phase template under harness/build/. Do not let an agent start an unrequested roadmap phase.
