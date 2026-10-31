# Agent Task Prompt Templates

Use these templates to keep agent tasks scoped and reviewable. Replace bracketed text before use.

## 1. Plan before implementation
You are working in the SkySheet Station repository.

Read AGENTS.md and the relevant sections of GOALS.md, PLANS.md, and docs/. Inspect the current implementation before proposing changes.

Task: [describe one outcome]

First return:
1. Current behavior and relevant files.
2. Proposed approach.
3. Files expected to change.
4. Compatibility or design risks.
5. Tests and validation to run.
6. Questions that block safe implementation.

Do not edit files until the plan is approved.

## 2. Implement one approved task
Implement only this approved task: [paste approved scope].

Follow AGENTS.md. Do not begin another roadmap phase. Keep the diff focused. Update documentation and fixtures if a public contract changes.

Before finishing:
- run relevant formatting, tests, and validation;
- inspect the final diff;
- report commands and observed results;
- list unresolved risks.

Do not commit, push, merge, publish, or deploy unless explicitly requested.

## 3. Review a change
Review the current diff for correctness, regressions, compatibility, security, and missing tests.

Prioritize actionable findings. For each finding include severity, file/line, impact, and a concrete fix. Do not modify files. If no findings are identified, state what was reviewed and what could not be verified.

## 4. Update the score contract
Before changing the score format, read:
- docs/music-data-format.md
- docs/schemas/sky-score-v1.schema.json
- docs/examples/sky-15-example.json
- crates/skysheet-score/

Describe the compatibility impact first. If the change is approved, update the specification, schema, example, Rust types, semantic validation, and tests together. Do not leave partial format changes.

## 5. Research a technical choice
Research the specific decision: [question].

Separate:
- facts supported by primary documentation or source code;
- benchmark results and their environment;
- assumptions and unknowns;
- recommendation for SkySheet and why.

Prefer maintained upstream repositories, official docs, published papers, and reproducible benchmarks. Check license, platform support, Rust integration path, model availability, maintenance activity, and deployment footprint. Do not add a dependency or modify code during research.
