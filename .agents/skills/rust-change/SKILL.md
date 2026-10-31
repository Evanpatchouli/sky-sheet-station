---
name: rust-change
description: Use when implementing or reviewing Rust code, crates, workspace configuration, or Rust tests in SkySheet Station.
---

# Rust Change Workflow

1. Read AGENTS.md and inspect the workspace and affected crate before editing.
2. Confirm the task's phase and acceptance criteria in PLANS.md. Do not implement unrelated roadmap items.
3. Keep domain logic in reusable libraries. Keep CLI parsing in the CLI crate and GUI integration in the Tauri application.
4. Prefer explicit types, small modules, descriptive errors, and narrow public APIs.
5. Avoid new production dependencies unless the user approves them. For a proposed dependency, summarize maintenance, license, platform support, and alternatives first.
6. Add tests for behavior and invariants, not just implementation details.
7. Run:
   - cargo fmt --all -- --check
   - the narrowest relevant cargo test command
   - cargo test --workspace for shared contracts or cross-crate changes
   - cargo clippy --workspace --all-targets -- -D warnings when practical
8. Inspect the final diff and update docs if behavior or public contracts changed.
9. Report commands actually run and their observed outcomes. Never imply a check passed if it was not run.
