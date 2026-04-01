# Contributing to Nanosistant

This is a private project. Contributions are by invitation only.

## Development Setup

```bash
# Clone with submodules
git clone --recurse-submodules https://github.com/PrinceJonaa/nanosistant.git
cd nanosistant

# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install protobuf compiler
# macOS:
brew install protobuf
# Ubuntu/Debian:
sudo apt install protobuf-compiler

# Build
cargo build --workspace

# Test
cargo test --workspace
```

## Code Standards

### Verification (must pass before every PR)

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Rust Style

- Zero `unsafe` code (enforced via workspace lint)
- Zero clippy warnings
- Every public function has doc comments explaining WHY
- Every public function has at least one unit test
- Files stay focused — one responsibility per module
- `#[must_use]` on all pure functions returning values
- Minimize the dependency tree

### Architecture Rules

1. **Tiers never import each other's internals.** Protobuf is the boundary.
2. **The orchestrator is deterministic code, not an LLM.**
3. **Handoffs between agents use typed structs.** No freeform text handoffs.
4. **Closed-form operations run as code.** Zero tokens.
5. **Don't build infrastructure speculatively.** Build when pain demands it.

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new deterministic function for X
fix: correct BPM rounding in song_bar_count
refactor: extract domain scoring into confidence ladder
test: add integration test for cross-agent handoff
docs: update README with ruflo integration details
```

## Working with Agent Configs

Adding a new domain agent:

1. Create `config/agents/your_domain.toml` (see existing configs for format)
2. Create `config/prompts/your_domain.md` (system prompt)
3. Optionally add domain knowledge to RuVector
4. The orchestrator auto-discovers new configs — no code changes required

## Working with Deterministic Functions

Adding a new deterministic function:

1. Add the function to `crates/common/src/deterministic.rs`
2. Add pattern matching in `try_deterministic_resolution()` if it should intercept queries
3. Add unit tests
4. The orchestrator calls `try_deterministic_resolution()` before any LLM routing

## Multi-Agent Development

See [AGENTS.md](AGENTS.md) for parallel development workflow using git worktrees.

## Questions

Reach out to Prince Jona for any questions about the architecture or contributing.
