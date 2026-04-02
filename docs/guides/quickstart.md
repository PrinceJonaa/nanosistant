# Developer Quickstart

Get Nanosistant running locally in under 10 minutes.

> **See also:** [Model Providers](./model-providers.md) · [Adding a Domain Agent](./adding-a-domain.md) · [Architecture Overview](../architecture/overview.md)

---

## 1. Prerequisites

| Tool | Required | Notes |
|---|---|---|
| **Rust** | Yes | 1.75+ via [rustup](https://rustup.rs). Check: `rustc --version` |
| **protoc** | Yes | protobuf compiler — needed to build `nstn-common` |
| **Git** | Yes | for cloning with submodules |
| **Node.js** | Optional | 20+ — only needed to run the ruflo MCP backend for AI-assisted routing of ambiguous queries |
| **Docker** | Optional | for the containerised setup with Qdrant |

**Install protoc:**

```bash
# Debian / Ubuntu
sudo apt install protobuf-compiler

# macOS
brew install protobuf

# Arch
sudo pacman -S protobuf
```

**Verify:**

```bash
rustc --version      # rustc 1.75.0 or later
protoc --version     # libprotoc 3.x or later
```

---

## 2. Clone with Submodules

The ruflo vendor directory contains git submodules:

```bash
git clone --recurse-submodules https://github.com/PrinceJonaa/nanosistant.git
cd nanosistant
```

If you already cloned without `--recurse-submodules`:

```bash
git submodule update --init --recursive
```

---

## 3. Build

Build all 13 workspace crates:

```bash
cargo build --workspace
```

For a release build:

```bash
cargo build --workspace --release
```

The main binaries produced are:
- `target/debug/nanosistant` — the terminal REPL (`nstn-cli`)
- `target/debug/nstn-server` — the HTTP/gRPC server (`nstn-server`)

---

## 4. Run Tests

```bash
# All workspace tests
cargo test --workspace

# A specific crate
cargo test -p nstn-common

# With output shown
cargo test --workspace -- --nocapture
```

The full quality check (run before committing):

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

---

## 5. Try the CLI with Different Providers

The CLI binary is `nanosistant`. After building it is at `target/debug/nanosistant`.

**With Anthropic (recommended):**

```bash
export ANTHROPIC_API_KEY=sk-ant-...
./target/debug/nanosistant
```

**With OpenAI:**

```bash
export OPENAI_API_KEY=sk-...
./target/debug/nanosistant
```

**With Azure OpenAI:**

```bash
export AZURE_OPENAI_API_KEY=your-key
export AZURE_OPENAI_BASE_URL=https://your-instance.openai.azure.com/openai/deployments/your-deployment
./target/debug/nanosistant
```

**With xAI (Grok):**

```bash
export XAI_API_KEY=xai-...
./target/debug/nanosistant
```

**With a local Ollama model (no API key):**

```bash
export OPENAI_BASE_URL=http://localhost:11434/v1
./target/debug/nanosistant
```

See [Model Providers](./model-providers.md) for full provider documentation and auto-detection logic.

---

## 6. Try Deterministic Queries (No API Key Needed)

Nanosistant resolves many queries with pure Rust code — no LLM tokens consumed, no API key required. Try these in the REPL or as one-shot prompts:

```bash
# Music theory
./target/debug/nanosistant --print "c major scale"
# → "C major scale: C - D - E - F - G - A - B"

./target/debug/nanosistant --print "140 bpm bar duration"
# → "At 140 BPM (4/4): one bar = 1.714s"

./target/debug/nanosistant --print "Am in C major"
# → "Am in C major = vi"

./target/debug/nanosistant --print "2500hz band"
# → "2500 Hz → Upper Mids"

# Utilities
./target/debug/nanosistant --print "what time is it"
# → "Current time: 2026-04-02T13:26:00Z"

./target/debug/nanosistant --print "word count: hello world foo"
# → "3 words"

# Finance
./target/debug/nanosistant --print "percentage change from 100 to 150"
# → "+50.00%"
```

These all resolve via `nstn_common::try_deterministic_resolution()` — zero network, zero tokens.

You can also call the underlying functions directly from Rust code:

```rust
use nstn_common::{scale_degrees, bpm_to_bar_duration, chord_to_roman};

let scale = scale_degrees("C", "major");
assert_eq!(scale, ["C", "D", "E", "F", "G", "A", "B"]);

let bar_dur = bpm_to_bar_duration(140, 4);
assert!((bar_dur - 1.714).abs() < 0.001);

let roman = chord_to_roman("Am", "C");
assert_eq!(roman, "vi");
```

---

## 7. Add a New Domain Agent

The quickest path is to copy an existing agent config and customise it. For a complete walkthrough, see [Adding a Domain Agent](./adding-a-domain.md).

```bash
# Copy the general agent as a starting point
cp config/agents/general.toml config/agents/fitness.toml
```

Edit `config/agents/fitness.toml`:

```toml
[agent]
name = "fitness"
description = "Personal training and nutrition specialist"
model = "claude-sonnet-4-20250514"
permission_mode = "read_only"

[agent.triggers]
keywords = [
    "workout", "training", "exercise", "nutrition",
    "calories", "protein", "cardio", "strength",
]
priority = 10

[agent.prompt]
identity_file = "config/prompts/identity.md"
domain_file = "config/prompts/fitness.md"

[agent.tools]
include = []
deterministic = []
```

Create the domain prompt at `config/prompts/fitness.md`:

```markdown
# Fitness Agent

You are a personal training and nutrition specialist...
```

Rebuild and test routing:

```bash
cargo build --workspace
./target/debug/nanosistant --print "how many calories in 100g of chicken"
```

---

## 8. Run with Docker

The Docker setup includes the HTTP/gRPC server and a Qdrant vector database.

**Build and start:**

```bash
docker compose up --build
```

This starts:
- `nanosistant` — HTTP server on port 3000, gRPC on port 50051
- `qdrant` — Qdrant vector DB on ports 6333 (HTTP REST) and 6334 (gRPC)

**Check health:**

```bash
curl http://localhost:3000/health
```

**Environment variables:** Pass your API key into the container:

```bash
ANTHROPIC_API_KEY=sk-ant-... docker compose up
```

Or add it to a `.env` file at the project root (not committed to git):

```bash
echo "ANTHROPIC_API_KEY=sk-ant-..." >> .env
docker compose up
```

**Enable Qdrant backend:** Edit `config/settings.toml` and uncomment:

```toml
[knowledge]
qdrant_url = "http://qdrant:6334"
store_type = "qdrant"
```

Then restart the compose stack.

**Production notes:**
- Bind-mount your config directory to `/etc/nanosistant/config`.
- Bind-mount a data volume to `/var/lib/nanosistant` for session and memory persistence.
- Set `RUST_LOG=info` (or `debug` for troubleshooting).
- The gRPC server on port 50051 is for `nstn-nanoclaw` (edge tier) and iOS client connections.
