# Nanosistant

[![CI](https://github.com/PrinceJonaa/nanosistant/actions/workflows/ci.yml/badge.svg)](https://github.com/PrinceJonaa/nanosistant/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/version-0.7.0-teal)](https://github.com/PrinceJonaa/nanosistant/releases)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://rustup.rs)
[![Tests](https://img.shields.io/badge/tests-840%20passing-green)](https://github.com/PrinceJonaa/nanosistant/actions)
[![License](https://img.shields.io/badge/license-proprietary-blue)](LICENSE)

**Sovereignty-first personal AI system written in Rust. Deterministic by default — LLM only when nothing else will do.**

Nanosistant is a three-tier distributed AI runtime with a 7-tier confidence-ladder router, 14 deterministic function packs, a typed memory system, and a community pack registry — all running on hardware you control, with no vendor lock-in.

→ **[NSTN Hub](https://princejonaa.github.io/nanosistant/)** — browse and install deterministic function packs

---

## How It Works

Every query passes through a deterministic confidence ladder. Tiers short-circuit upward the moment a confident answer is found. The LLM is tier 7 — the last resort, not the first.

```
Query
  │
  ▼
① Deterministic functions ─── O(1)     BPM math, chord lookup, date calc, geospatial → instant answer
② Aho-Corasick automaton ──── O(n+z)   Exact keyword/phrase patterns
③ Compiled regex bank ──────── O(n×R)   Morphological variants (refactoring, compilation…)
④ Weighted keyword map ──────── O(n)    Dynamic domain scoring, updatable at runtime
⑤ Fuzzy / Levenshtein ──────── O(n×A)  Typo recovery: "refactr" → "refactor"
⑥ Combined score ─────────────   —     Weighted blend of tiers 1–4
⑦ ruflo MCP ──────────────────   —     Q-learning policy, Mixture-of-Experts, semantic routing
⑧ LLM classifier ─────────────   —     Escape hatch for genuine ambiguity only
```

**The orchestrator is always Rust code. It never delegates control to a prompt.**

---

## Architecture

```
User (iOS · CLI · HTTP)
    │
    ▼  HTTPS / gRPC / CLI
┌───────────────────────────────────────┐
│        NanoClaw  ·  Edge Tier         │
│  Local-first · Offline queue          │
│  iOS Swift client (NanoClawKit)       │
└──────────────────┬────────────────────┘
                   │ gRPC + protobuf
                   ▼
┌───────────────────────────────────────┐
│          RuFlo  ·  Brain Tier         │
│                                       │
│  Confidence ladder (tiers 1–8)        │
│  ruflo MCP bridge (Q-learning/MoE)    │
│  Memory: L0 → L1 → L2 → L3           │
│  Watchdog · Budget · Dreamer          │
│  Typed-IR validation                  │
└──────────────────┬────────────────────┘
                   │ gRPC + protobuf
                   ▼
┌───────────────────────────────────────┐
│       RuVector  ·  Knowledge Tier     │
│  Qdrant · Hash embeddings · RAG       │
└───────────────────────────────────────┘
```

| Tier | Crate | Role |
|------|-------|------|
| **Edge** — NanoClaw | `nstn-nanoclaw` | Local-first resolution, offline queue, sync-on-connect, iOS client |
| **Brain** — RuFlo | `nstn-ruflo` | Confidence-ladder router, ruflo MCP bridge, budget, watchdog, dreamer |
| **Knowledge** — RuVector | `nstn-ruvector` | Qdrant vector store, hash embeddings, document ingestion, domain RAG |

---

## Quick Start

### Prerequisites

- **Rust 1.75+** — [rustup.rs](https://rustup.rs)
- **protoc** — `brew install protobuf` / `sudo apt install protobuf-compiler`
- **Node.js 20+** _(optional)_ — only needed for live ruflo MCP routing

### Clone and Build

```bash
git clone --recurse-submodules https://github.com/PrinceJonaa/nanosistant.git
cd nanosistant
cargo build --workspace
```

### Run the CLI

```bash
cargo run --bin nanosistant
```

The `nanosistant` binary starts an interactive REPL with vim keybindings, markdown rendering, syntax highlighting, and 28 slash commands (`--help` for all flags).

### Pipe mode

```bash
echo "BPM duration of 120bpm in 4/4?" | cargo run --bin nanosistant
```

### Verify

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Docker (full stack + Qdrant)

```bash
docker compose up
```

---

## Model Providers

Any OpenAI-compatible provider works. Set via environment variables or `config/settings.toml`:

| Provider | Env var | Example model |
|----------|---------|---------------|
| Anthropic | `ANTHROPIC_API_KEY` | `claude-opus-4-5` |
| OpenAI | `OPENAI_API_KEY` | `gpt-4o` |
| Azure OpenAI | `AZURE_OPENAI_API_KEY` + `AZURE_OPENAI_ENDPOINT` | `gpt-4o` |
| xAI (Grok) | `XAI_API_KEY` | `grok-3` |
| Ollama (local) | _(none — set base URL)_ | `llama3.2`, `qwen2.5-coder` |

```toml
# config/settings.toml
[model]
default  = "claude-opus-4-5"
fallback = "llama3.2"   # used offline
```

Runtime override: `nanosistant --model ollama/llama3.2`

---

## Pack System

Packs are portable bundles of deterministic functions and routing rules. A pack ships as a TOML rule file, a compiled Rust fn, or both. The community publishes and installs packs through [NSTN Hub](https://princejonaa.github.io/nanosistant/).

### Install a pack

```bash
nanosistant /packs install nstn-music
nanosistant /packs install nstn-finance
nanosistant /packs list
nanosistant /packs remove nstn-music
```

### Pack structure

```
my-pack/
├── pack.toml      # Metadata, routing triggers, compatibility
├── rules.toml     # Declarative TOML rule evaluator (optional)
└── src/
    └── lib.rs     # Native Rust functions (optional)
```

### pack.toml

```toml
[pack]
name        = "my-pack"
version     = "0.1.0"
author      = "your-github-handle"
tier        = "Domain"
domain      = "music"
description = "Extended music theory: modes, voicings, MIDI utilities"
tags        = ["music", "midi", "theory"]

[pack.routing]
keywords             = ["mode", "dorian", "voicing", "midi"]
semantic_description = "Music theory calculations: modes, chord voicings, MIDI"
confidence_threshold = 0.72
```

### Submit a pack

Open a PR adding your pack to `packs/` — the CI workflow validates `pack.toml` automatically. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full checklist.

---

## Built-in Packs

14 packs ship out of the box. All functions are pure, deterministic, and run with zero tokens.

### Universal

| Pack | Functions | What it does |
|------|-----------|-------------|
| `nstn-logic` | 18 | Boolean algebra, set operations, propositional calculus, inference rules |
| `nstn-graph` | 14 | Shortest path, cycle detection, topological sort, spanning trees |
| `nstn-information` | 10 | Shannon entropy, KL divergence, mutual information, channel capacity |
| `nstn-probability` | 22 | Bayes theorem, distributions, combinatorics, expected value, hypothesis testing |

### Domain

| Pack | Functions | What it does |
|------|-----------|-------------|
| `nstn-music` | 28 | BPM math, bar duration, note frequencies, scales, chords, intervals |
| `nstn-finance` | 35 | Options pricing, Sharpe ratio, compound interest, volatility, risk metrics |
| `nstn-data` | 30 | Descriptive stats, correlation, percentiles, z-scores, normalization, outlier detection |
| `nstn-time` | 24 | Timezone conversion, duration arithmetic, calendar calculations, ISO 8601 |
| `nstn-text` | 20 | Word count, readability scores, syllables, character frequency, text similarity |
| `nstn-code` | 25 | Semver parsing/comparison, base64/hex encoding, hashing, UUID generation |
| `nstn-geo` | 18 | Haversine distance, coordinate conversion, bounding boxes, bearing |
| `nstn-physics` | 32 | Kinematics, thermodynamics, optics, electromagnetism, SI/imperial conversions |
| `nstn-health` | 20 | BMI, BMR, target heart rate, VO2 max, calorie estimation, body fat |
| `nstn-social` | 16 | Engagement rates, influence scoring, centrality measures, community metrics |

**316 functions total · 100% test coverage on all 14 packs · zero tokens per call**

---

## Memory System

| Level | Name | Scope | Persistence |
|-------|------|-------|-------------|
| **L0** | Working | Single turn | In-process — slot-limited, evicted on turn completion |
| **L1** | Episodic | Session | JSON on disk — full message history per session |
| **L2** | Semantic | Domain | Qdrant / in-memory — vector-embedded domain knowledge |
| **L3** | Identity | Operator | Config + TOML — system prompt, persona, operator rules |

When a session ends, a consolidation loop replays L1 episodes, detects MAST failure patterns (StuckLoop, TokenWaste, HandoffFailure, BudgetBlindness, SpecRepetition), and promotes durable knowledge to L2. The watchdog monitors live sessions for the same patterns and fires circuit breakers before they compound.

---

## Slash Commands

28 slash commands built into the REPL:

| Category | Commands |
|----------|---------|
| Session | `/help`, `/status`, `/clear`, `/cost`, `/compact`, `/model`, `/version` |
| Memory | `/memory`, `/resume`, `/config`, `/init` |
| Code | `/diff`, `/branch`, `/worktree`, `/commit`, `/commit-push-pr`, `/pr`, `/issue` |
| Power | `/ultraplan`, `/teleport`, `/bughunter`, `/agents`, `/skills`, `/plugins`, `/permissions` |
| Packs | `/packs install`, `/packs list`, `/packs remove` |

---

## Crates

| Crate | Package | Description |
|-------|---------|-------------|
| `crates/common` | `nstn-common` | Deterministic modules (all 14), confidence-ladder router, domain classifier, proto types |
| `crates/runtime` | `nstn-runtime` | ConversationRuntime — agent loop, hooks, permissions, sessions, compaction, usage tracking |
| `crates/api` | `nstn-api` | LLM API client — Anthropic + OpenAI-compatible, SSE streaming |
| `crates/tools` | `nstn-tools` | Tool definitions (bash, file ops, web, search), execution framework |
| `crates/plugins` | `nstn-plugins` | Plugin system with pre/post hook pipeline |
| `crates/ruflo` | `nstn-ruflo` | Orchestrator, confidence ladder, ruflo MCP bridge, budget, watchdog, dreamer |
| `crates/nanoclaw` | `nstn-nanoclaw` | Edge runtime, gRPC client, offline queue, sync-on-connect |
| `crates/ruvector` | `nstn-ruvector` | Qdrant vector store, hash embeddings, document ingestion, domain-filtered search |
| `crates/server` | `nstn-server` | axum HTTP/SSE server, health endpoint |
| `crates/packs` | `nstn-packs` | Pack loader, TOML rule evaluator, Rust fn registry, marketplace client |
| `crates/memory` | `nstn-memory` | L0–L3 memory types, consolidation loop, MAST watchdog |
| `crates/cli` | `nstn-cli` | REPL, vim keybindings, markdown render, slash command registry |
| `crates/proto` | `nstn-proto` | Protobuf codegen (tonic-build) |
| `crates/typed-ir` | `nstn-typed-ir` | Typed intermediate representation — LLM proposal validation |
| `clients/ios` | `NanoClawKit` | Swift Package — iOS 17+ / macOS 14+ edge client |

**14 Rust crates · 1 Swift package · 840 tests · 54,000+ lines**

---

## Project Structure

```
nanosistant/
├── Cargo.toml             # Workspace root
├── README.md
├── CONTRIBUTING.md        # Setup, standards, pack submission checklist
├── CHANGELOG.md           # Full version history
├── AGENTS.md              # Multi-agent development workflow
├── SECURITY.md
├── CODE_OF_CONDUCT.md
├── LICENSE                # Proprietary — Intervised LLC
├── proto/                 # Protobuf contracts (all tier boundaries)
├── config/
│   ├── agents/            # Domain agent TOML configs
│   ├── prompts/           # System prompts per domain
│   ├── ingestion.toml
│   └── settings.toml
├── crates/                # 14 Rust crates
├── clients/
│   └── ios/NanoClawKit/   # Swift Package
├── packs/                 # Built-in pack registry
│   ├── universal/         # logic, graph, information, probability
│   └── domain/            # music, finance, data, time, text, code, geo, physics, health, social
├── hub/                   # NSTN Hub static site (GitHub Pages)
├── vendor/
│   └── ruflo/             # git submodule — github.com/ruvnet/ruflo
├── tests/
│   └── integration/
└── .github/
    ├── workflows/          # CI, release, security audit, pages, pack validation
    ├── ISSUE_TEMPLATE/     # 5 issue forms
    └── PULL_REQUEST_TEMPLATE.md
```

---

## ruflo Integration

[ruflo](https://github.com/ruvnet/ruflo) runs as a git submodule under `vendor/ruflo`. The Rust orchestrator spawns ruflo as a child process and communicates over JSON-RPC 2.0 via stdio (MCP protocol). Rust is always the entry point and always the exit point — ruflo extends capability without ever receiving traffic directly.

When the confidence ladder returns `Ambiguous`, ruflo fires: Q-learning policy selection, Mixture-of-Experts model routing, semantic embedding search, and 205+ registered MCP tools.

Ruflo is optional. Without Node.js, the orchestrator falls back to the LLM classifier at tier 8.

---

## Roadmap

| Version | Status | Highlights |
|---------|--------|------------|
| **v0.1** | ✓ | 3-tier architecture, confidence ladder, ruflo MCP, 30+ deterministic functions |
| **v0.2** | ✓ | Real gRPC (tonic), Qdrant, hash embeddings, MCP tool server, ruflo swarm |
| **v0.3** | ✓ | iOS NanoClawKit Swift client, knowledge ingestion pipeline, session persistence, Docker |
| **v0.4** | ✓ | Full MCP client (6 transports), filesystem sandbox, 28 slash commands, OAuth PKCE, LSP |
| **v0.5** | ✓ | Live LLM API end-to-end, ruflo MCP live routing, gRPC in Swift, cross-device session sync |
| **v0.6** | ✓ | L0–L3 typed memory, Dreamer consolidation loop, MAST watchdog, Typed-IR, 7 domain det/ modules |
| **v0.7** | ✓ | nstn-packs crate, all 14 universal/domain packs, operator tier TOML runtime, NSTN Hub, 840 tests |
| **v0.8** | Next | Public pack registry API, multi-operator federation, WASM pack sandbox, community hub integration |

---

## Design Principles

1. **Sovereignty** — All data stays on user-controlled infrastructure. No vendor lock-in.
2. **Deterministic backbone** — The orchestrator is code, not a prompt. LLMs propose; Rust decides.
3. **Presence over performance** — Collapse the distance between intention and execution.
4. **Typed boundaries** — Protobuf at every tier boundary. Crates never import across tier lines.
5. **Subtraction before addition** — Build when pain demands it, not speculatively.
6. **Produce the artifact** — Ship the thing, not a report about the thing.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, code standards, architecture rules, and the full pack submission checklist.

Questions, ideas, or pack showcase → [GitHub Discussions](https://github.com/PrinceJonaa/nanosistant/discussions)

---

## License

Copyright © 2026 Prince Jona / Intervised LLC. All rights reserved.

Source-available. Open-source release planned. See [LICENSE](LICENSE) for details.

---

**[NSTN Hub](https://princejonaa.github.io/nanosistant/) · [Discussions](https://github.com/PrinceJonaa/nanosistant/discussions) · [Intervised LLC](https://intervised.com)**
