# `nstn-ruflo`

**Path:** `crates/ruflo`  
**Package:** `nstn-ruflo` v0.5.0  
**Lines of code:** 8 748  
**Dependencies on other nstn-\* crates:** `nstn-common`

> **See also:** [Architecture Overview](../architecture/overview.md) · [Memory System](../architecture/memory.md) · [Typed IR](../architecture/typed-ir.md) · [nstn-common](./nstn-common.md)

---

## Purpose

`nstn-ruflo` is the **brain tier** of Nanosistant. It owns the complete routing and execution pipeline once a message leaves the edge. The name "RuFlo" combines *Ru* (Rust) with *Flo* (flow), describing a Rust wrapper around the ruflo TypeScript runtime's MCP tools.

Responsibilities:

- **Orchestration** — routes messages through deterministic functions, the confidence ladder, and (when needed) the ruflo MCP stack.
- **Multi-agent dispatch** — maintains a registry of domain agents and dispatches turns to their runtimes.
- **Four-tier memory** — L0 (working context), L1 (episodic trace), L2 (semantic lessons), L3 (identity/policy).
- **Dreaming loop** — offline batch analysis of L1 traces, failure classification (MAST taxonomy), and lesson card generation.
- **Watchdog** — pattern detection for agent dysfunction (stuck loops, budget blindness, handoff failures).
- **MCP bridge** — stdio JSON-RPC communication with the ruflo TypeScript runtime.
- **Session persistence** — save/restore sessions to disk.
- **gRPC service** — exposes the orchestrator as a `NanoClawService` server.

---

## Modules

```
nstn_ruflo
├── orchestrator      — Orchestrator, RouteResult, execute()
├── memory            — MemorySystem, L0–L3, all types
├── dreamer           — DreamingReport, failure taxonomy types
├── dreamer_applier   — DreamerApplier, ApplyResult
├── evaluators        — StructuralEvaluator, HumanSignalCollector, evaluate_for_learning
├── god_time          — check_god_time, GodTimeCheckResult
├── external_mirror   — ExternalMirror, MirrorNotification, NotificationType
├── agent_config      — AgentConfig, load_agent_configs
├── agent_factory     — AgentHandle, AgentRuntime, MockAgentRuntime
├── budget            — BudgetManager, BudgetState, BudgetError
├── watchdog          — Watchdog, WatchdogPattern, WatchdogAlert
├── mcp_bridge        — McpBridge, BridgeConfig
├── ruflo_proxy       — RufloProxy, swarm types
├── session_store     — SessionStore, PersistedSession, SessionMessage
├── grpc_server       — NanoClawGrpcService
└── model_router      — ModelTier, model selection logic
```

---

## Module: `orchestrator`

**File:** `crates/ruflo/src/orchestrator.rs`

The orchestrator is the entry point for all messages in the brain tier. Rust is always both the entry point and the exit point — ruflo MCP never receives traffic directly.

**Routing pipeline:**
1. Try deterministic resolution (zero tokens, pure code).
2. Budget check (refuse early if exhausted).
3. Apply domain hint if present (confidence = 1.0).
4. Run confidence ladder (AC → Regex → Weighted → Fuzzy).
5. If ambiguous → try ruflo MCP (Q-learning, MoE, semantic).
6. If ruflo unavailable → return `RouteResult::Ambiguous`.

### `RouteResult`

```rust
pub enum RouteResult {
    Deterministic {
        response: String,
        domain: String,
    },
    AgentRoute {
        domain: String,
        agent_name: String,
        enriched_message: String,   // may include budget warning
        confidence: f64,
        resolved_at_tier: u8,       // 1–5 = ladder, 6 = ruflo MCP
    },
    BudgetExhausted {
        message: String,
    },
    Ambiguous {
        message: String,
        best_guess: String,
        confidence: f64,
        scores: HashMap<String, f64>,
    },
}
```

### `Orchestrator`

```rust
pub struct Orchestrator { /* private */ }

impl Orchestrator {
    /// Create with ruflo in offline mode (confidence ladder only).
    pub fn new(agents: Vec<AgentHandle>, max_tokens: u32) -> Self;

    /// Create with a live ruflo proxy for MCP fallback routing.
    pub fn with_ruflo(agents: Vec<AgentHandle>, max_tokens: u32, ruflo: RufloProxy) -> Self;

    /// Route a message through the full pipeline.
    pub fn route(&mut self, session_id: &str, message: &str, domain_hint: &str) -> RouteResult;

    /// Execute a routed message on the appropriate agent.
    pub fn execute(&mut self, route: &RouteResult) -> Result<AgentTurnResult, String>;

    /// Record token usage from a completed turn.
    pub fn record_turn_tokens(&mut self, tokens: u32);

    /// Coordinate a multi-agent task through ruflo's swarm.
    pub fn coordinate_swarm(&mut self, task: &str, agent_types: &[String],
                            topology: &str) -> Result<SwarmCoordinationResult, String>;

    /// Check swarm status through ruflo.
    pub fn swarm_status(&mut self) -> Result<RufloSwarmStatus, String>;

    pub fn event_log(&self) -> &EventLog;
    pub fn budget(&self) -> &BudgetManager;
    pub fn budget_mut(&mut self) -> &mut BudgetManager;
    pub fn ruflo_available(&self) -> bool;
    pub fn ruflo_mut(&mut self) -> &mut RufloProxy;
    pub fn agent_count(&self) -> usize;
    pub fn has_agent(&self, domain: &str) -> bool;
}
```

Budget warning injection: when `BudgetState >= Yellow` (≥ 75 % consumed), the message is enriched with `[BUDGET WARNING: X% consumed, N tokens remaining]` before dispatch.

---

## Module: `memory`

**File:** `crates/ruflo/src/memory.rs`

Four-tier memory system. **LLMs never write to memory directly** — they produce typed proposals; deterministic Rust validates and applies changes.

### Shared Types

```rust
pub enum StepStatus { Pending, InProgress, Complete, Failed, Skipped }

pub struct PlanStep {
    pub id: String,
    pub role: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub termination_condition: String,
    pub rollback: Option<String>,
    pub status: StepStatus,
}

pub struct ExecutionPlan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
}

pub enum TaskType {
    FileOp, CodeEdit, WebSearch, MemoryQuery,
    Orchestration, ComputerControl, MultiAgent, Unknown,
}

pub enum Outcome { Success, Partial, Failure, Aborted, UnsafeBlocked }
```

### L0 — Working Context

Volatile scratchpad. Built fresh per task, never persisted.

```rust
pub struct WorkingContext {
    pub current_goal: String,
    pub active_plan: Option<ExecutionPlan>,
    pub relevant_episodes: Vec<L1Event>,
    pub relevant_lessons: Vec<LessonCard>,
    pub notes: Vec<String>,
}

impl WorkingContext {
    pub fn new(goal: &str) -> Self;
    pub fn clear(&mut self);
    pub fn add_note(&mut self, note: &str);
    pub fn inject_episodes(&mut self, episodes: Vec<L1Event>);
    pub fn inject_lessons(&mut self, lessons: Vec<LessonCard>);
}
```

### L1 — Episodic Trace

Append-only trace. Never rewritten. Each task produces one `L1Event`.

```rust
pub struct L1Event {
    pub episode_id: String,        // UUID v4
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub agent: String,
    pub task_type: TaskType,
    pub intent: String,
    pub plan_stated: Option<String>,
    pub steps_taken: Vec<String>,
    pub tools_called: Vec<ToolCallRecord>,
    pub outcome: Outcome,
    pub user_feedback: Option<String>,
    pub watchdog_fired: bool,
    pub watchdog_reason: Option<String>,
    pub memory_tiers_used: Vec<String>,
    pub error_messages: Vec<String>,
    pub loop_count: u32,
    pub token_budget_at_close: f64,
    pub notes: String,
}

impl L1Event {
    pub fn new(session_id: impl Into<String>, agent: impl Into<String>,
               task_type: TaskType, intent: impl Into<String>,
               outcome: Outcome) -> Self;
}

pub struct ToolCallRecord {
    pub tool: String,
    pub input_summary: String,
    pub output_summary: String,
    pub status: ToolCallStatus,
}

pub enum ToolCallStatus { Ok, Error, Timeout }

pub struct EpisodicStore {
    // Appends L1Events to disk; provides recent-event queries
}

impl EpisodicStore {
    pub fn new(storage_dir: impl AsRef<Path>) -> Self;
    pub fn append(&mut self, event: L1Event) -> Result<(), std::io::Error>;
    pub fn recent(&self, n: usize) -> Vec<&L1Event>;
    pub fn load_all(&mut self) -> Result<(), std::io::Error>;
}
```

### L2 — Semantic Memory

Distilled lesson cards produced by the Dreamer from L1 batches.

```rust
pub struct LessonCard {
    pub id: String,
    pub task_types: Vec<TaskType>,
    pub primary_class: String,       // MAST failure class, e.g. "FC1.1"
    pub galileo_pattern: Option<String>,
    pub confidence: String,
    pub supporting_episodes: Vec<String>,
    pub situation: String,
    pub what_happened: String,
    pub instruction: LessonInstruction,
    pub verifiable_signal: String,
    pub created_at: DateTime<Utc>,
    pub superseded_by: Option<String>,
}

pub enum LessonInstruction {
    DoThis(String),
    AvoidThis(String),
    CheckBefore(String),
}

pub struct SemanticMemory {
    // In-memory store of LessonCards; keyword search for retrieval
}

impl SemanticMemory {
    pub fn new() -> Self;
    pub fn insert(&mut self, lesson: LessonCard);
    pub fn query(&self, intent: &str, task_type: Option<&TaskType>) -> Vec<&LessonCard>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

### L3 — Identity / Policy

Typed patch proposals against the system's identity documents (SOUL.md, etc.).

```rust
pub struct L3PatchProposal {
    pub id: String,
    pub target_file: String,
    pub proposed_change: ProposedChange,
    pub risk_level: RiskLevel,
    pub rationale: String,
    pub supporting_lessons: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub status: PatchStatus,
}

pub enum ProposedChange {
    AddSection { heading: String, content: String },
    ModifySection { heading: String, new_content: String },
    RemoveSection { heading: String },
}

pub enum RiskLevel { Low, Medium, High, Critical }
pub enum PatchStatus { Pending, ApprovedByJona, Rejected, Applied }

pub struct IdentityPolicy {
    // Holds queued L3 patches; requires Jona approval before application
}

impl IdentityPolicy {
    pub fn new(storage_dir: impl AsRef<Path>) -> Self;
    pub fn queue_patch(&mut self, patch: L3PatchProposal);
    pub fn pending_patches(&self) -> Vec<&L3PatchProposal>;
    pub fn approve(&mut self, patch_id: &str) -> Result<(), String>;
    pub fn reject(&mut self, patch_id: &str) -> Result<(), String>;
    pub fn apply_approved(&mut self) -> Vec<String>;  // returns applied patch IDs
}
```

### `MemorySystem`

```rust
pub struct MemorySystem {
    pub l0: WorkingContext,
    pub l1: EpisodicStore,
    pub l2: SemanticMemory,
    pub l3: IdentityPolicy,
}

impl MemorySystem {
    pub fn new(storage_dir: impl AsRef<Path>) -> Self;
}
```

---

## Module: `dreamer`

**File:** `crates/ruflo/src/dreamer.rs`

The Dreamer is an offline batch analysis agent. It analyses L1 episodic traces, classifies failures using the MAST taxonomy, and produces a typed JSON `DreamingReport`. It **never** writes to memory, dispatches agents, or modifies config directly — `DreamerApplier` does that.

### Input

```rust
pub struct DreamerInput {
    pub batch_window: String,         // ISO 8601 time range
    pub total_episodes: usize,
    pub session_count: usize,
    pub system_mode: SystemMode,      // ReadOnly | Active
    pub soul_md_hash: String,
    pub prior_lesson_ids: Vec<String>,
    pub episodes: Vec<L1Event>,
}

pub enum SystemMode {
    ReadOnly,   // lesson cards only — no L3 proposals, no dispatch
    Active,     // full dreaming
}
```

### Failure Classification

```rust
pub struct FailureClassification {
    pub episode_id: String,
    pub primary_class: String,          // "FC1.1", "FC1.2", …
    pub secondary_class: Option<String>,
    pub galileo_pattern: Option<String>, // "G1", "G2", …
    pub evidence: String,
    pub confidence: ClassificationConfidence,
}

pub enum ClassificationConfidence { High, Medium, Low, Unclassified }
```

### Dreamer Output

```rust
pub struct DreamingReport {
    pub batch_id: String,
    pub generated_at: String,
    pub system_mode: SystemMode,
    pub health: HealthSignal,
    pub failure_classifications: Vec<FailureClassification>,
    pub lens_analysis: Vec<LensAnalysis>,
    pub insufficient_evidence: Vec<InsufficientEvidence>,
    pub lesson_cards: Vec<DreamerLessonCard>,
    pub l3_patch_proposals: Vec<L3PatchProposal>,
    pub routing_weight_hints: Vec<RoutingWeightHint>,
    pub tool_description_fixes: Vec<ToolDescriptionFix>,
    pub orchestrator_dispatch: Vec<OrchestratorTask>,
    pub partition: Partition,
}

pub enum HealthSignal { Healthy, Degraded, Critical }

pub struct LensAnalysis {
    pub lens_name: String,
    pub findings: Vec<String>,
}

pub struct InsufficientEvidence {
    pub reason: String,
    pub min_episodes_needed: usize,
}

pub struct Partition {
    pub success_count: usize,
    pub failure_count: usize,
    pub partial_count: usize,
}
```

### Lesson Card (Dreamer variant)

```rust
pub struct DreamerLessonCard {
    pub id: String,
    pub task_types: Vec<String>,
    pub primary_class: String,
    pub galileo_pattern: Option<String>,
    pub confidence: ClassificationConfidence,
    pub supporting_episodes: Vec<String>,
    pub contradicts_prior: Option<String>,
    pub supersedes_prior: Option<String>,
    pub situation: String,
    pub what_happened: String,
    pub instruction: LessonInstruction,
    pub verifiable_signal: String,
    pub orchestrator_task: Option<OrchestratorTask>,
}
```

### Orchestrator Task

```rust
pub struct OrchestratorTask {
    pub task_id: String,
    pub agent_role: String,       // "coder" | "config" | "memory_writer"
    pub task_description: String,
    pub context: TaskContext,
    pub expected_output: String,
    pub verification_step: String,
    pub priority: Priority,
}

pub struct TaskContext {
    pub lesson_card_id: Option<String>,
    pub l3_patch_id: Option<String>,
    pub target: String,
}

pub enum Priority { High, Medium, Low }
```

### Routing Weight Hint

```rust
pub struct RoutingWeightHint {
    pub task_type: String,
    pub route_stage: String,
    pub direction: String,   // "increase" | "decrease"
    pub magnitude: String,   // "small" | "medium" | "large"
    pub reason: String,
    pub supporting_episodes: Vec<String>,
}

pub struct ToolDescriptionFix {
    pub tool_name: String,
    pub current_description: String,
    pub proposed_description: String,
    pub reason: String,
    pub supporting_episodes: Vec<String>,
}

/// Validate a DreamingReport before applying it.
pub fn validate_dreaming_report(report: &DreamingReport) -> Result<(), Vec<String>>;
```

---

## Module: `dreamer_applier`

**File:** `crates/ruflo/src/dreamer_applier.rs`

Deterministic code that applies a validated `DreamingReport`. **Never calls an LLM. Never interprets prose.**

```rust
pub struct ApplyResult {
    pub lessons_inserted: usize,
    pub lessons_skipped: usize,
    pub l3_patches_queued: usize,
    pub weight_hints_recorded: usize,
    pub dispatch_tasks: usize,
    pub errors: Vec<String>,
}

pub struct DreamerApplier;

impl DreamerApplier {
    pub fn apply(report: &DreamingReport, memory: &mut MemorySystem) -> ApplyResult;
}
```

Apply steps:
1. Insert all `lesson_cards` into `memory.l2`.
2. Queue all `l3_patch_proposals` into `memory.l3` (only in `SystemMode::Active`).
3. Count `routing_weight_hints` (surfaced separately to Orchestrator via `ExternalMirror`).
4. Count `orchestrator_dispatch` tasks (handled by Orchestrator, not DreamerApplier).

---

## Module: `evaluators`

**File:** `crates/ruflo/src/evaluators.rs`

Three heterogeneous evaluators that together gate the self-learning weight-update loop.

### `StructuralEvaluator`

Deterministic, zero tokens. Checks whether expected outputs appear in actual outputs and whether tool calls succeeded.

```rust
pub struct StructuralEvaluator;

impl StructuralEvaluator {
    pub fn evaluate(
        expected_outputs: &[String],
        actual_outputs: &[String],
        tool_calls: &[ToolCallRecord],
    ) -> StructuralEval;
}
```

### `HumanSignalCollector`

Stores human confirmation/correction/abandon signals keyed by session ID.

```rust
pub struct HumanSignalCollector { /* private */ }

impl HumanSignalCollector {
    pub fn new() -> Self;
    pub fn record(&mut self, session_id: &str, signal: HumanSignal);
    pub fn get(&self, session_id: &str) -> Option<&HumanSignal>;
    pub fn clear(&mut self, session_id: &str);
}
```

### `evaluate_for_learning`

```rust
pub fn evaluate_for_learning(
    signal: &EvaluationSignal,
    episode: &L1Event,
) -> Option<Vec<WeightDelta>>
```

Combines all three signals. Returns `Some(deltas)` when `can_update_weights()` is true; returns `None` otherwise. See `nstn_common::typed_ir::can_update_weights` for the gate conditions.

---

## Module: `god_time`

**File:** `crates/ruflo/src/god_time.rs`

God-Time vs Drift-Time detection. Before any non-trivial action, checks whether the system is acting from presence (God-Time) or pattern-fill (Drift-Time). Fully deterministic — no LLM.

```rust
pub struct GodTimeCheckResult {
    pub status: GodTimeStatus,    // from typed_ir: Confirmed | DriftDetected | Unknown
    pub reason: String,
    pub action_allowed: bool,
}

pub fn check_god_time(
    context: &WorkingContext,
    recent_events: &[L1Event],
) -> GodTimeCheckResult
```

**Drift signals** (any one triggers a block):
1. `context.current_goal` is empty — no clear goal.
2. Last 3+ `recent_events` share the same `intent` — stuck loop.
3. Last 3+ `recent_events` are all failure-class outcomes with no change in `plan_stated` — failing without adapting.
4. Any `context.notes` contain `"confused"` or `"unsure"` — self-reported confusion.

---

## Module: `external_mirror`

**File:** `crates/ruflo/src/external_mirror.rs`

The human gate for all L3 changes, watchdog escalations, and dreaming reports. Maintains a persistent notification queue serialised to disk as JSON.

### `NotificationType`

```rust
pub enum NotificationType {
    WatchdogTrigger,
    DreamingReport,
    L3PatchProposal,
    SafetyIncident,
    SystemHealth,
}
```

### `MirrorNotification`

```rust
pub struct MirrorNotification {
    pub id: String,                           // UUID v4
    pub timestamp: DateTime<Utc>,
    pub notification_type: NotificationType,
    pub summary: String,
    pub details: serde_json::Value,
    pub requires_action: bool,
    pub acknowledged: bool,
}

impl MirrorNotification {
    pub fn new(notification_type: NotificationType, summary: impl Into<String>,
               details: serde_json::Value, requires_action: bool) -> Self;
}
```

### `ExternalMirror`

```rust
pub struct ExternalMirror { /* private */ }

impl ExternalMirror {
    pub fn new(storage_path: impl AsRef<Path>) -> Self;
    pub fn push(&mut self, notification: MirrorNotification);
    pub fn pending(&self) -> Vec<&MirrorNotification>;
    pub fn acknowledge(&mut self, id: &str) -> Result<(), String>;
    pub fn save(&self) -> Result<(), std::io::Error>;
    pub fn load(&mut self) -> Result<(), std::io::Error>;
}
```

---

## Module: `agent_config`

**File:** `crates/ruflo/src/agent_config.rs`

Loads per-agent TOML configuration files.

```rust
pub struct AgentConfig {
    pub name: String,
    pub description: String,
    pub model: String,                 // e.g. "claude-sonnet-4-20250514"
    pub permission_mode: String,       // "read_only" | "workspace_write" | "danger_full_access"
    pub triggers: TriggerConfig,       // from nstn-common
    pub prompt: PromptConfig,
    pub knowledge: Option<KnowledgeConfig>,
    pub tools: ToolsConfig,
}

pub struct PromptConfig {
    pub identity_file: String,   // e.g. "config/prompts/identity.md"
    pub domain_file: String,     // e.g. "config/prompts/music.md"
}

pub struct KnowledgeConfig {
    pub domain_filter: String,
    pub auto_retrieve: bool,
}

pub struct ToolsConfig {
    pub include: Vec<String>,         // Claude-facing tool names
    pub deterministic: Vec<String>,   // deterministic function names exposed as tools
}

/// Load all *.toml agent config files from a directory.
pub fn load_agent_configs(config_dir: &Path) -> Result<Vec<AgentConfig>, AgentConfigError>;

pub enum AgentConfigError {
    Io { path: String, source: std::io::Error },
    Toml { path: String, source: toml::de::Error },
    DirNotFound(String),
}
```

**TOML format:** Each file wraps `AgentConfig` under an `[agent]` table:

```toml
[agent]
name = "music"
description = "Hip-hop and music production specialist"
model = "claude-sonnet-4-20250514"
permission_mode = "read_only"

[agent.triggers]
keywords = ["verse", "hook", "beat", "bpm", "808"]
priority = 10

[agent.prompt]
identity_file = "config/prompts/identity.md"
domain_file = "config/prompts/music.md"

[agent.knowledge]
domain_filter = "music"
auto_retrieve = true

[agent.tools]
include = ["bash", "file_ops"]
deterministic = ["scale_degrees", "bpm_to_bar_duration"]
```

---

## Module: `agent_factory`

**File:** `crates/ruflo/src/agent_factory.rs`

```rust
pub trait AgentRuntime: Send {
    fn run_turn(&mut self, message: &str) -> Result<AgentTurnResult, String>;
    fn session_tokens(&self) -> usize;
}

pub struct AgentTurnResult {
    pub response_text: String,
    pub tool_calls: Vec<String>,
    pub tokens_used: u32,
    pub iterations: usize,
}

pub struct MockAgentRuntime { /* private */ }

impl MockAgentRuntime {
    pub fn new(domain: impl Into<String>) -> Self;
}

impl AgentRuntime for MockAgentRuntime { /* … */ }

pub struct AgentHandle {
    pub config: AgentConfig,
    runtime: Option<Box<dyn AgentRuntime>>,
}

impl AgentHandle {
    pub fn from_config(config: AgentConfig) -> Self;
    pub fn with_runtime(self, runtime: Box<dyn AgentRuntime>) -> Self;
    pub fn with_mock_runtime(self) -> Self;
    pub fn runtime_mut(&mut self) -> Option<&mut Box<dyn AgentRuntime>>;
}

pub struct AgentFactory;

impl AgentFactory {
    pub fn build_from_configs(configs: Vec<AgentConfig>) -> Vec<AgentHandle>;
}
```

---

## Module: `budget`

**File:** `crates/ruflo/src/budget.rs`

```rust
pub enum BudgetState {
    Green,      // < 50%
    Amber,      // ≥ 50%
    Yellow,     // ≥ 75%
    Red,        // ≥ 90%
    Exhausted,  // 100%
}

impl BudgetState {
    pub fn as_str(self) -> &'static str;
}

pub enum BudgetError {
    Exhausted { tokens_used: u32, max_tokens: u32 },
}

pub struct BudgetManager { /* private */ }

impl BudgetManager {
    pub fn new(max_tokens: u32) -> Self;
    pub fn record_usage(&mut self, tokens: u32);
    pub fn check(&self) -> Result<(), BudgetError>;
    pub fn state(&self) -> BudgetState;
    pub fn remaining(&self) -> u32;
    pub fn utilization_pct(&self) -> f64;   // 0.0–1.0
    pub fn to_proto(&self) -> proto::BudgetStatus;
}
```

---

## Module: `watchdog`

**File:** `crates/ruflo/src/watchdog.rs`

Pattern detection on event streams. All detection is deterministic — no LLM.

```rust
pub enum WatchdogPattern {
    StuckLoop,       // same problem 3+ times across turns
    TokenWaste,      // > 40% of session tokens on non-productive ops
    HandoffFailure,  // 2+ handoff rejections in a session
    BudgetBlindness, // running past 75% budget without awareness event
    SpecRepetition,  // same output structure 3+ times
}

pub enum AlertSeverity { Warning, Critical }

pub struct WatchdogAlert {
    pub pattern: WatchdogPattern,
    pub description: String,
    pub severity: AlertSeverity,
}

pub struct Watchdog { /* private */ }

impl Watchdog {
    pub fn new() -> Self;
    pub fn check(&self, events: &[Event], budget: &BudgetManager) -> Vec<WatchdogAlert>;
}
```

---

## Module: `mcp_bridge`

**File:** `crates/ruflo/src/mcp_bridge.rs`

stdio JSON-RPC 2.0 bridge to the ruflo TypeScript MCP server. Spawns ruflo as a child process and communicates via its stdin/stdout.

```rust
pub struct BridgeConfig {
    pub command: String,           // e.g. "node"
    pub args: Vec<String>,         // e.g. ["dist/mcp-server.js"]
    pub timeout_ms: u64,           // per-request timeout
}

pub struct McpBridge { /* private */ }

impl McpBridge {
    pub fn new(config: BridgeConfig) -> Self;
    pub fn start(&mut self) -> Result<(), BridgeError>;
    pub fn stop(&mut self);
    pub fn is_running(&self) -> bool;
    pub fn call_tool(&mut self, tool_name: &str,
                     params: serde_json::Value) -> Result<serde_json::Value, BridgeError>;
    pub fn list_tools(&mut self) -> Result<Vec<McpTool>, BridgeError>;
}

pub enum BridgeError {
    NotRunning,
    Io(std::io::Error),
    Serialization(String),
    Timeout,
    RpcError(JsonRpcError),
}
```

---

## Module: `ruflo_proxy`

**File:** `crates/ruflo/src/ruflo_proxy.rs`

Typed Rust interface over ruflo's MCP routing and orchestration tools. The orchestrator calls these methods when the confidence ladder returns `Ambiguous`.

```rust
pub enum RufloRouterType {
    QLearning,        // Q-learning reinforcement learning
    MixtureOfExperts, // MoE gating network
    Semantic,         // embedding similarity
    ModelRouter,      // model complexity routing
    IntentRouter,     // intent router plugin
    Fallback,
}

pub struct RufloRouteResult {
    pub route: String,                         // selected domain
    pub confidence: f64,
    pub router_type: RufloRouterType,
    pub model_recommendation: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct RufloModelSelection {
    pub model: String,
    pub complexity_score: f64,
    pub cost_multiplier: f64,
    pub reason: String,
}

pub struct RufloSwarmStatus {
    pub active_agents: u32,
    pub topology: String,
    // … additional status fields
}

pub struct SwarmAgentHandle {
    pub agent_id: String,
    pub agent_type: String,
    pub status: SwarmAgentStatus,
}

pub enum SwarmAgentStatus { Idle, Working, Complete, Failed }

pub struct SwarmCoordinationResult {
    pub task_id: String,
    pub assigned_agents: Vec<SwarmAgentHandle>,
    pub estimated_completion_ms: u64,
}

pub struct RufloProxy { /* private */ }

impl RufloProxy {
    pub fn new(bridge: McpBridge) -> Self;
    pub fn offline() -> Self;     // no-op proxy for offline mode
    pub fn is_available(&self) -> bool;
    pub fn route_message(&mut self, message: &str,
                         context: &HashMap<String, String>) -> Result<RufloRouteResult, ProxyError>;
    pub fn select_model(&mut self, message: &str,
                        domain: &str) -> Result<RufloModelSelection, ProxyError>;
    pub fn swarm_coordinate(&mut self, task: &str, agent_types: &[String],
                            topology: &str) -> Result<SwarmCoordinationResult, ProxyError>;
    pub fn swarm_status(&mut self) -> Result<RufloSwarmStatus, ProxyError>;
}

pub enum ProxyError {
    Bridge(BridgeError),
    UnexpectedResult(String),
    Unavailable,
}
```

---

## Module: `session_store`

**File:** `crates/ruflo/src/session_store.rs`

```rust
pub struct SessionMessage {
    pub role: String,         // "user" | "assistant" | "tool"
    pub content: String,
    pub domain: String,
    pub timestamp: DateTime<Utc>,
    pub tokens: u32,
}

pub struct PersistedSession {
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub current_domain: String,
    pub turn_count: u32,
    pub tokens_used: u32,
    pub messages: Vec<SessionMessage>,
}

impl PersistedSession {
    pub fn new(session_id: impl Into<String>, domain: impl Into<String>) -> Self;
}

pub struct SessionStore { /* private */ }

impl SessionStore {
    /// Create backed by `storage_dir` (created if not present).
    pub fn new(storage_dir: impl AsRef<Path>) -> Self;
    pub fn save(&mut self, session: PersistedSession) -> Result<(), std::io::Error>;
    pub fn load(&self, session_id: &str) -> Option<PersistedSession>;
    pub fn load_all(&mut self) -> Result<(), std::io::Error>;
    pub fn list(&self) -> Vec<&PersistedSession>;
    pub fn delete(&mut self, session_id: &str) -> Result<(), std::io::Error>;
}
```

Sessions are serialised as `{storage_dir}/{session_id}.json`.

---

## Module: `grpc_server`

**File:** `crates/ruflo/src/grpc_server.rs`

Implements the `NanoClawService` trait generated by `tonic` from `nanosistant.proto`.

```rust
pub struct NanoClawGrpcService {
    orchestrator: Arc<Mutex<Orchestrator>>,
}

impl NanoClawGrpcService {
    pub fn new(orchestrator: Arc<Mutex<Orchestrator>>) -> Self;
}

#[tonic::async_trait]
impl NanoClawService for NanoClawGrpcService {
    async fn process_message(
        &self,
        request: Request<EdgeRequest>,
    ) -> Result<Response<EdgeResponse>, Status>;

    type StreamMessageStream = ReceiverStream<Result<EdgeResponse, Status>>;
    async fn stream_message(
        &self,
        request: Request<EdgeRequest>,
    ) -> Result<Response<Self::StreamMessageStream>, Status>;
}
```

Processing order per request:
1. Lock the orchestrator.
2. Call `orchestrator.route()` to get a `RouteResult`.
3. Call `orchestrator.execute()` to produce an `AgentTurnResult`.
4. Build and return an `EdgeResponse` proto.

---

## Module: `model_router`

**File:** `crates/ruflo/src/model_router.rs`

Deterministic model-tier selection. Ported from the ruflo upstream TypeScript model-routing concept.

```rust
pub enum ModelTier {
    Fast,      // claude-haiku-4-20250514
    Balanced,  // claude-sonnet-4-20250514
    Powerful,  // claude-opus-4-20250514
}

impl ModelTier {
    pub fn as_str(self) -> &'static str;
    pub fn from_model_id(model_id: &str) -> Self;
}

pub struct ModelRouterInput {
    pub router_confidence: f64,
    pub complexity: f64,         // 0.0–1.0 estimated query complexity
    pub domain: String,
    pub agent_model_floor: String, // from AgentConfig.model
}

pub struct ModelRouterOutput {
    pub tier: ModelTier,
    pub model: String,           // full model ID string
    pub reason: String,
}

pub fn select_model(input: &ModelRouterInput) -> ModelRouterOutput;
```

**Selection logic:**

| Condition | Tier |
|---|---|
| `confidence ≥ 0.95` and `complexity < 0.30` | `Fast` (haiku) |
| `confidence ≥ 0.70` and `complexity < 0.65` | `Balanced` (sonnet) |
| `complexity ≥ 0.65` or `domain == "framework"` | `Powerful` (opus) |
| (default) | `Balanced` (sonnet) |

The `agent_model_floor` from `AgentConfig.model` is treated as a minimum — the router never downgrades below the configured model.

---

## Usage Example

```rust
use nstn_ruflo::{
    AgentConfig, AgentHandle, Orchestrator, RouteResult,
    MemorySystem, WorkingContext, check_god_time,
};
use nstn_common::TriggerConfig;

// Build agent handles
let music_handle = AgentHandle::from_config(AgentConfig {
    name: "music".into(),
    description: "Music production specialist".into(),
    model: "claude-sonnet-4-20250514".into(),
    permission_mode: "read_only".into(),
    triggers: TriggerConfig {
        keywords: vec!["verse".into(), "beat".into(), "bpm".into()],
        priority: 10,
    },
    prompt: nstn_ruflo::agent_config::PromptConfig {
        identity_file: "config/prompts/identity.md".into(),
        domain_file: "config/prompts/music.md".into(),
    },
    knowledge: None,
    tools: Default::default(),
}).with_mock_runtime();

// Build orchestrator
let mut orch = Orchestrator::new(vec![music_handle], 50_000);

// Route and execute
let route = orch.route("session-1", "help me write a verse", "");
let result = orch.execute(&route).unwrap();
println!("{}", result.response_text);

// Memory
let mut memory = MemorySystem::new("/data/memory");
let ctx = WorkingContext::new("write a verse");
let god_time = check_god_time(&ctx, &[]);
assert!(god_time.action_allowed);
```
