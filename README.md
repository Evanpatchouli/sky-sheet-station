# SkySheet Station

A Rust-first, CLI-first music analysis and arrangement tool for adapting audio and MIDI into scores playable on Sky: Children of the Light's 15-key instruments.

## Project status

Early development. The score contract is not fully frozen: D1-D8 are approved,
including D6-A through D6-F for PPQ-based SMF Format 0/1. Phase 2 implements MIDI
import/export and opens MIDI through the existing desktop player. Unsupported
sky-15 pitches fail with diagnostics on direct import; Phase 4-B/C provides
explicit arrangement. Audio Transcription now adds offline MP3/WAV/FLAC → MIDI
through `skysheet transcribe`, with desktop progress, reports, cancellation and
the existing arrangement/preview/audition/save flow. See
[audio transcription and limits](docs/audio-transcription.md),
[MIDI usage and limits](docs/midi.md) and the
[development plan](PLANS.md).

Phase 3 adds unrestricted MIDI inspection with `inspect-music <FILE> [--json]`
and analysis with `analyze <FILE> [--json]` (`analyze-midi` remains an alias):
source notes, tempo/meter maps, statistics and uncertain melody/key/role candidates.
It retains pitches outside sky-15 and does not generate arrangements. See
[analysis usage](docs/music-analysis.md) and [delivery evidence](docs/phase-3-delivery.md).
The existing desktop's **分析 MIDI** entry displays the same Rust analysis report.

Phase 1.5 now provides a runnable Windows CLI Alpha for existing Score JSON:

```powershell
cargo build -p skysheet-cli --release --locked --target x86_64-pc-windows-msvc
& .\target\x86_64-pc-windows-msvc\release\skysheet.exe inspect .\docs\examples\sky-15-example.json
& .\target\x86_64-pc-windows-msvc\release\skysheet.exe validate .\docs\examples\sky-15-example.json --json
```

See [CLI Alpha](docs/cli-alpha.md) for help/version, diagnostics, exit codes and
the [Windows verification record](harness/build/phase-015-cli-alpha.md).

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
- [Independent numeric verification and current parser differences](./docs/numeric-verification.md)
- [Audio development samples](./audios/README.md)
- [Agent workflow harness](./harness/README.md)

## Current Rust workspace

The workspace contains the reusable `skysheet-score` crate and the
`skysheet-midi` interoperability crate, the instrument-independent
`skysheet-music` source/analysis crate, `skysheet-arrange`, `skysheet-audio`, and the `skysheet-cli` package,
whose executable is named `skysheet`.

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
Rust plus the isolated Rust numeric/Schema gate below; it does not freeze
the compiler version. The N2 repair passed Windows checks and Linux CI; passing
the existing Cargo suite alone does not complete the remaining Phase 1 work.

If Rust is installed but `cargo` is unavailable in a PowerShell session, reopen
the terminal after installation or add its existing bin directory for that session:

    $env:PATH = (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + $env:PATH

For the independent gate, also check the isolated Rust tool workspace:

    cargo fmt --manifest-path tools/numeric-validation/Cargo.toml --all -- --check
    cargo test --manifest-path tools/numeric-validation/Cargo.toml --workspace --locked
    cargo clippy --manifest-path tools/numeric-validation/Cargo.toml --workspace --all-targets --locked -- -D warnings
    cargo run --manifest-path tools/numeric-validation/Cargo.toml --locked --

No additional runtime is needed. This workspace and its Cargo.lock are isolated
because Schema dependencies enable precision features that must not alter the
observed root reader. Root workspace tests alone do not cover the tool.
The default gate preserves 267 fixed cases and all 10,000 historical samples,
now passing after the accepted RawValue repair. The old 2,981 differences remain
archived as historical evidence. See [numeric verification](docs/numeric-verification.md) for the exact Oracle,
corpus/evidence preservation, report, exit codes and single-case reproductions.

The project stack is defined in [AGENTS.md](AGENTS.md). Rust is preferred and
required for core logic; Go/C++ need owner approval, Node.js/Bun are permitted
when necessary for auxiliary/frontend work, and Python is prohibited in every
project context. The retired validation scripts and runtime setup are not needed.

## Contributing with coding agents

Start with AGENTS.md. For a non-trivial task, use the plan-first workflow in PROMPTS.md and the phase template under harness/build/. Do not let an agent start an unrequested roadmap phase.
