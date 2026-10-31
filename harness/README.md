# Agent Workflow Harness

This directory contains lightweight project-specific workflow records. It follows the plan-first, phase-based approach described in the OpenAI Codex iterative development workflow recipe, adapted to SkySheet Station.

## Standard loop

1. **Orient:** read AGENTS.md, then the relevant goal, plan, and design documents.
2. **Preflight:** inspect current code, tests, repository state, and applicable nested instructions.
3. **Plan:** write a small phase/task plan with scope, non-goals, files, acceptance criteria, and verification.
4. **Human gate:** obtain approval before implementation for non-trivial or compatibility-sensitive tasks.
5. **Build:** implement only the approved phase.
6. **Verify:** run checks and record observed results. Do not claim unrun checks passed.
7. **Review:** inspect the diff, compatibility impact, and documentation consistency.
8. **Record:** update PLANS.md and append a concise build-log entry when the phase completes or becomes blocked.
9. **Stop:** do not continue into the next phase without a new request.

## Directory ownership

- build/: phase plans and the reusable phase template.
- context/: concise task-specific context notes that are not already captured elsewhere.
- code_review/: durable review reports when they add value.
- build-log.md: short record of completed or blocked phases.

## Keep the harness lightweight

Do not create a context file, review report, or plan artifact for every trivial edit. Use the smallest amount of process that makes the work reviewable. Do not duplicate AGENTS.md, GOALS.md, PLANS.md, or docs content here.
