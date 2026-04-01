# Changelog

All notable changes to Nanosistant are documented here.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] - 2026-04-01

### Added

#### gRPC Transport (NanoClaw ↔ RuFlo)
- Real tonic gRPC client in NanoClaw (replaces v0.1 stub)
- `NanoClawGrpcService` server implementation in RuFlo
- Async `connect()` / `send()` on `GrpcClient` with tonic channel
- Proto codegen via `tonic-build` (server + client stubs generated)
- `EdgeRuntime::process_message` now async-capable for gRPC path

#### ConversationRuntime Wiring
- `AgentRuntime` trait — abstraction over runtime implementations
- `MockAgentRuntime` — canned responses for testing and offline mode
- `AgentHandle` now holds `Option<Box<dyn AgentRuntime>>`
- `Orchestrator::execute()` — runs a routed message on the target agent's runtime
- `AgentFactory::build_with_mocks()` — creates agents with mock runtimes for testing
- Full token tracking: execute records usage in budget manager

#### Qdrant Integration (RuVector)
- `VectorBackend` trait — abstraction over storage backends
- `InMemoryBackend` — refactored v0.1 TF-IDF logic with embedding support
- `QdrantBackend` — connects to Qdrant via HTTP REST API
  - Health check, collection creation, point upsert, search, scroll
  - Graceful degradation when Qdrant is unavailable
- `VectorStore::in_memory()` and `VectorStore::qdrant()` constructors

#### Embedding-Based Semantic Search
- `EmbeddingProvider` trait with `embed()`, `embed_batch()`, `dimension()`
- `HashEmbedding` — deterministic hash-based embeddings (no model required)
- `cosine_similarity()` — vector similarity computation
- `StoredChunk` now carries `Option<Vec<f32>>` embedding
- `ingest_with_embeddings()` — optional embedding generation during ingestion
- `query_by_embedding()` on all backends — cosine similarity search

#### MCP Tool Server (RuVector)
- Full JSON-RPC 2.0 MCP server (replaces v0.1 stub)
- 4 tools: `ruvector_query`, `ruvector_ingest`, `ruvector_domains`, `ruvector_stats`
- `run_stdio()` — blocking server loop on stdin/stdout
- MCP initialize handshake with capabilities

#### ruflo Swarm Coordination
- `swarm_spawn_agent()` — spawn agent in ruflo's swarm
- `swarm_agent_status()` — check agent progress
- `swarm_coordinate()` — coordinate multi-agent tasks with topology selection
- `swarm_cancel()` — cancel running swarm tasks
- `Orchestrator::coordinate_swarm()` and `swarm_status()` convenience methods
- `SwarmAgentHandle`, `SwarmAgentStatus`, `SwarmCoordinationResult` types

### Changed
- Proto codegen switched from `prost-build` to `tonic-build`
- `AgentHandle` no longer implements `Clone` (holds runtime)
- RuVector store refactored around `VectorBackend` trait

### Stats
- 350 tests passing (up from 289)
- 28,200+ lines across 55 source files
- 9 crates

---

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
