# Changelog

All notable changes to Nanosistant are documented here.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.4.0] - 2026-04-02

### Added

#### Full MCP Client (6 transports)
- `mcp.rs` — tool namespacing (`mcp__server__tool`), server signatures, config hashing
- `mcp_client.rs` — bootstrap types for Stdio, SSE, HTTP, WebSocket, SDK, ManagedProxy
- `mcp_stdio.rs` — async stdio transport (~1,700 lines): JSON-RPC 2.0, tool discovery with pagination, resource protocol, connection lifecycle
- `mcp_config.rs` — McpServerConfig, 6 transport config types, McpConfig

#### Sandbox / Filesystem Isolation
- `FilesystemIsolationMode` — Off, WorkspaceOnly, AllowList
- Container detection (Docker, Podman, K8s) via /.dockerenv, env vars, /proc/1/cgroup
- Linux `unshare` namespace isolation for bash execution
- Network isolation toggle
- `SandboxConfig` → `SandboxStatus` resolution with fallback reasons

#### Slash Commands System (28+ commands)
- `/help`, `/status`, `/compact`, `/model`, `/permissions`, `/clear`, `/cost`
- `/resume`, `/config`, `/memory`, `/init`, `/diff`, `/version`
- `/bughunter`, `/branch`, `/worktree`, `/commit`, `/commit-push-pr`, `/pr`, `/issue`
- `/ultraplan`, `/teleport`, `/agents`, `/skills`, `/plugins`
- `SlashCommand::parse()`, `CommandRegistry`, `render_slash_command_help()`

#### CLI / REPL (`nanosistant` binary)
- Interactive REPL with model streaming and markdown rendering
- Vim keybindings (Normal/Insert/Visual/Command modes) in input editor
- `syntect` syntax highlighting in code blocks
- `pulldown-cmark` markdown rendering to terminal
- `Spinner` with animated braille frames
- Non-interactive prompt mode for piped input
- `--model`, `--permission-mode`, `--output-format`, `--help`, `--version`
- Session resume from file

#### Full OAuth System
- PKCE S256 code challenge generation
- `OAuthAuthorizationRequest` — builds authorize URL
- Token exchange, refresh, callback parsing
- File-based credential persistence (save/load/clear)

#### Remote Session / Upstream Proxy
- `RemoteSessionContext` — enabled flag, session ID, base URL from env
- `UpstreamProxyBootstrap` — token reading, CA bundle resolution
- `UpstreamProxyState` — proxy URL, NO_PROXY hosts

#### SSE Runtime Streaming
- `SseEvent` struct with event/data/id fields
- `IncrementalSseParser` for streaming SSE chunks
- Wire-format serialization

#### Bootstrap System
- `BootstrapPhase` enum (12 phases from ConfigLoad to Ready)
- `BootstrapPlan` with ordered phase execution

#### Compat Harness
- `UpstreamPaths` — resolve upstream TypeScript source paths
- `ExtractedManifest` — commands + tools + bootstrap plan extraction
- Multi-candidate repo root resolution

#### LSP Integration
- `LspClient` — JSON-RPC over stdio to language servers
- `LspManager` — multi-server management, extension-based routing
- Full document lifecycle (open, change, hover, definition, diagnostics)
- Type definitions: positions, ranges, diagnostics, completions

### Stats
- 470 tests passing (up from 365)
- 43,000+ Rust lines + 2,100 Swift lines across 77 source files
- 13 Rust crates + 1 Swift package
- `nanosistant` CLI binary builds successfully

---

## [0.3.0] - 2026-04-01

### Added

#### iOS NanoClaw Client (Swift)
- `clients/ios/NanoClawKit/` — Swift Package (iOS 17+, macOS 14+)
- `NanoClawClient` — main entry point: deterministic → brain → offline-queue pipeline
- `DeterministicResolver` — full port of Rust deterministic functions to Swift
- `MusicTheory` — BPM, scales, chords, transpose, frequency bands, syllables (identical output to Rust)
- `BrainClient` — async HTTP client to RuFlo server
- `OfflineQueue` — bounded FIFO with JSON disk persistence, oldest-evicts overflow
- `SessionManager` — session state with message history, disk save/load
- `Models` — EdgeRequest, EdgeResponse, BudgetStatus, RouteSource (Codable, Sendable)
- 30+ Swift test assertions across 3 test files

#### Knowledge Ingestion Pipeline
- `IngestionPipeline` — processes multiple file/directory sources into VectorStore
- `IngestionSource` config — path, domain, doc_type, extensions, recursive flag
- `IngestionPipeline::from_config()` — parses TOML `[[source]]` tables
- `config/ingestion.toml` — default configuration with examples
- 8 tests covering single file, directory, recursion, extension filter, stats

#### Session Persistence
- `SessionStore` — save/restore sessions to JSON files on disk
- `PersistedSession` — session_id, timestamps, domain, turn count, message history
- Per-session JSON file storage with load_all/save/delete/recent
- 7 tests covering full lifecycle

#### Production Deployment
- `Dockerfile` — multi-stage build (rust:1.75 builder → debian:bookworm-slim)
- `docker-compose.yml` — nanosistant + qdrant services with named volumes
- `nstn-server` binary — axum server with health endpoint
- `GET /health` — returns `{status, version, service}`

### Stats
- 365 Rust tests + Swift tests passing
- 29,100+ Rust lines + 2,100 Swift lines across 69 source files
- 9 Rust crates + 1 Swift package

---

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
