# AGENTS.md

## Parallel Development via Git Worktrees

git worktree add ../nstn-task-XX main

Each agent works in its own worktree. Merges via PR to main.

## Task Rules

- Lower-numbered tasks must complete before higher-numbered dependents.
- See parallelization map for which tasks can run simultaneously.
- Each task has INPUTS and OUTPUTS. Done = outputs exist + tests pass.

## File Ownership

- proto/ — Task 01 only. All others consume.
- crates/runtime/ — Task 02 (fork). Others import.
- crates/common/ — Task 03. Others import.

## CI Gate

Every PR: cargo fmt --check && cargo clippy && cargo test
