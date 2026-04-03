# Nanosistant

[![CI](https://github.com/PrinceJonaa/nanosistant/actions/workflows/ci.yml/badge.svg)](https://github.com/PrinceJonaa/nanosistant/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/version-0.7.0-teal)](https://github.com/PrinceJonaa/nanosistant/releases)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://rustup.rs)
[![Tests](https://img.shields.io/badge/tests-840%20passing-green)](https://github.com/PrinceJonaa/nanosistant/actions)
[![License](https://img.shields.io/badge/license-proprietary-blue)](LICENSE)

Most AI systems reach for a language model first. Nanosistant reaches for one last.

It's a personal AI runtime built in Rust — three tiers, eight routing layers, 316 deterministic functions, and a typed memory system. Before any token gets spent, your query passes through a confidence ladder of pure math: Aho-Corasick, regex, fuzzy matching, domain scoring. The LLM only fires when every deterministic path has genuinely failed to answer. The orchestrator is always code. It never hands control to a prompt.

→ **[NSTN Hub](https://princejonaa.github.io/nanosistant/)** — browse and install community packs

---

## The Confidence Ladder

Every query travels the same path. Each tier short-circuits the moment it's confident — the ones below it never run.

```
Query
  │
  ▼
① Deterministic packs ─── O(1)      316 pure functions across 14 domains — no tokens, no latency
② Aho-Corasick ─────────── O(n+z)   Exact keyword and phrase matching at line speed
③ Regex bank ───────────── O(n×R)   Morphological variants — catches "refactoring", "compiling", etc.
④ Weighted keywords ─────── O(n)    Domain scoring updated dynamically at runtime
⑤ Fuzzy / Levenshtein ───── O(n×A)  Typo recovery: "trnaspose" → "transpose"
⑥ Combined score ───────────  —     Weighted blend of tiers 2–5
⑦ ruflo MCP ────────────────  —     Q-learning policy, Mixture-of-Experts, semantic routing
⑧ LLM classifier ──────────   —     True last resort — only fires on genuine ambiguity
```

Tiers 1–6 are deterministic and stateless. Tier 7 is ruflo — an MCP runtime with Q-learning and 205+ tools, running as a child process that Rust spawns and controls. Tier 8 is the LLM — powerful, expensive, and rarely needed.

---

## Architecture

The system runs as three tiers connected by gRPC and protobuf contracts. No tier imports another's internals. Boundaries are enforced at the type level.

```
User (iOS · CLI · HTTP)
    │
    ▼  gRPC / protobuf
┌────────────────────────────────────┐
│   NanoClaw  ·  Edge                │
│   Local-first, offline queue       │
│   NanoClawKit iOS Swift client     │
└──────────────┬─────────────────────┘
               │ gRPC
               ▼
┌────────────────────────────────────┐
│   RuFlo  ·  Brain                  │
│   Confidence ladder (tiers 1–8)    │
│   ruflo MCP bridge                 │
│   Memory L0 → L1 → L2 → L3        │
│   Watchdog · Budget · Dreamer      │
│   Typed-IR validation              │
└──────────────┬─────────────────────┘
               │ gRPC
               ▼
┌────────────────────────────────────┐
│   RuVector  ·  Knowledge           │
│   Qdrant, hash embeddings, RAG     │
└────────────────────────────────────┘
```

| Tier | Crate | What it owns |
|------|-------|-------------|
| **Edge** — NanoClaw | `nstn-nanoclaw` | Local resolution, offline queue, sync-on-connect, iOS client |
| **Brain** — RuFlo | `nstn-ruflo` | Router, memory, budget, watchdog, dreamer, typed-IR |
| **Knowledge** — RuVector | `nstn-ruvector` | Vector store, embeddings, document ingestion, domain RAG |

---

## Getting Started

### Prerequisites

- **Rust 1.75+** — [rustup.rs](https://rustup.rs)
- **protoc** — `brew install protobuf` / `sudo apt install protobuf-compiler`
- **Node.js 20+** _(optional)_ — only required for live ruflo MCP routing at tier 7

### Install

```bash
git clone --recurse-submodules https://github.com/PrinceJonaa/nanosistant.git
cd nanosistant
cargo build --workspace
```

### Run

```bash
# Interactive REPL — vim keybindings, markdown rendering, 28 slash commands
cargo run --bin nanosistant

# Pipe mode
echo "what are the delay times at 140 BPM?" | cargo run --bin nanosistant
```

### Verify

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Docker (full stack with Qdrant)

```bash
docker compose up
```

---

## Model Providers

Nanosistant works with any OpenAI-compatible provider. Configure via `config/settings.toml` or environment variables:

| Provider | Env var | Model |
|----------|---------|-------|
| Anthropic | `ANTHROPIC_API_KEY` | `claude-opus-4-5` |
| OpenAI | `OPENAI_API_KEY` | `gpt-4o` |
| Azure OpenAI | `AZURE_OPENAI_API_KEY` + `AZURE_OPENAI_ENDPOINT` | `gpt-4o` |
| xAI (Grok) | `XAI_API_KEY` | `grok-3` |
| Ollama (local) | _(set base URL, no key needed)_ | `llama3.2`, `qwen2.5-coder` |

```toml
# config/settings.toml
[model]
default  = "claude-opus-4-5"
fallback = "llama3.2"   # used when offline
```

Override at runtime: `nanosistant --model ollama/llama3.2`

---

## Packs

A pack is a portable bundle of deterministic functions and routing rules. It can be a TOML rule file, a compiled Rust fn, or both. Packs are installed locally — they extend the confidence ladder's tier 1 without touching any other layer.

The community registry lives at [NSTN Hub](https://princejonaa.github.io/nanosistant/).

### Using packs

```bash
nanosistant /packs install nstn-music
nanosistant /packs list
nanosistant /packs remove nstn-music
```

### Writing a pack

```
my-pack/
├── pack.toml      # Metadata, routing config, compatibility range
├── rules.toml     # Declarative rule evaluator (optional)
└── src/
    └── lib.rs     # Native Rust functions (optional)
```

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

Open a PR to `packs/` — CI validates `pack.toml` automatically. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full checklist.

---

## Built-in Packs

14 packs ship with the repo. All 316 functions are pure, stateless, and cost zero tokens.

### Universal

These packs cover math and reasoning primitives that apply across every domain.

| Pack | Fns | Coverage |
|------|-----|----------|
| `nstn-logic` | 18 | Boolean algebra, set operations, propositional calculus, inference rules |
| `nstn-graph` | 14 | Shortest path, cycle detection, topological sort, spanning trees |
| `nstn-information` | 10 | Shannon entropy, KL divergence, mutual information, channel capacity |
| `nstn-probability` | 22 | Bayes theorem, distributions, combinatorics, expected value, hypothesis testing |

### Domain

These packs handle calculations that belong to a specific field — the kind of thing people constantly ask AI about but which never needed a language model in the first place.

| Pack | Fns | Coverage |
|------|-----|----------|
| `nstn-music` | 28 | BPM math, bar duration, note frequencies, scales, chords, intervals |
| `nstn-finance` | 35 | Options pricing, Sharpe ratio, compound interest, volatility, risk metrics |
| `nstn-data` | 30 | Descriptive stats, correlation, percentiles, z-scores, normalization, outliers |
| `nstn-time` | 24 | Timezone conversion, duration arithmetic, calendar calculations, ISO 8601 |
| `nstn-text` | 20 | Word count, readability scores, syllables, character frequency, similarity |
| `nstn-code` | 25 | Semver parsing/comparison, base64/hex encoding, hashing, UUID generation |
| `nstn-geo` | 18 | Haversine distance, coordinate conversion, bounding boxes, bearing |
| `nstn-physics` | 32 | Kinematics, thermodynamics, optics, electromagnetism, unit conversions |
| `nstn-health` | 20 | BMI, BMR, target heart rate, VO2 max, calorie estimation, macros |
| `nstn-social` | 16 | Engagement rates, influence scoring, centrality measures, network metrics |

**316 functions · 100% test coverage across all 14 packs · $0.00 per call**

---

## Memory

Nanosistant maintains four memory levels. Each one has a different scope, lifetime, and backing store — and each one is explicitly typed so nothing leaks between them.

| Level | Name | Scope | How it persists |
|-------|------|-------|-----------------|
| **L0** | Working | Single turn | In-process. Slot-limited. Evicted when the turn ends. |
| **L1** | Episodic | Session | JSON on disk. Full message history. Load, save, delete, resume. |
| **L2** | Semantic | Domain | Qdrant or in-memory. Vector-embedded knowledge, queried on every turn. |
| **L3** | Identity | Operator | Config + TOML. System prompt, persona, operator rules. Immutable at runtime. |

At the end of each session, a consolidation loop replays L1 and looks for five failure patterns from the MAST taxonomy: StuckLoop, TokenWaste, HandoffFailure, BudgetBlindness, SpecRepetition. Anything worth keeping gets promoted to L2. The same watchdog runs live during sessions and trips circuit breakers before failures compound.

---

## Slash Commands

The REPL ships 28 built-in commands:

| Category | Commands |
|----------|---------|
| Session | `/help`, `/status`, `/clear`, `/cost`, `/compact`, `/model`, `/version` |
| Memory | `/memory`, `/resume`, `/config`, `/init` |
| Code | `/diff`, `/branch`, `/worktree`, `/commit`, `/commit-push-pr`, `/pr`, `/issue` |
| Power | `/ultraplan`, `/teleport`, `/bughunter`, `/agents`, `/skills`, `/plugins`, `/permissions` |
| Packs | `/packs install`, `/packs list`, `/packs remove` |

---

## Crates

| Crate | Package | What it does |
|-------|---------|-------------|
| `crates/common` | `nstn-common` | All 14 deterministic modules, confidence-ladder router, domain classifier, proto types |
| `crates/runtime` | `nstn-runtime` | Agent loop, hooks, permissions, sessions, compaction, usage tracking |
| `crates/api` | `nstn-api` | LLM client — Anthropic + OpenAI-compatible, SSE streaming |
| `crates/tools` | `nstn-tools` | Tool definitions (bash, file ops, web, search) and execution framework |
| `crates/plugins` | `nstn-plugins` | Plugin system with pre/post hook pipeline |
| `crates/ruflo` | `nstn-ruflo` | Orchestrator, confidence ladder, ruflo MCP bridge, budget, watchdog, dreamer |
| `crates/nanoclaw` | `nstn-nanoclaw` | Edge runtime, gRPC client, offline queue, sync-on-connect |
| `crates/ruvector` | `nstn-ruvector` | Qdrant vector store, hash embeddings, document ingestion, domain search |
| `crates/server` | `nstn-server` | axum HTTP/SSE server, health endpoint |
| `crates/packs` | `nstn-packs` | Pack loader, TOML rule evaluator, Rust fn registry, marketplace client |
| `crates/memory` | `nstn-memory` | L0–L3 memory types, consolidation loop, MAST watchdog |
| `crates/cli` | `nstn-cli` | REPL, vim keybindings, markdown rendering, slash command registry |
| `crates/proto` | `nstn-proto` | Protobuf codegen via tonic-build |
| `crates/typed-ir` | `nstn-typed-ir` | Typed intermediate representation — validates LLM proposals before execution |
| `clients/ios` | `NanoClawKit` | Swift Package — iOS 17+ / macOS 14+ edge client |

**14 Rust crates · 1 Swift package · 840 tests · 54,000+ lines**

---

## Project Layout

```
nanosistant/
├── Cargo.toml
├── proto/                 # Protobuf contracts — the only interface between tiers
├── config/
│   ├── settings.toml      # Global config: model, memory, budget
│   ├── agents/            # Per-domain agent configs
│   └── prompts/           # System prompts per domain
├── crates/                # 14 Rust crates
├── clients/
│   └── ios/NanoClawKit/   # Swift Package
├── packs/
│   ├── universal/         # logic · graph · information · probability
│   └── domain/            # music · finance · data · time · text · code · geo · physics · health · social
├── hub/                   # NSTN Hub — static site served via GitHub Pages
├── vendor/
│   └── ruflo/             # git submodule: github.com/ruvnet/ruflo
├── tests/integration/
└── .github/
    ├── workflows/         # CI, release, security, pages, pack validation
    └── ISSUE_TEMPLATE/
```

---

## ruflo

[ruflo](https://github.com/ruvnet/ruflo) is a SOTA MCP runtime with Q-learning policy routing, Mixture-of-Experts model selection, semantic tool search, and 205+ registered tools. It runs as a git submodule under `vendor/ruflo`.

Nanosistant spawns ruflo as a child process and speaks to it over JSON-RPC 2.0 via stdio. Rust is the entry point. Rust is the exit point. ruflo never receives a request directly — it only runs when the Rust orchestrator decides tier 7 is warranted and hands it a scoped task. Control always returns to Rust.

If Node.js isn't available, the system falls back to the LLM at tier 8. ruflo is powerful but optional.

---

## Roadmap

| Version | Status | What shipped |
|---------|--------|-------------|
| v0.1 | ✓ | 3-tier architecture, confidence ladder, ruflo MCP, 30+ deterministic functions |
| v0.2 | ✓ | Real gRPC (tonic), Qdrant, hash embeddings, MCP tool server, ruflo swarm coordination |
| v0.3 | ✓ | iOS NanoClawKit (Swift), knowledge ingestion pipeline, session persistence, Docker |
| v0.4 | ✓ | Full MCP client (6 transports), filesystem sandbox, 28 slash commands, OAuth PKCE, LSP |
| v0.5 | ✓ | Live LLM API end-to-end, ruflo live routing, native gRPC in Swift, cross-device sync |
| v0.6 | ✓ | L0–L3 typed memory, Dreamer consolidation loop, MAST watchdog, Typed-IR, 7 domain modules |
| v0.7 | ✓ | All 14 packs, nstn-packs crate, operator TOML runtime, NSTN Hub, 840 tests |
| v0.8 | next | Public pack registry API, multi-operator federation, WASM pack sandbox |

---

## Principles

These aren't aspirational — they're the constraints the codebase was built around.

1. **Sovereignty** — Data stays on hardware you control. No cloud dependency, no vendor lock-in.
2. **Deterministic backbone** — The orchestrator is code, not a prompt. LLMs propose. Rust decides.
3. **Presence over performance** — Minimize the distance between what you intend and what executes.
4. **Typed boundaries** — Protobuf at every tier crossing. Crates don't reach into each other's internals.
5. **Subtraction before addition** — Build when pain demands it. Remove before you add.
6. **Produce the artifact** — Ship the thing. Not a plan. Not a report. The thing.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, code standards, architecture rules, and the pack submission checklist.

For questions or to share what you've built — [GitHub Discussions](https://github.com/PrinceJonaa/nanosistant/discussions).

---

## License

Copyright © 2026 Prince Jona / Intervised LLC. All rights reserved.

Source-available. Open-source release is planned. See [LICENSE](LICENSE).

---

**[NSTN Hub](https://princejonaa.github.io/nanosistant/) · [Discussions](https://github.com/PrinceJonaa/nanosistant/discussions) · [Intervised LLC](https://intervised.com)**
