# Crate Reference

Nanosistant is organised as a Cargo workspace with 13 crates. All crates share `version = "0.5.0"` and `edition = "2021"`.

> **See also:** [Architecture Overview](../architecture/overview.md) · [Routing Pipeline](../architecture/routing.md) · [Memory System](../architecture/memory.md)

---

## All Crates

| Package | Path | Description | Key Public Types | LoC | Depends on |
|---|---|---|---|---|---|
| `nstn-common` | `crates/common` | Shared primitives: deterministic functions, confidence-ladder router, domain classifier, typed IR schemas, event system, handoff validation, and generated protobuf types. | `ConfidenceLadderRouter`, `RouterBuilder`, `DomainClassifier`, `TriggerConfig`, `Event`, `EventLog`, `EventType`, `HandoffValidator`, `HandoffError`, `RoutingProposal`, `ExecutionPlanIR`, `EvaluationSignal`, `WeightDelta`, `AlignmentCheck`, proto `EdgeRequest` / `EdgeResponse` / `AgentHandoff` | 3 471 | — |
| `nstn-ruflo` | `crates/ruflo` | Brain-tier orchestrator: routing pipeline, multi-agent dispatch, four-tier memory system, dreaming loop, watchdog, MCP bridge to the ruflo TypeScript runtime, gRPC service, and session store. | `Orchestrator`, `RouteResult`, `MemorySystem`, `WorkingContext`, `EpisodicStore`, `SemanticMemory`, `L1Event`, `LessonCard`, `L3PatchProposal`, `DreamingReport`, `DreamerApplier`, `ApplyResult`, `Watchdog`, `WatchdogPattern`, `WatchdogAlert`, `McpBridge`, `BridgeConfig`, `RufloProxy`, `BudgetManager`, `BudgetState`, `SessionStore`, `PersistedSession`, `NanoClawGrpcService`, `AgentConfig`, `AgentHandle`, `AgentRuntime`, `MockAgentRuntime`, `ExternalMirror`, `MirrorNotification`, `GodTimeCheckResult` | 8 748 | `nstn-common` |
| `nstn-ruvector` | `crates/ruvector` | Knowledge-tier vector store: pluggable backends (in-memory, Qdrant), embedding trait, document ingestion pipeline, MCP server, and gRPC service. | `VectorStore`, `VectorBackend`, `InMemoryBackend`, `QdrantBackend`, `KnowledgeChunk`, `StoredChunk`, `EmbeddingProvider`, `HashEmbedding`, `DocumentIngester`, `IngestionPipeline`, `IngestionSource`, `IngestionResult`, `McpServer`, `RuVectorGrpcService`, `StoreError`, `EmbeddingError` | 2 573 | `nstn-common` |
| `nstn-nanoclaw` | `crates/nanoclaw` | Edge-tier runtime: deterministic intercept, gRPC client to RuFlo, offline queue. | `EdgeRuntime`, `EdgeRequest`, `EdgeResponse`, `EdgeError`, `SessionContext`, `GrpcClient`, `GrpcError`, `LocalExecutor`, `OfflineQueue`, `QueuedMessage` | 537 | `nstn-common` |
| `nstn-runtime` | `crates/runtime` | Core conversation runtime: config loading, session management, MCP client/server, tool dispatch, sandbox, OAuth, SSE, compact, prompt, remote API calls, usage tracking. | `ConversationRuntime`, `Config`, `Session`, `McpClient`, `McpConfig`, `Prompt`, `Sandbox`, `OAuthClient`, `UsageTracker` | 8 667 | `nstn-plugins` |
| `nstn-api` | `crates/api` | HTTP API client: OpenAI-compatible and native Nanosistant providers, SSE streaming, error types. | `ApiClient`, `ApiProvider`, `OpenAiCompatProvider`, `NstnProvider`, `ApiError`, `SseStream`, `ChatRequest`, `ChatResponse` | 3 216 | `nstn-runtime` |
| `nstn-commands` | `crates/commands` | Built-in slash commands (`/compact`, `/help`, etc.) and their dispatch logic. | `CommandDispatcher`, `Command`, `CommandResult` | 1 868 | `nstn-runtime`, `nstn-plugins` |
| `nstn-tools` | `crates/tools` | Tool implementations: bash execution, file operations, web search, and tool registry. | `BashTool`, `FileOpsTool`, `ToolRegistry`, `ToolResult` | 5 207 | `nstn-api`, `nstn-plugins`, `nstn-runtime` |
| `nstn-plugins` | `crates/plugins` | Plugin trait and hook infrastructure for extending the assistant with external integrations. | `Plugin`, `PluginHook`, `HookContext`, `HookResult` | 3 340 | — |
| `nstn-lsp` | `crates/lsp` | Language Server Protocol client: communicates with language servers for code intelligence features. | `LspClient`, `LspManager`, `LspError`, `LspTypes` | 1 185 | — |
| `nstn-server` | `crates/server` | HTTP server (`axum`) that exposes the runtime as an OpenAI-compatible REST API with SSE streaming. | `AppState`, `ChatHandler`, `SseEncoder` | 486 | `nstn-runtime` |
| `nstn-compat-harness` | `crates/compat-harness` | Compatibility test harness for validating integrations across runtime, commands, and tools. | `CompatHarness`, `HarnessRunner` | 356 | `nstn-runtime`, `nstn-commands`, `nstn-tools` |
| `nstn-cli` | `crates/nstn-cli` | Terminal client binary (`nanosistant`): REPL loop, markdown rendering, syntax highlighting, input handling, init wizard. | `main`, `Renderer`, `InputHandler`, `InitWizard` | 7 023 | `nstn-runtime`, `nstn-api`, `nstn-commands`, `nstn-plugins`, `nstn-tools`, `nstn-common`, `nstn-ruflo` |

---

## Dependency Graph

```
nstn-cli ──────────────────────────────────────────────────────────┐
           ├── nstn-runtime ──── nstn-plugins                      │
           ├── nstn-api    ──── nstn-runtime                       │
           ├── nstn-commands ── nstn-runtime, nstn-plugins         │
           ├── nstn-tools  ──── nstn-api, nstn-plugins, nstn-runtime │
           ├── nstn-common                                          │
           └── nstn-ruflo  ──── nstn-common                        │
                                                                    │
nstn-nanoclaw ──────────── nstn-common                             │
nstn-ruvector ──────────── nstn-common                             │
                                                                    │
nstn-server ────────────── nstn-runtime                            │
nstn-lsp ────────────────── (no nstn-* deps)                       │
nstn-plugins ────────────── (no nstn-* deps)                       │
nstn-compat-harness ─────── nstn-runtime, nstn-commands, nstn-tools│
```

---

## Crates with Detailed API Docs

Full module-level documentation is available for the four primary crates:

- [nstn-common](./nstn-common.md) — shared primitives and routing
- [nstn-ruflo](./nstn-ruflo.md) — brain-tier orchestrator
- [nstn-ruvector](./nstn-ruvector.md) — knowledge-tier vector store
- [nstn-nanoclaw](./nstn-nanoclaw.md) — edge-tier runtime
