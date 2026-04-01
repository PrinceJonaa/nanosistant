# Changelog

All notable changes to Nanosistant are documented here.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-01

### Added

#### Architecture
- Three-tier Rust architecture: NanoClaw (edge) → RuFlo (brain) → RuVector (knowledge)
- Protobuf contracts at all tier boundaries (`proto/nanosistant.proto`)
- gRPC service definitions for `NanoClawService` and `RuVectorService`

#### Routing
- Confidence-ladder router with 4 deterministic tiers:
  - Tier 1: Aho-Corasick automaton (O(n+z) multi-pattern matching)
  - Tier 2: Compiled regex bank (morphological variants)
  - Tier 3: Dynamic weighted keyword map (runtime-updatable)
  - Tier 4: Levenshtein edit-distance (typo recovery)
- ruflo MCP integration — Q-learning, MoE, and semantic routing as fallback
- `RouteResult::Ambiguous` with per-domain scores for LLM escape hatch
- Domain classification from agent TOML trigger configs

#### ruflo Integration
- Git submodule at `vendor/ruflo` (github.com/ruvnet/ruflo v3.5)
- MCP stdio bridge — JSON-RPC 2.0 over stdin/stdout
- Typed `RufloProxy` with methods for routing, model selection, memory, swarm
- Graceful offline fallback when ruflo/Node.js unavailable

#### Deterministic Functions (30+)
- Music: BPM, bar count, scale degrees, chord-to-roman, transpose, frequency bands, syllable density
- Finance: percentage change, CAGR, position sizing
- Business: release timeline, ISRC validation, streaming loudness check
- Universal: datetime, word count, reading time, JSON/URL validation

#### Agent System
- Config-driven domain agents (TOML + prompt markdown)
- 5 built-in agents: general, music, investment, development, framework
- Agent factory creates ConversationRuntimes from config
- New domains added without code changes

#### Runtime (forked from claw-code)
- `ConversationRuntime` — core agent loop with hooks, permissions, compaction
- LLM API client (Anthropic + OpenAI-compatible, SSE streaming)
- 19 tool definitions (bash, file ops, web, search)
- Plugin system with pre/post hook pipeline

#### Knowledge Tier
- In-memory vector store with TF-IDF keyword scoring
- Document ingestion (splits on `##` markdown headers)
- Domain-filtered search
- MCP server and gRPC service stubs

#### Edge Tier
- Local-first deterministic resolution
- gRPC client interface (stub for v0.1.0)
- Offline queue with sync-on-connect

#### Observability
- Structured event system (routing, deterministic, agent turns, handoffs, budget)
- Token budget manager with circuit breakers (green/amber/yellow/red/exhausted)
- Watchdog with 5 pattern detectors: StuckLoop, TokenWaste, HandoffFailure, BudgetBlindness, SpecRepetition
- Typed handoff validation (prevents MAST-identified specification-gap failures)

#### Infrastructure
- GitHub Actions CI (fmt, clippy, test)
- 289 passing tests across 9 crates

### Foundation
- Built on [claw-code](https://github.com/anthropics/claw-code) (March 2026 SOTA open-source agent runtime)
- Informed by MAST taxonomy (arXiv 2503.13657) multi-agent failure modes research
