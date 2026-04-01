# Nanosistant (NSTN-1)

**A self-hosted, sovereignty-first, general-purpose personal AI system.**

> Nanosistant is NOT a music project, NOT a coding assistant, NOT an investment bot.
> It is all of those and anything else the operator needs — a 24/7 personal intelligence
> layer that lives across data sources, orchestrates any task domain, and keeps control
> with the user at all times.

```
User (any device)
    │
    ▼
┌──────────────────────────────────┐
│         NanoClaw (Edge)          │  ← local-first, offline-capable
│  Deterministic resolution        │
│  Offline queue, sync-on-connect  │
└──────────────┬───────────────────┘
               │ gRPC / protobuf
               ▼
┌──────────────────────────────────┐
│          RuFlo (Brain)           │  ← Rust wraps confidence ladder + ruflo
│                                  │
│  ┌────────────────────────────┐  │
│  │  Deterministic intercept   │  │  zero tokens, pure code
│  │  Confidence Ladder         │  │  AC → Regex → Weighted → Fuzzy
│  │  ruflo MCP fallback        │  │  Q-learning / MoE / Semantic
│  │  Budget + Watchdog         │  │
│  └──┬────────┬────────┬──────┘  │
│     ▼        ▼        ▼         │
│  [Agent A] [Agent B] [Agent C]  │  config-driven, domain-scoped
└──────────────┬───────────────────┘
               │ gRPC / protobuf
               ▼
┌──────────────────────────────────┐
│       RuVector (Knowledge)       │  ← vector search, RAG, document ingestion
│  Domain-filtered hybrid search   │
└──────────────────────────────────┘
```

---

## Quick Start

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- `protoc` (protobuf compiler) — `apt install protobuf-compiler` or `brew install protobuf`
- Node.js 20+ (optional, for ruflo MCP backend)

### Build

```bash
git clone --recurse-submodules https://github.com/PrinceJonaa/nanosistant.git
cd nanosistant
cargo build --workspace
```

### Test

```bash
cargo test --workspace
```

### Verify

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

---

## Architecture

### Design Principles

1. **Sovereignty** — All data stays on user-controlled infrastructure. No vendor lock-in.
2. **Presence over performance** — Collapse the distance between intention and execution.
3. **Subtraction before addition** — Don't build infrastructure you don't need yet.
4. **Deterministic backbone, scoped intelligence** — The orchestrator is code, not an LLM.
5. **Verify before asserting absence** — Research it first.
6. **Produce the artifact, not a report about it** — Build the thing.

### Three-Tier Design

| Tier | Crate | What It Does |
|------|-------|-------------|
| **Edge** | `nstn-nanoclaw` | Local-first resolution, gRPC client, offline queue, sync-on-connect |
| **Brain** | `nstn-ruflo` | Deterministic orchestrator, confidence-ladder router, ruflo MCP bridge, budget manager, watchdog |
| **Knowledge** | `nstn-ruvector` | In-memory vector store, document ingestion (## header splitting), domain-filtered search |

### Routing Pipeline

The orchestrator routes every message through a confidence ladder before touching an LLM:

| Tier | Engine | Complexity | What It Catches |
|------|--------|-----------|-----------------|
| 0 | Deterministic functions | O(1) | Closed-form queries (BPM calc, chord lookup, dates) |
| 1 | Aho-Corasick automaton | O(n+z) | Exact keyword/phrase patterns |
| 2 | Compiled regex bank | O(n×R) | Morphological variants (compilation, refactoring) |
| 3 | Weighted keyword map | O(n) | Dynamic scoring, updatable at runtime |
| 4 | Levenshtein distance | O(n×A) | Typo recovery (refactr → refactor) |
| 5 | Combined score | — | Weighted blend of all tiers |
| 6 | ruflo MCP | — | Q-learning, MoE, semantic routing (via stdio JSON-RPC) |
| 7 | LLM classifier | — | Final escape hatch for genuine ambiguity |

### ruflo Integration

[ruflo](https://github.com/ruvnet/ruflo) (v3.5) is integrated as a git submodule under `vendor/ruflo`. The Rust orchestrator communicates with ruflo via MCP (JSON-RPC 2.0 over stdio). When the confidence ladder returns Ambiguous, ruflo's full routing stack fires: Q-learning, Mixture-of-Experts, semantic routing, and 205+ MCP tools.

Ruflo runs as a child process — Rust is always the entry point, always the exit point.

---

## Crates

| Crate | Package | Lines | Tests | Description |
|-------|---------|-------|-------|-------------|
| `crates/common` | `nstn-common` | ~3,500 | 59 | Proto types, deterministic functions (30+), domain classifier, confidence-ladder router, events, handoff validation |
| `crates/runtime` | `nstn-runtime` | ~4,800 | 49 | Forked ConversationRuntime (claw-code), hooks, permissions, sessions, compaction, usage tracking |
| `crates/api` | `nstn-api` | ~3,200 | 36 | LLM API client — Anthropic + OpenAI-compatible providers, SSE streaming |
| `crates/tools` | `nstn-tools` | ~4,500 | 30 | Tool definitions (bash, file ops, web, search), execution framework |
| `crates/plugins` | `nstn-plugins` | ~2,600 | 25 | Plugin system with pre/post hook pipeline |
| `crates/ruflo` | `nstn-ruflo` | ~2,500 | 68 | Orchestrator, confidence ladder, ruflo MCP bridge/proxy, budget, watchdog |
| `crates/nanoclaw` | `nstn-nanoclaw` | ~500 | 9 | Edge runtime, gRPC client (stub), offline queue |
| `crates/ruvector` | `nstn-ruvector` | ~600 | 11 | Vector store, document ingestion, domain-filtered search |
| `crates/server` | `nstn-server` | ~500 | 2 | axum HTTP/SSE server |

**Total: 365+ tests, ~31,200 lines across 69 source files (Rust + Swift).**

---

## Domain Agents

Agents are config-driven — add a new domain by creating a `.toml` file and a prompt `.md` file. Zero code changes.

| Domain | Config | What It Covers |
|--------|--------|---------------|
| General | `config/agents/general.toml` | Fallback for unmatched queries |
| Music | `config/agents/music.toml` | Songwriting, production, mixing, mastering |
| Investment | `config/agents/investment.toml` | Trading, equity research, financial analysis |
| Development | `config/agents/development.toml` | Coding, architecture, debugging |
| Framework | `config/agents/framework.toml` | Distortion Lattice, archetype analysis |

### Adding a New Domain

```toml
# config/agents/your_domain.toml
[agent]
name = "your_domain"
description = "What this agent does"
model = "claude-sonnet-4-20250514"
permission_mode = "workspace_write"

[agent.triggers]
keywords = ["keyword1", "keyword2", "multi word phrase"]
priority = 10

[agent.prompt]
identity_file = "config/prompts/identity.md"
domain_file = "config/prompts/your_domain.md"

[agent.tools]
include = ["bash", "read_file", "write_file", "WebSearch"]
```

---

## Deterministic Functions

30+ functions that run as pure code (zero tokens). Available to all agents:

**Music:** `bpm_to_bar_duration`, `song_bar_count`, `scale_degrees`, `chord_to_roman`, `roman_to_chord`, `transpose`, `note_to_frequency`, `frequency_to_band`, `syllable_count`, `density_lambda`

**Finance:** `percentage_change`, `compound_annual_growth`, `position_size`

**Business:** `release_timeline`, `isrc_validate`, `streaming_loudness_check`

**Universal:** `current_datetime`, `days_until`, `word_count`, `reading_time_minutes`, `json_validate`, `url_validate`

---

## Protobuf Contracts

All cross-tier communication uses protobuf. Contracts are defined in `proto/nanosistant.proto`:

- `EdgeRequest` / `EdgeResponse` — NanoClaw ↔ RuFlo
- `KnowledgeQuery` / `KnowledgeResult` — RuFlo ↔ RuVector
- `AgentHandoff` — typed inter-agent handoffs with validation
- `Event` — structured event logging
- `NanoClawService` / `RuVectorService` — gRPC service definitions

---

## Development

### Project Structure

```
nanosistant/
├── CLAUDE.md              # AI coding rules
├── AGENTS.md              # Multi-agent development guide
├── Cargo.toml             # Workspace root
├── proto/                 # Protobuf contracts
├── config/
│   ├── agents/            # Agent TOML configs
│   ├── prompts/           # System prompts per domain
│   └── settings.toml      # Global settings
├── crates/
│   ├── common/            # Proto types, deterministic functions, router
│   ├── runtime/           # ConversationRuntime (forked from claw-code)
│   ├── api/               # LLM API client
│   ├── tools/             # Tool definitions
│   ├── plugins/           # Plugin system
│   ├── ruflo/             # Orchestrator + ruflo MCP bridge
│   ├── nanoclaw/          # Edge tier
│   ├── ruvector/          # Knowledge tier
│   └── server/            # HTTP/SSE server
├── vendor/
│   └── ruflo/             # git submodule → github.com/ruvnet/ruflo
├── tests/
│   └── integration/       # Cross-crate integration tests
└── .github/
    └── workflows/         # CI/CD
```

### Code Standards

- Zero `clippy` warnings
- Every public function has tests
- Comments explain WHY, not what
- Files stay focused — one responsibility per module
- Protobuf is the tier boundary. Crates never import each other's internals across tiers.

---

## Roadmap

### v0.1.0
- [x] Three-tier architecture (NanoClaw, RuFlo, RuVector)
- [x] Confidence-ladder router (AC → Regex → Weighted → Fuzzy)
- [x] ruflo MCP integration (submodule + stdio bridge)
- [x] 30+ deterministic functions
- [x] Config-driven domain agents
- [x] Protobuf contracts
- [x] 289 passing tests

### v0.2.0
- [x] Real gRPC transport (tonic) for NanoClaw ↔ RuFlo
- [x] Qdrant integration for RuVector (with in-memory fallback)
- [x] Embedding-based semantic search (HashEmbedding + cosine similarity)
- [x] MCP tool server in RuVector (4 tools: query, ingest, domains, stats)
- [x] ConversationRuntimes wired into orchestrator (AgentRuntime trait + execute())
- [x] ruflo swarm coordination (spawn, status, coordinate, cancel)
- [x] 350 passing tests

### v0.3.0 (current)
- [x] iOS NanoClaw client (Swift Package, 2,100 lines)
- [x] Knowledge ingestion pipeline (TOML-configurable, directory recursive)
- [x] Session persistence (JSON per-session, save/load/delete/recent)
- [x] Production deployment (Dockerfile, docker-compose, health endpoint)
- [x] 365+ tests across Rust and Swift

### v0.4.0 (next)
- [ ] Real LLM API integration (wire ConversationRuntime to Anthropic API)
- [ ] Live ruflo MCP connection (spawn and test end-to-end)
- [ ] Native gRPC in Swift client (replace HTTP bridge)
- [ ] Knowledge ingestion for operator’s framework docs (all 22 files)
- [ ] Session sync across devices via gRPC

---

## License

Copyright (c) 2026 Prince Jona / Intervised LLC. All rights reserved.

This software is proprietary and confidential. See [LICENSE](LICENSE) for details.

---

## Author

**Prince Jona** — [Intervised LLC](https://intervised.com)
