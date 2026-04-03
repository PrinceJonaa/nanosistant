# Contributing to Nanosistant

Thank you for your interest in contributing! This document covers everything you need to go from zero to a merged pull request.

> **Community standards:** All contributors are expected to follow our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## Table of Contents

1. [Development Setup](#development-setup)
2. [Project Structure](#project-structure)
3. [How to Add a Deterministic Function](#how-to-add-a-deterministic-function)
4. [How to Add a New Domain Module](#how-to-add-a-new-domain-module)
5. [How to Create and Submit a Pack](#how-to-create-and-submit-a-pack)
6. [How to Add a Slash Command](#how-to-add-a-slash-command)
7. [Code Style Guide](#code-style-guide)
8. [Git Workflow](#git-workflow)
9. [PR Process](#pr-process)
10. [Review Criteria](#review-criteria)

---

## Development Setup

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | stable (see `rust-toolchain.toml`) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| protobuf compiler | ≥ 3.x | `brew install protobuf` / `sudo apt install protobuf-compiler` |
| Git | any recent | — |

### Clone and Build

```bash
# Clone with submodules
git clone --recurse-submodules https://github.com/PrinceJonaa/nanosistant.git
cd nanosistant

# Build the full workspace
cargo build --workspace

# Run all tests
cargo test --workspace
```

### Verification Suite

All three commands must pass before submitting a PR:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Set `RUST_LOG=debug` for verbose output during development:

```bash
RUST_LOG=debug cargo run --bin nanosistant
```

---

## Project Structure

```
nanosistant/
├── crates/
│   ├── api/            # HTTP client — Anthropic, Azure OpenAI, OpenAI, xAI
│   ├── commands/       # Slash-command registry and built-in command handlers
│   ├── common/         # Shared types: deterministic fns, router, typed IR, handoffs
│   ├── compat-harness/ # Backward-compat shims
│   ├── lsp/            # Language Server Protocol integration
│   ├── nanoclaw/       # Edge client (iOS NanoClawKit bridge)
│   ├── nstn-cli/       # Binary entry point (nanosistant binary)
│   ├── packs/          # Pack loader and runtime
│   ├── plugins/        # Plugin manager
│   ├── ruflo/          # MCP client / ruflo integration
│   ├── runtime/        # Session, memory tiers (L0–L3), Dreamer, compaction
│   ├── ruvector/       # Vector knowledge store
│   ├── server/         # gRPC server
│   └── tools/          # Tool definitions for LLM tool-calling
├── packs/
│   ├── universal/      # Domain-agnostic packs (logic, graph, probability, information)
│   └── domain/         # Domain-specific packs (music, finance, code, geo, …)
├── config/
│   ├── agents/         # Per-domain agent TOML configs (auto-discovered)
│   ├── prompts/        # System prompt markdown files
│   └── settings.toml   # Global runtime settings
├── docs/
│   ├── architecture/   # Routing, memory, Dreamer architecture docs
│   ├── crates/         # Per-crate reference docs
│   ├── diagrams/       # Architecture diagrams
│   └── guides/         # Quickstart and how-to guides
├── proto/              # Protobuf definitions (tier boundaries)
└── tests/              # Integration tests
```

### Core Architecture Rules

1. **Tiers never import each other's internals.** Protobuf is the boundary between NanoClaw (edge), RuFlo (brain), and RuVector (knowledge).
2. **The orchestrator is deterministic code, not an LLM.** Routing decisions are made by the confidence ladder before any model is called.
3. **Handoffs between agents use typed structs.** No freeform text handoffs — use `HandoffPayload` from `nstn-common`.
4. **Closed-form operations run as code.** Zero tokens. If it can be computed, it must be computed deterministically.
5. **Don't build infrastructure speculatively.** Build when pain demands it.

---

## How to Add a Deterministic Function

Deterministic functions are pure Rust — no side effects, no I/O, no LLM calls. They run at Tier 0, before routing, and are available to all agents.

### Step-by-step

**1. Choose the right file in `crates/common/src/`:**

| File | Covers |
|------|--------|
| `deterministic.rs` | Universal gateway and simple utilities |
| `det_music.rs` | Music theory and audio calculations |
| `det_time.rs` | Date, time, and calendar operations |
| `det_finance.rs` | Financial math |
| `det_code.rs` | Code analysis utilities |
| `det_data.rs` | Data transformation utilities |
| `det_text.rs` | Text processing |
| `det_geo.rs` | Geographic calculations |
| `det/` subdirectory | Complex domain functions organized by module |

**2. Write a pure function:**

```rust
/// Converts beats per minute to milliseconds per beat.
///
/// # Examples
///
/// ```
/// use nstn_common::det_music::bpm_to_ms;
/// assert_eq!(bpm_to_ms(120.0), 500.0);
/// ```
#[must_use]
pub fn bpm_to_ms(bpm: f64) -> f64 {
    60_000.0 / bpm
}
```

Rules:
- `#[must_use]` on every function that returns a value
- Doc comment explains *what* the function computes (not just "converts X to Y" — add the formula or units)
- At least one `# Examples` block with a runnable doctest
- No `unwrap()` — return `Result<T, String>` for fallible operations
- No `unsafe`

**3. Add the function to `try_deterministic_resolution()` if it should intercept natural-language queries:**

Open `crates/common/src/deterministic.rs` and find `try_deterministic_resolution()`. Add a match arm:

```rust
// Natural language patterns that map to bpm_to_ms
if let Some(caps) = BPM_PATTERN.captures(query) {
    let bpm: f64 = caps[1].parse().ok()?;
    return Some(format!("{:.1} ms per beat", bpm_to_ms(bpm)));
}
```

**4. Write unit tests in the same file:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpm_to_ms_120() {
        assert_eq!(bpm_to_ms(120.0), 500.0);
    }

    #[test]
    fn test_bpm_to_ms_60() {
        assert_eq!(bpm_to_ms(60.0), 1000.0);
    }
}
```

**5. Consider Swift parity:**

If this function would be useful in the iOS NanoClawKit client (`clients/ios/`), note it in your PR description. A Swift equivalent may be added in a follow-up.

---

## How to Add a New Domain Module

**1. Create the agent config:**

```toml
# config/agents/your_domain.toml
[agent]
name = "your_domain"
description = "Brief description of what this agent handles"
model = "claude-opus-4-20250514"   # or appropriate tier

[routing]
keywords = ["keyword1", "keyword2"]
patterns = ["regex pattern 1", "regex pattern 2"]
confidence_threshold = 0.70

[limits]
max_tokens = 4096
temperature = 0.3
```

The orchestrator auto-discovers new configs at startup — no code changes needed to register the agent.

**2. Create the system prompt:**

```markdown
<!-- config/prompts/your_domain.md -->
You are a specialized agent for [domain].

## Capabilities
...

## Constraints
...
```

**3. Add domain-specific deterministic functions (optional):**

Create `crates/common/src/det_your_domain.rs` following the pattern in existing det modules. Add `pub mod det_your_domain;` to `crates/common/src/lib.rs`.

**4. Add routing patterns to the confidence ladder:**

Open `crates/common/src/router.rs` and add patterns to the appropriate tier (Aho-Corasick literals, regexes, or weighted keywords). Higher-confidence signals go in lower-numbered tiers.

**5. Optionally seed domain knowledge into RuVector:**

See `docs/guides/ruvector-ingestion.md` for how to add domain knowledge to the vector store.

---

## How to Create and Submit a Pack

Packs are self-contained collections of deterministic functions distributed as TOML + Rust source.

### Pack directory layout

```
packs/domain/your-pack/
├── pack.toml       # Required: metadata and routing hints
├── functions.rs    # Required: pure Rust implementations
├── rules.toml      # Optional: TOML-based declarative rules
└── README.md       # Required: usage docs and examples
```

### pack.toml format

```toml
[pack]
name = "your-pack"
version = "0.1.0"
author = "Your Name"
description = "What this pack computes."
nstn_version = ">=0.7.0"
domain = "your_domain"          # "universal" for cross-domain packs
tier = "Domain"                 # Universal | Domain | Operator
tags = ["tag1", "tag2"]
license = "MIT"                 # or SPDX expression
functions = 8                   # count
test_coverage = "100%"

[pack.routing]
keywords = ["keyword1", "keyword2"]
semantic_description = "Plain-English description for semantic routing"
confidence_threshold = 0.70
```

### functions.rs checklist

- All functions `#[must_use]` if they return a value
- No `std::process`, no network I/O, no file system access
- No LLM calls — pure computation only
- Every function has at least one `#[test]`
- All tests pass with `cargo test`

### Submission options

- **Via GitHub Issues:** Open a [Pack Submission](https://github.com/PrinceJonaa/nanosistant/issues/new?template=pack_submission.yml) issue and attach your files.
- **Via NSTN Hub:** Use the [NSTN Hub](https://princejonaa.github.io/nstn-hub/) web interface for guided submission.
- **Via PR:** Fork the repo, add your pack under `packs/`, and open a PR with the `[PACK]` prefix.

---

## How to Add a Slash Command

Slash commands (`/compact`, `/clear`, `/help`, etc.) live in `crates/commands/src/lib.rs`.

**1. Add a new command variant to the registry:**

```rust
// In crates/commands/src/lib.rs
pub struct MyCommand;

impl BuiltinCommand for MyCommand {
    fn name(&self) -> &'static str { "mycommand" }
    fn description(&self) -> &'static str { "Does something useful." }
    fn usage(&self) -> &'static str { "/mycommand [args]" }

    fn execute(&self, args: &[&str], ctx: &mut CommandContext) -> CommandResult {
        // Implementation — no LLM calls allowed here
        Ok(CommandOutput::text("Done."))
    }
}
```

**2. Register the command:**

In the `CommandRegistry::new()` constructor, add:

```rust
registry.register(Box::new(MyCommand));
```

**3. Add tests:**

```rust
#[test]
fn test_mycommand_basic() {
    let mut ctx = CommandContext::default();
    let cmd = MyCommand;
    let result = cmd.execute(&[], &mut ctx).unwrap();
    assert!(result.text().contains("Done."));
}
```

**Slash command rules:**
- Commands must be synchronous and fast (< 10 ms)
- No LLM calls — use `ctx.runtime` for session data
- Output must be `CommandOutput::text()` or `CommandOutput::structured()`

---

## Code Style Guide

### Rust conventions

- **Edition:** 2021 (set in workspace `Cargo.toml`)
- **Toolchain:** pinned in `rust-toolchain.toml` — use the exact version
- **Format:** `cargo fmt` with settings from `.rustfmt.toml`
- **Lints:** `cargo clippy --workspace --all-targets -- -D warnings` must be clean

### Must-use and purity markers

```rust
// All pure functions that return a value MUST have #[must_use]
#[must_use]
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

// Fallible functions return Result, never panic
pub fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| format!("invalid date '{s}': {e}"))
}
```

### Doc comments

Every public item needs a doc comment explaining **what it computes** (not just its signature):

```rust
// BAD
/// Converts BPM to milliseconds.
pub fn bpm_to_ms(bpm: f64) -> f64 { ... }

// GOOD
/// Milliseconds per beat at the given tempo.
///
/// `ms = 60_000 / bpm`
///
/// # Examples
///
/// ```
/// assert_eq!(nstn_common::det_music::bpm_to_ms(120.0), 500.0);
/// ```
#[must_use]
pub fn bpm_to_ms(bpm: f64) -> f64 { 60_000.0 / bpm }
```

### Error handling

- Use `thiserror` for library error types
- Use `anyhow` only in binary crates (`nstn-cli`)
- Never use `unwrap()` or `expect()` in library code (clippy will catch this)
- Prefer `?` propagation over nested `match`

### Dependency hygiene

- Add new dependencies only when strictly necessary
- Prefer workspace-level dependency declarations in the root `Cargo.toml`
- Avoid duplicating transitive dependencies — run `cargo tree --duplicates` before adding

### File organization

- One responsibility per module — split files when they exceed ~400 lines
- `lib.rs` is for re-exports and module declarations only; no logic
- Place integration tests in the top-level `tests/` directory

---

## Git Workflow

### Fork and branch

```bash
# Fork on GitHub, then:
git clone https://github.com/YOUR_USERNAME/nanosistant.git
cd nanosistant
git remote add upstream https://github.com/PrinceJonaa/nanosistant.git

# Create a branch
git checkout -b feat/your-feature-name
```

### Branch naming

| Prefix | Use for |
|--------|---------|
| `feat/` | New features or deterministic functions |
| `fix/` | Bug fixes |
| `refactor/` | Code restructuring without behavior change |
| `docs/` | Documentation only |
| `test/` | Adding or improving tests |
| `pack/` | New or updated pack submissions |
| `chore/` | CI, tooling, dependency updates |

### Commit messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add bpm_to_ms deterministic function
fix: correct rounding in song_bar_count for odd time signatures
refactor: extract domain scoring into confidence ladder module
test: add integration test for cross-agent handoff
docs: add routing architecture diagram
pack(music): add chord-theory pack with 14 functions
chore: bump aho-corasick to 1.1.3
```

Scope is optional but encouraged for crate-specific changes: `feat(common):`, `fix(ruflo):`, etc.

### Keeping your branch up to date

```bash
git fetch upstream
git rebase upstream/main
```

---

## PR Process

1. **Open a draft PR** as soon as you have something reviewable, even if not complete.
2. **Fill out the PR template** — all checkboxes must be checked or explained.
3. **Ensure CI passes** — the `ci.yml` workflow runs `fmt`, `clippy`, and `test` automatically.
4. **Request a review** when ready; assign `@PrinceJonaa` or the relevant code owner from `CODEOWNERS`.
5. **Address feedback** — push new commits; do not force-push after a review starts.
6. **Squash or rebase** before merge (the maintainer will do this if needed).

### PR size guidelines

- Prefer small, focused PRs — one concern per PR
- Large refactors should be preceded by an issue discussion
- Pack submissions can include all pack files in a single PR

---

## Review Criteria

A PR is ready to merge when all of the following are true:

| Criterion | Details |
|-----------|---------|
| **CI green** | `fmt`, `clippy`, `test` all pass |
| **Architecture compliance** | No tier boundary violations; no LLM calls in det functions |
| **Purity** | New deterministic functions have no side effects, no I/O |
| **Test coverage** | Every new public function has at least one unit test |
| **Doc comments** | All public items documented with `///` |
| **`#[must_use]`** | Applied to all pure functions returning a value |
| **No new clippy warnings** | Including `clippy::pedantic` items flagged in the workspace config |
| **PR template complete** | All relevant checkboxes checked |

For pack submissions, additionally:
- `pack.toml` has all required fields
- `README.md` explains what the pack does and includes usage examples
- Test coverage is noted in `pack.toml`

---

## Questions

- **Architecture questions:** Open a [Discussion](https://github.com/PrinceJonaa/nanosistant/discussions) or check the [architecture docs](docs/architecture/).
- **Multi-agent development workflow:** See [AGENTS.md](AGENTS.md) for git worktree patterns.
- **Pack browsing:** Visit the [NSTN Hub](https://princejonaa.github.io/nstn-hub/).
- **Quick start:** See [docs/guides/quickstart.md](docs/guides/quickstart.md).
