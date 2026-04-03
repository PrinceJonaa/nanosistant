# Nanosistant (NSTN-1)

![CI](https://github.com/PrinceJonaa/nanosistant/workflows/CI/badge.svg)
![Version](https://img.shields.io/badge/version-0.7.0-teal)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
![Tests](https://img.shields.io/badge/tests-840%20passing-green)
![License](https://img.shields.io/badge/license-proprietary-blue)

**Sovereignty-first personal AI OS written in Rust. Deterministic by default — LLM only when nothing else will do.**

Nanosistant is a complete personal intelligence system: three-tier distributed architecture, a 7-tier confidence-ladder router, 14 deterministic function modules, a typed memory system, and a pack marketplace — all running on hardware you control, with no vendor lock-in.

---

## Feature Highlights

→ **Deterministic-first routing** — 840 tests, 54,000+ lines. Closed-form queries (BPM, chord lookup, date math, geospatial) resolve in O(1) with zero tokens spent  
→ **7-tier confidence ladder** — AC automaton → Regex → Weighted keywords → Fuzzy/Levenshtein → ruflo MCP (Q-learning / MoE / Semantic) → LLM classifier. LLMs are the last resort, not the first  
→ **14 deterministic modules** — universal (logic, graph, information, probability) and domain (music, finance, data, time, text, code, geo, physics, health, social)  
→ **L0–L3 typed memory** — working / episodic / semantic / identity, with a dreaming/consolidation loop informed by the MAST failure taxonomy  
→ **ruflo MCP integration** — 205+ tools, Q-learning policy, Mixture-of-Experts routing, semantic embeddings, swarm coordination  
→ **nstn-packs marketplace** — TOML rule evaluator + native Rust fn support. Install, compose, and publish deterministic function packs  
→ **Local-first edge tier** — iOS Swift client (NanoClawKit), offline queue, sync-on-connect, gRPC protobuf transport throughout  
→ **Typed-IR discipline** — LLMs propose structured output; Rust validates and executes. The orchestrator is always code, never a prompt  

---

## Architecture

```
User (any device)
    │
    ▼ HTTPS / gRPC / CLI
┌─────────────────────────────────────────────┐
│          NanoClaw  ·  Edge Tier             │
│  Local-first · Offline queue · iOS client   │
└──────────────────┬──────────────────────────┘
                   │ gRPC + protobuf
                   ▼
┌─────────────────────────────────────────────┐
│            RuFlo  ·  Brain Tier             │
│                                             │
│  ① Deterministic functions (zero tokens)   │
│  ② AC automaton  O(n+z)  Tier 1            │
│  ③ Regex bank              Tier 2          │
│  ④ Weighted keywords       Tier 3          │
│  ⑤ Fuzzy / Levenshtein     Tier 4          │
│  ⑥ ruflo MCP ──────────────Tier 6          │
│     Q-learning · MoE · Semantic             │
│  ⑦ LLM classifier          Tier 7          │
│                                             │
│  Memory: L0 → L1 → L2 → L3                │
│  Watchdog · Budget · Dreamer               │
└──────────────────┬──────────────────────────┘
                   │ gRPC + protobuf
                   ▼
┌─────────────────────────────────────────────┐
│          RuVector  ·  Knowledge Tier        │
│  Qdrant · Embeddings · Domain-filtered RAG  │
└─────────────────────────────────────────────┘
```

### Three-Tier Design

| Tier | Crate | Role |
|------|-------|------|
| **Edge** — NanoClaw | `nstn-nanoclaw` | Local-first resolution, offline queue, sync-on-connect, iOS Swift client (NanoClawKit) |
| **Brain** — RuFlo | `nstn-ruflo` | Confidence-ladder router, ruflo MCP bridge, orchestrator, budget manager, watchdog, dreamer |
| **Knowledge** — RuVector | `nstn-ruvector` | Qdrant-backed vector store, hash embeddings, document ingestion, domain-filtered RAG |

### Routing Pipeline

Every message passes through the confidence ladder in order. Each tier short-circuits upward if it reaches a decision:

| Tier | Engine | Complexity | Catches |
|------|--------|------------|---------|
| 0 | Deterministic functions | O(1) | Closed-form queries — BPM, chord lookup, date math, position sizing |
| 1 | Aho-Corasick automaton | O(n+z) | Exact keyword and phrase patterns |
| 2 | Compiled regex bank | O(n×R) | Morphological variants (refactoring, compilation, etc.) |
| 3 | Weighted keyword map | O(n) | Dynamic domain scoring, updatable at runtime |
| 4 | Levenshtein distance | O(n×A) | Typo recovery — `refactr` → `refactor` |
| 5 | Combined score | — | Weighted blend of tiers 1–4 |
| 6 | ruflo MCP | — | Q-learning policy, Mixture-of-Experts, semantic routing (JSON-RPC 2.0 over stdio) |
| 7 | LLM classifier | — | Escape hatch for genuine ambiguity |

---

## Quick Start

### Prerequisites

- **Rust 1.75+** — install via [rustup.rs](https://rustup.rs)
- **protoc** — `brew install protobuf` (macOS) or `sudo apt install protobuf-compiler` (Debian/Ubuntu)
- **Node.js 20+** (optional) — required only for live ruflo MCP routing

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

The `nanosistant` binary starts an interactive REPL with vim keybindings, markdown rendering, syntax highlighting, and 28 slash commands. Pass `--help` for all flags.

### Non-interactive (pipe) mode

```bash
echo "what is the BPM duration of 120bpm in 4/4?" | cargo run --bin nanosistant
```

### Verify everything passes

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

Nanosistant supports any OpenAI-compatible provider. Configure via environment variables or `config/settings.toml`:

| Provider | Env var | Model example |
|----------|---------|---------------|
| Anthropic | `ANTHROPIC_API_KEY` | `claude-opus-4-5` |
| OpenAI | `OPENAI_API_KEY` | `gpt-4o` |
| Azure OpenAI | `AZURE_OPENAI_API_KEY` + `AZURE_OPENAI_ENDPOINT` | `gpt-4o` |
| xAI (Grok) | `XAI_API_KEY` | `grok-3` |
| Ollama (local) | _(none — set base URL)_ | `llama3.2`, `qwen2.5-coder` |

Set the default model in `config/settings.toml`:

```toml
[model]
default = "claude-opus-4-5"
fallback = "llama3.2"   # Ollama, used when offline
```

Override at runtime: `nanosistant --model ollama/llama3.2`

---

## Pack System (nstn-packs)

Packs are portable bundles of deterministic functions and routing rules. A pack can ship as a TOML rule file, a compiled Rust fn (via dynamic dispatch), or both.

### Install a pack

```bash
nanosistant /packs install music-theory-pro
nanosistant /packs install finance-signals
```

### Pack structure

```
my-pack/
├── pack.toml          # Metadata, dependencies, activation triggers
├── rules.toml         # Declarative TOML rule evaluator
└── src/
    └── lib.rs         # Optional native Rust functions
```

`pack.toml` example:

```toml
[pack]
name = "music-theory-pro"
version = "1.0.0"
author = "Your Name"
description = "Extended music theory functions: modes, voicings, MIDI utilities"

[[function]]
name = "mode_degrees"
trigger_patterns = ["mode", "dorian", "phrygian", "lydian"]
deterministic = true
```

### Browse and publish packs

→ [NSTN Hub](https://princejonaa.github.io/nstn-hub/) — browse, install, and contribute packs  
→ See [CONTRIBUTING.md](CONTRIBUTING.md) for the pack submission checklist

---

## Deterministic Modules

14 modules ship out of the box. All functions are pure, tested, and run with zero tokens.

### Universal modules

| Module | Key functions |
|--------|--------------|
| **Logic** | `bool_eval`, `if_then_else`, `not`, `and_all`, `or_any` |
| **Graph** | `shortest_path`, `topological_sort`, `connected_components`, `degree_centrality` |
| **Information** | `json_validate`, `url_validate`, `word_count`, `reading_time_minutes`, `levenshtein_distance` |
| **Probability** | `bayes_update`, `expected_value`, `entropy`, `combinations`, `permutations` |

### Domain modules

| Module | Key functions |
|--------|--------------|
| **Music** | `bpm_to_bar_duration`, `song_bar_count`, `scale_degrees`, `chord_to_roman`, `roman_to_chord`, `transpose`, `note_to_frequency`, `frequency_to_band`, `syllable_count`, `density_lambda` |
| **Finance** | `percentage_change`, `compound_annual_growth`, `position_size`, `sharpe_ratio`, `max_drawdown` |
| **Data** | `describe`, `normalize`, `zscore`, `percentile_rank`, `rolling_mean` |
| **Time** | `current_datetime`, `days_until`, `parse_duration`, `timezone_convert`, `iso_week` |
| **Text** | `tokenize`, `sentence_split`, `ngrams`, `strip_markdown`, `truncate_tokens` |
| **Code** | `isrc_validate`, `semver_compare`, `glob_match`, `regex_test`, `url_parse` |
| **Geo** | `haversine_distance`, `bbox_contains`, `geocode_lookup`, `timezone_from_coords` |
| **Physics** | `unit_convert`, `db_to_amplitude`, `celsius_to_fahrenheit`, `ohms_law`, `wavelength` |
| **Health** | `bmi`, `bmr_mifflin`, `heart_rate_zone`, `vo2max_estimate`, `macros_from_tdee` |
| **Social** | `reading_level_flesch`, `sentiment_polarity`, `formality_score`, `syllable_stress` |

---

## Memory System

| Level | Name | Scope | Persistence | Notes |
|-------|------|-------|-------------|-------|
| **L0** | Working | Single turn | In-process | Slot-limited; evicted on turn completion |
| **L1** | Episodic | Session | JSON on disk | Full message history per session; load/save/delete/recent |
| **L2** | Semantic | Domain | Qdrant / in-memory | Vector-embedded domain knowledge; queried on every turn |
| **L3** | Identity | Operator | Config + TOML | System prompt, persona, operator rules; immutable at runtime |

### Dreaming / Consolidation

When the session ends, a consolidation loop replays L1 episodes, detects MAST failure patterns (StuckLoop, TokenWaste, HandoffFailure, BudgetBlindness, SpecRepetition), and promotes durable knowledge to L2. The watchdog monitors live sessions for the same five patterns and fires circuit breakers before they compound.

---

## Slash Commands

The `nanosistant` CLI ships 28 slash commands:

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
| `crates/common` | `nstn-common` | Proto types, deterministic modules (all 14), confidence-ladder router, domain classifier, events, handoff validation |
| `crates/runtime` | `nstn-runtime` | ConversationRuntime — agent loop, hooks, permissions, sessions, compaction, usage tracking |
| `crates/api` | `nstn-api` | LLM API client — Anthropic + OpenAI-compatible, SSE streaming |
| `crates/tools` | `nstn-tools` | Tool definitions (bash, file ops, web, search), execution framework |
| `crates/plugins` | `nstn-plugins` | Plugin system with pre/post hook pipeline |
| `crates/ruflo` | `nstn-ruflo` | Orchestrator, confidence ladder, ruflo MCP bridge/proxy, budget, watchdog, dreamer |
| `crates/nanoclaw` | `nstn-nanoclaw` | Edge runtime, gRPC client, offline queue, sync-on-connect |
| `crates/ruvector` | `nstn-ruvector` | Qdrant vector store, hash embeddings, document ingestion, domain-filtered search |
| `crates/server` | `nstn-server` | axum HTTP/SSE server, health endpoint |
| `crates/packs` | `nstn-packs` | Pack loader, TOML rule evaluator, Rust fn registry, marketplace client |
| `crates/memory` | `nstn-memory` | L0–L3 memory types, consolidation loop, MAST watchdog |
| `crates/cli` | `nstn-cli` | REPL, vim keybindings, markdown render, slash command registry |
| `crates/proto` | `nstn-proto` | Protobuf codegen (`tonic-build`) |
| `crates/typed-ir` | `nstn-typed-ir` | Typed intermediate representation — LLM proposal validation |
| `clients/ios` | `NanoClawKit` | Swift Package — iOS 17+ / macOS 14+ edge client |

**14 Rust crates · 1 Swift package · 840 tests · 54,000+ lines**

---

## Project Structure

```
nanosistant/
├── AGENTS.md              # Multi-agent development workflow
├── CONTRIBUTING.md        # Contribution guide
├── CHANGELOG.md           # Full version history
├── Cargo.toml             # Workspace root
├── proto/                 # Protobuf contracts (all tier boundaries)
├── config/
│   ├── agents/            # Domain agent TOML configs
│   ├── prompts/           # System prompts per domain
│   ├── ingestion.toml     # Knowledge ingestion config
│   └── settings.toml      # Global settings
├── crates/                # 14 Rust crates
├── clients/
│   └── ios/NanoClawKit/   # Swift Package (iOS client)
├── packs/                 # Built-in nstn-packs
├── vendor/
│   └── ruflo/             # git submodule — github.com/ruvnet/ruflo v3.5
├── tests/
│   └── integration/       # Cross-crate integration tests
└── .github/
    └── workflows/         # CI, release, security audit, stale
```

---

## Roadmap

| Version | Status | Highlights |
|---------|--------|------------|
| **v0.1** | ✓ Complete | 3-tier architecture, confidence ladder, ruflo MCP, 30+ deterministic functions, protobuf contracts |
| **v0.2** | ✓ Complete | Real gRPC (tonic), Qdrant integration, hash embeddings, MCP tool server, ruflo swarm coordination |
| **v0.3** | ✓ Complete | iOS NanoClawKit Swift client, knowledge ingestion pipeline, session persistence, Docker deploy |
| **v0.4** | ✓ Complete | Full MCP client (6 transports), filesystem sandbox, 28 slash commands, `nanosistant` CLI binary, OAuth PKCE, LSP integration |
| **v0.5** | ✓ Complete | Live LLM API end-to-end, ruflo MCP live routing, native gRPC in Swift, cross-device session sync |
| **v0.6** | ✓ Complete | nstn-packs marketplace (TOML evaluator + Rust fn), L0–L3 typed memory, dreaming/consolidation loop |
| **v0.7** | ✓ Complete | All 14 deterministic modules, typed-IR discipline, 840 tests, 54k+ lines |
| **v0.8** | Next | Public pack registry API, multi-operator federation, WASM pack sandbox, community hub integration |

---

## ruflo Integration

[ruflo](https://github.com/ruvnet/ruflo) (v3.5) runs as a git submodule under `vendor/ruflo`. The Rust orchestrator spawns ruflo as a child process and communicates over JSON-RPC 2.0 via stdio (MCP protocol). Rust is always the entry point and always the exit point — ruflo extends capability without ceding control.

When the confidence ladder returns `Ambiguous`, ruflo's full stack fires: Q-learning policy selection, Mixture-of-Experts model routing, semantic embedding search, and 205+ registered MCP tools.

Ruflo is optional. If Node.js is unavailable, the orchestrator gracefully falls back to the LLM classifier at Tier 7.

---

## Design Principles

1. **Sovereignty** — All data stays on user-controlled infrastructure. No vendor lock-in.
2. **Deterministic backbone** — The orchestrator is code, not a prompt. LLMs propose; Rust decides.
3. **Presence over performance** — Collapse the distance between intention and execution.
4. **Typed boundaries** — Protobuf at every tier boundary. Crates never import each other's internals across tiers.
5. **Subtraction before addition** — Build when pain demands it, not speculatively.
6. **Produce the artifact** — Ship the thing, not a report about it.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, code standards, architecture rules, and the pack submission checklist.

For questions, ideas, or pack showcase — use [GitHub Discussions](https://github.com/PrinceJonaa/nanosistant/discussions).

---

## License

Copyright © 2026 Prince Jona / Intervised LLC. All rights reserved.

This software is currently proprietary. Open-source release is planned. See [LICENSE](LICENSE) for details.

---

**Author:** [Prince Jona](https://intervised.com) — Intervised LLC
