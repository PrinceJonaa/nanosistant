# Nanosistant Architecture Overview

## System Overview

Nanosistant is a sovereign, self-learning AI assistant built on a three-tier architecture that keeps all data and inference on user infrastructure. At the edge, **NanoClaw** (Rust) intercepts every message before an LLM is ever called, resolving closed-form queries deterministically and routing the rest through a seven-tier confidence ladder. Ambiguous messages pass to **RuFlo** (the orchestrator brain), which runs domain-specialized agents backed by a four-tier memory system (L0–L3) and an offline dreaming loop that distills episodic traces into durable lesson cards. A **RuVector** service provides semantic knowledge retrieval over Qdrant. Throughout the stack, a strict Typed-IR discipline ensures that LLMs only ever *propose* — deterministic Rust code validates and *executes*.

---

## 1. Three-Tier Architecture

```mermaid
flowchart TD
    iOS["📱 NanoClawKit (iOS)\nSwift / gRPC"]
    CLI["💻 nstn-cli\n(Rust terminal client)"]

    subgraph Edge ["Edge — crate: nstn-nanoclaw"]
        NC["NanoClaw\nDeterministic intercept\n+ Confidence Ladder Router\n+ gRPC client"]
    end

    subgraph Brain ["Brain — crate: nstn-ruflo"]
        ORCH["Orchestrator\norchestrator.rs"]
        AGENTS["Domain Agents\ngeneral · music · investment\ndevelopment · framework"]
        MEM["MemorySystem\nL0 · L1 · L2 · L3"]
        DREAM["Dreamer + DreamerApplier\ndreamer.rs · dreamer_applier.rs"]
        WATCH["Watchdog\nwatchdog.rs"]
        MCPB["McpBridge → ruflo MCP\nmcp_bridge.rs (stdio JSON-RPC)"]
    end

    subgraph Knowledge ["Knowledge — crate: nstn-ruvector"]
        RVS["RuVectorService\ngrpc_server.rs"]
        EMB["Embeddings\nembeddings.rs"]
        QD["Qdrant / in-memory store\nqdrant.rs · store.rs"]
    end

    iOS -->|"gRPC (proto/nanosistant.proto)\nNanoClawService.ProcessMessage"| NC
    CLI -->|"local / gRPC"| NC
    NC -->|"gRPC\nEdgeRequest → EdgeResponse"| ORCH
    ORCH -->|"domain dispatch"| AGENTS
    ORCH -->|"read/write"| MEM
    ORCH -->|"pattern detection"| WATCH
    ORCH -->|"ambiguous routing"| MCPB
    DREAM -->|"L1 batch → DreamingReport"| MEM
    AGENTS -->|"gRPC KnowledgeQuery"| RVS
    RVS --> EMB
    RVS --> QD

    style iOS fill:#1c6ef3,color:#fff
    style CLI fill:#444,color:#fff
    style Edge fill:#0d4a2a,color:#fff
    style Brain fill:#3b1f5e,color:#fff
    style Knowledge fill:#7a2020,color:#fff
```

---

## 2. Routing Pipeline

Every incoming message is passed through a deterministic pipeline before any LLM token is consumed. Each tier has a confidence threshold; the message falls through only when no tier is confident enough.

```mermaid
flowchart TD
    MSG(["User message"])

    T0["Tier 0 — Deterministic functions\ntry_deterministic_resolution()\n30+ pure-Rust functions, O(1)\nExamples: BPM calc · scale lookup · date queries"]
    T1["Tier 1 — Aho-Corasick automaton\nO(n+z), pattern dictionary\nThreshold ≥ 0.95"]
    T2["Tier 2 — Regex bank\nCompiled at startup\nMorphological variants\nThreshold ≥ 0.80"]
    T3["Tier 3 — Weighted keywords\nDynamic, runtime-updatable\nThreshold ≥ 0.65"]
    T4["Tier 4 — Fuzzy Levenshtein\nNormalized edit distance\nTypo recovery\nThreshold ≥ 0.50"]
    T6["Tier 6 — ruflo MCP bridge\nQ-learning · MoE · semantic\nJSON-RPC over stdio"]
    T7["Tier 7 — LLM classifier\nEscape hatch\nRoutingProposal IR"]

    EXEC(["Route → Domain agent\nor deterministic response"])

    MSG --> T0
    T0 -->|"resolved"| EXEC
    T0 -->|"None"| T1
    T1 -->|"confidence ≥ 0.95"| EXEC
    T1 -->|"below threshold"| T2
    T2 -->|"confidence ≥ 0.80"| EXEC
    T2 -->|"below threshold"| T3
    T3 -->|"confidence ≥ 0.65"| EXEC
    T3 -->|"below threshold"| T4
    T4 -->|"confidence ≥ 0.50"| EXEC
    T4 -->|"Ambiguous"| T6
    T6 -->|"routing decision"| EXEC
    T6 -->|"unknown"| T7
    T7 -->|"RoutingProposal (validated)"| EXEC

    style T0 fill:#0a5c36,color:#fff
    style T6 fill:#5c2d91,color:#fff
    style T7 fill:#7a2020,color:#fff
    style EXEC fill:#1c4a8f,color:#fff
```

See [routing.md](routing.md) for a full deep-dive.

---

## 3. Memory Tier Diagram

Nanosistant's memory system has four tiers with distinct durability and write-access rules.

```mermaid
flowchart LR
    subgraph L0 ["L0 — Working Context\nWorkingContext (volatile)\nPer-task scratchpad"]
        L0C["current_goal\nactive_plan: Option&lt;ExecutionPlan&gt;\nrelevant_episodes: Vec&lt;L1Event&gt;\nrelevant_lessons: Vec&lt;LessonCard&gt;\nnotes: Vec&lt;String&gt;"]
    end

    subgraph L1 ["L1 — Episodic Trace\nEpisodicStore (append-only)\nJSONL per session"]
        L1E["L1Event\n— episode_id · timestamp · session_id\n— task_type · intent · outcome\n— tools_called · watchdog_fired\n— loop_count · token_budget_at_close"]
    end

    subgraph L2 ["L2 — Semantic Memory\nSemanticMemory (lesson cards)\nlessons.json"]
        L2C["LessonCard\n— situation · what_happened\n— instruction: LessonInstruction\n— confidence · supporting_episodes\n— deprecated · usage_count"]
    end

    subgraph L3 ["L3 — Identity & Policy\nIdentityPolicy (versioned)\nidentity.json"]
        L3P["core_values · tool_policies\nfilesystem_mode · patch_queue\nL3PatchProposal (pending Jona)"]
    end

    ORCH["Orchestrator\n(Rust)"] -->|"prepare_task()\nread-only inject"| L0
    ORCH -->|"record_event()\nappend-only"| L1
    DA["DreamerApplier\n(Rust only)"] -->|"insert LessonCard\nfrom DreamingReport"| L2
    DA -->|"queue_patch()\nJona gate required"| L3
    L1 -->|"dreaming_batch()"| DR["Dreamer (LLM)\nproduces DreamingReport"]
    DR -->|"validated report"| DA
    L2 -->|"retrieve()"| L0
    L1 -->|"recent()"| L0
    L3 -->|"values() · policy()"| ORCH

    style L0 fill:#1a3a5c,color:#fff
    style L1 fill:#1a4a2a,color:#fff
    style L2 fill:#5c3a1a,color:#fff
    style L3 fill:#5c1a1a,color:#fff
```

See [memory.md](memory.md) for full tier definitions.

---

## 4. Typed-IR Discipline

LLMs produce typed JSON proposals; Rust validates and executes them. No LLM call directly invokes a tool, writes a file, or changes routing weights.

```mermaid
sequenceDiagram
    participant User
    participant Orchestrator as Orchestrator (Rust)
    participant LLM as LLM (Anthropic/Azure/Ollama)
    participant Validator as Typed-IR Validator (Rust)
    participant Tools as Tools / State
    participant L1 as L1 EpisodicStore

    User->>Orchestrator: message
    Orchestrator->>LLM: prompt + context
    LLM-->>Orchestrator: typed JSON<br/>(RoutingProposal / ExecutionPlanIR /<br/>ToolRanking / AlignmentCheck)
    Orchestrator->>Validator: validate_routing_proposal()<br/>validate_execution_plan()<br/>validate_alignment_check()
    alt validation fails
        Validator-->>Orchestrator: Vec&lt;String&gt; errors
        Orchestrator->>LLM: re-prompt with error detail
    else validation passes
        Validator-->>Orchestrator: Ok(())
        Orchestrator->>Tools: execute typed plan
        Tools-->>Orchestrator: result
        Orchestrator->>L1: append L1Event (append-only)
        Orchestrator-->>User: response
    end
```

See [typed-ir.md](typed-ir.md) for all schema definitions and validation rules.

---

## 5. Dreaming Loop

Offline dreaming converts raw L1 traces into structured L2 lessons. Jona (the human principal) is the mandatory gate for any L3 policy changes.

```mermaid
flowchart TD
    L1["L1 EpisodicStore\nJSONL traces\n(failures · watchdog events)"]
    BATCH["dreaming_batch(max=N)\nSelect: Failure · Partial ·\nAborted · UnsafeBlocked +\nwatchdog_fired events"]
    INPUT["DreamerInput\nbatch_window · episodes\nsystem_mode · soul_md_hash"]
    LLM["Dreamer (LLM)\nMAST failure classification\nconfig/prompts/dreamer_v05.md"]
    REPORT["DreamingReport\n— lesson_cards (max 5)\n— l3_patch_proposals\n— routing_weight_hints\n— orchestrator_dispatch\n— health_signal\n— failure_classifications"]
    VALIDATE["validate_dreaming_report()\nRust guard — structural checks:\n• ≤5 lesson cards\n• ReadOnly mode enforced\n• episodes cited\n• health signal consistent"]
    APPLIER["DreamerApplier (Rust)\nDeterministic only\nNEVER calls LLM"]
    L2["L2 SemanticMemory\nLessonCard::from_dreamer()"]
    JONA["Jona (human gate)\nExternalMirror notifications\nqueue_patch() → approve/reject"]
    L3["L3 IdentityPolicy\nL3PatchProposal applied\nonly after Jona approval"]

    L1 --> BATCH --> INPUT --> LLM --> REPORT
    REPORT --> VALIDATE
    VALIDATE -->|"Ok(())"| APPLIER
    VALIDATE -->|"errors"| LLM
    APPLIER -->|"lessons_inserted"| L2
    APPLIER -->|"l3_patches_queued"| JONA
    JONA -->|"approve_patch()"| L3

    style LLM fill:#5c2d91,color:#fff
    style APPLIER fill:#0a5c36,color:#fff
    style JONA fill:#7a2020,color:#fff
    style L3 fill:#5c1a1a,color:#fff
```

---

## 6. Crate Dependency Graph

The project is a Cargo workspace with 13 crates. Arrows indicate `depends on`.

```mermaid
graph TD
    common["nstn-common\nTyped-IR · Router · Events\nDeterministic · Domain · Handoff\nProto (generated)"]
    plugins["nstn-plugins\nPlugin hooks interface"]
    runtime["nstn-runtime\nSession · Permissions · Sandbox\nMCP client · Prompt · SSE · OAuth"]
    api["nstn-api\nHTTP providers\nOpenAI-compat · NSTN provider"]
    tools["nstn-tools\nbash · file_ops"]
    commands["nstn-commands\nSlash-command handlers"]
    lsp["nstn-lsp\nLanguage Server Protocol client"]
    server["nstn-server\nAxum HTTP server (REST + SSE)"]
    cli["nstn-cli\nTerminal REPL (nanosistant bin)"]
    nanoclaw["nstn-nanoclaw\nEdge layer · gRPC client\nDeterministic intercept"]
    ruflo["nstn-ruflo\nOrchestrator · Memory (L0-L3)\nDreamer · DreamerApplier\nEvaluators · Watchdog\nGodTime · ExternalMirror\nMcpBridge · GrpcServer"]
    ruvector["nstn-ruvector\nKnowledge retrieval\nEmbeddings · Qdrant · Pipeline\nMCP server · gRPC server"]
    compat["nstn-compat-harness\nIntegration test harness"]

    %% dependency arrows
    api --> runtime
    tools --> api
    tools --> plugins
    tools --> runtime
    commands --> runtime
    commands --> plugins
    server --> runtime
    cli --> runtime
    cli --> api
    cli --> commands
    cli --> plugins
    cli --> tools
    cli --> common
    cli --> ruflo
    nanoclaw --> common
    ruflo --> common
    ruvector --> common
    compat --> runtime
    compat --> commands
    compat --> tools
    runtime --> plugins

    style common fill:#1c4a8f,color:#fff
    style ruflo fill:#5c2d91,color:#fff
    style nanoclaw fill:#0a5c36,color:#fff
    style ruvector fill:#7a2020,color:#fff
    style cli fill:#444,color:#fff
```

---

## Cross-Reference

| Document | Topic |
|---|---|
| [routing.md](routing.md) | Confidence ladder tiers, TOML config, adding a domain |
| [memory.md](memory.md) | L0–L3 schemas, dreaming, data flow |
| [typed-ir.md](typed-ir.md) | All IR types, validation functions, self-learning loop |
| [deployment.md](deployment.md) | Build, Docker, env vars, model providers, CI/CD |
| [security.md](security.md) | Sovereignty, permission tiers, sandbox, MCP isolation |
