# `nstn-common`

**Path:** `crates/common`  
**Package:** `nstn-common` v0.5.0  
**Lines of code:** 3 471  
**Dependencies on other nstn-\* crates:** none (leaf crate)

> **See also:** [Routing Pipeline](../architecture/routing.md) · [Typed IR](../architecture/typed-ir.md) · [nstn-ruflo](./nstn-ruflo.md) · [nstn-nanoclaw](./nstn-nanoclaw.md)

---

## Purpose

`nstn-common` is the shared foundation for the entire Nanosistant workspace. It provides:

- **30+ deterministic (zero-token) functions** covering music theory, finance, scheduling, and utilities — the base of the routing pipeline's Tier 0.
- **`ConfidenceLadderRouter`** — the five-tier deterministic router (Aho-Corasick → Regex → Weighted keywords → Fuzzy edit-distance → LLM escape hatch).
- **Domain classification** types (`DomainClassifier`, `TriggerConfig`) used to register agents.
- **Event system** (`Event`, `EventLog`, `EventType`) for structured audit logging.
- **Handoff validation** (`HandoffValidator`, `HandoffError`) — typed gating on all cross-agent transfers.
- **Typed IR schemas** — the boundary layer between LLM proposals and Rust execution.
- **Generated protobuf types** from `nanosistant.proto` (via `prost`/`tonic`).

All public types are re-exported from `nstn_common::*` for convenience.

---

## Modules

```
nstn_common
├── deterministic   — zero-token pure-Rust functions (re-exported at crate root)
├── domain          — DomainClassifier, TriggerConfig, Domain
├── router          — ConfidenceLadderRouter, RouterBuilder, RouteDecision, …
├── events          — Event, EventLog, EventType
├── handoff         — HandoffValidator, HandoffError
├── typed_ir        — RoutingProposal, ExecutionPlanIR, EvaluationSignal, WeightDelta, …
└── proto           — generated protobuf types (EdgeRequest, EdgeResponse, AgentHandoff, …)
```

---

## Module: `deterministic`

**File:** `crates/common/src/deterministic.rs`  
**Doc:** *"Zero-token deterministic functions. Every function here runs as pure code — no LLM inference. Available to all agents via the orchestrator's interception layer AND as registered tools within agent `ConversationRuntimes`."*

All functions are `#[must_use]` unless they return `Result`. They are re-exported at the crate root via `pub use deterministic::*`.

### Entry Point

```rust
pub fn try_deterministic_resolution(message: &str) -> Option<String>
```

Attempts to resolve a user message without an LLM. Returns `Some(response)` on a match, `None` when the message requires judgment. Called by the orchestrator as Tier 0 before the confidence ladder.

---

### Universal Functions

| Function | Signature | Description |
|---|---|---|
| `current_datetime` | `() -> String` | Returns current UTC time in ISO 8601 / RFC 3339 format. |
| `days_until` | `(target_date: &str) -> Result<i64, String>` | Days remaining until a `YYYY-MM-DD` date. Negative when past. |
| `word_count` | `(text: &str) -> usize` | Word count via `split_whitespace`. |
| `reading_time_minutes` | `(text: &str) -> f64` | Estimated reading time at 250 words/min, rounded to 1 decimal. |
| `json_validate` | `(text: &str) -> bool` | Returns `true` when text is valid JSON. |
| `url_validate` | `(text: &str) -> bool` | Returns `true` for `http://` or `https://` strings. |

---

### Music Domain Functions

| Function | Signature | Description |
|---|---|---|
| `bpm_to_bar_duration` | `(bpm: u32, beats_per_bar: u32) -> f64` | Duration of one bar in seconds at the given BPM and time signature. |
| `song_bar_count` | `(bpm: u32, target_duration_secs: f64) -> u32` | Number of 4/4 bars needed to fill a target duration. |
| `scale_degrees` | `(key: &str, mode: &str) -> Vec<String>` | Note names for the given key and mode. Supports `major`, `minor`, `dorian`, `phrygian`, `lydian`, `mixolydian`, `locrian`. |
| `chord_to_roman` | `(chord: &str, key: &str) -> String` | Converts a chord name to Roman numeral notation in a given key (e.g. `"Am"` in `"C"` → `"vi"`). |
| `roman_to_chord` | `(roman: &str, key: &str) -> String` | Converts a Roman numeral to a chord name in a given key. |
| `transpose` | `(notes: &[String], semitones: i32) -> Vec<String>` | Transposes a list of note names by the given number of semitones. Handles enharmonic equivalents. |
| `note_to_frequency` | `(note: &str, octave: u32) -> f64` | Concert pitch frequency in Hz (A4 = 440 Hz). |
| `frequency_to_band` | `(hz: f64) -> &'static str` | Maps a frequency to its EQ band name: `"Sub Bass"`, `"Bass"`, `"Low Mids"`, `"Mids"`, `"Upper Mids"`, `"Presence"`, `"Brilliance"`, `"Air"`. |
| `syllable_count` | `(text: &str) -> u32` | Syllable count using a vowel-group heuristic with silent-e correction. |
| `density_lambda` | `(text: &str, _bpm: u32, bars: u32) -> f64` | Syllable density (syllables per beat) for a lyric over a bar count. |

---

### Business / Release Functions

| Function | Signature | Description |
|---|---|---|
| `release_timeline` | `(release_date: &str, template: &str) -> Result<Vec<TimelineEntry>, String>` | Generates a release schedule from a date. Template `"standard"` produces 8 milestones (master due → release day → first-week analytics). |
| `isrc_validate` | `(code: &str) -> bool` | Validates ISRC format `CC-XXX-YY-NNNNN` (12 alphanumeric chars). |
| `streaming_loudness_check` | `(lufs: f64, platform: &str) -> LoudnessReport` | Checks loudness compliance for Spotify (−14 LUFS), Apple Music (−16 LUFS), YouTube, Amazon, Tidal. Returns a `LoudnessReport` with `status` of `"pass"`, `"too_loud"`, or `"too_quiet"`. |

**Supporting types:**

```rust
pub struct TimelineEntry {
    pub date: String,           // YYYY-MM-DD
    pub label: String,          // e.g. "Pre-save campaign launch"
    pub days_before_release: i64,
}

pub struct LoudnessReport {
    pub platform: String,
    pub target_lufs: f64,
    pub measured_lufs: f64,
    pub adjustment_db: f64,
    pub status: String,         // "pass" | "too_loud" | "too_quiet"
}
```

---

### Finance Functions

| Function | Signature | Description |
|---|---|---|
| `percentage_change` | `(from: f64, to: f64) -> f64` | Percentage change between two values, rounded to 2 decimal places. |
| `compound_annual_growth` | `(start: f64, end: f64, years: f64) -> f64` | Compound annual growth rate as a percentage (e.g. 14.87). |
| `position_size` | `(capital: f64, risk_pct: f64, entry: f64, stop: f64) -> f64` | Maximum position size (units) given capital, risk %, entry, and stop-loss prices. |

---

### Session Cost Functions

| Function | Signature | Description |
|---|---|---|
| `session_cost_summary` | `(events: &[Event]) -> CostSummary` | Aggregates token usage and estimates USD cost (~$9/MTok blended rate). |
| `budget_check` | `(events: &[Event], max_tokens: u32) -> proto::BudgetStatus` | Returns a protobuf `BudgetStatus` with status string `"green"` / `"amber"` / `"yellow"` / `"red"` based on utilisation thresholds (50 % / 75 % / 90 %). |

```rust
pub struct CostSummary {
    pub total_tokens: u32,
    pub deterministic_calls: u32,
    pub llm_calls: u32,
    pub estimated_cost_usd: f64,
}
```

---

## Module: `domain`

**File:** `crates/common/src/domain.rs`

```rust
pub struct TriggerConfig {
    pub keywords: Vec<String>,
    pub priority: u32,
}
```

Each agent TOML file specifies a `[triggers]` table that deserialises into `TriggerConfig`. Keywords feed both the legacy `DomainClassifier` and the newer `ConfidenceLadderRouter` via `router_from_trigger_configs`.

```rust
pub struct DomainClassifier { /* … */ }

impl DomainClassifier {
    pub fn new() -> Self;
    pub fn register(&mut self, domain: &str, config: TriggerConfig);
    pub fn classify(&self, message: &str) -> Domain;
}
```

`Domain` is a lightweight newtype over a domain name string with helpers:

```rust
pub struct Domain(String);

impl Domain {
    pub fn new(name: String) -> Self;
    pub fn name(&self) -> &str;
    pub fn from_hint(hint: &str) -> Option<Self>; // returns None for empty string
}
```

---

## Module: `router`

**File:** `crates/common/src/router.rs`

The confidence-ladder router is the core of Nanosistant's deterministic routing. It runs queries through five tiers, each gated by a confidence threshold.

```
Input Query
    │
    ▼
[Tier 1] Aho-Corasick automaton ──► confidence ≥ 0.95 → route
    │
    ▼
[Tier 2] Regex pattern scoring ──► confidence ≥ 0.80 → route
    │
    ▼
[Tier 3] Weighted keyword scoring ── confidence ≥ 0.65 → route
    │
    ▼
[Tier 4] Fuzzy edit-distance ──── confidence ≥ 0.50 → route
    │
    ▼
[Tier 5] Combined score / LLM escape hatch
```

### `RouteDecision`

```rust
pub struct RouteDecision {
    pub domain: Option<String>,    // None = ambiguous
    pub confidence: f64,           // 0.0–1.0
    pub resolved_at_tier: u8,      // 1–5, or 0 for ambiguous
    pub scores: HashMap<String, f64>,
}

impl RouteDecision {
    pub fn is_confident(&self) -> bool;  // domain.is_some()
    pub fn is_ambiguous(&self) -> bool;  // domain.is_none()
}
```

### Pattern Types

```rust
pub struct RoutePattern {
    pub pattern: String,   // literal text (case-insensitive)
    pub domain: String,
    pub weight: f64,       // 0.0–1.0
    pub tags: Vec<String>,
}

pub struct RegexPattern {
    pub regex: Regex,
    pub domain: String,
    pub weight: f64,
}

pub struct WeightedKeyword {
    pub keyword: String,
    pub domain: String,
    pub weight: f64,
}

pub struct FuzzyAnchor {
    pub term: String,
    pub domain: String,
    pub weight: f64,
}
```

### `RouterThresholds`

```rust
pub struct RouterThresholds {
    pub tier1_ac: f64,        // default 0.95
    pub tier2_regex: f64,     // default 0.80
    pub tier3_weighted: f64,  // default 0.65
    pub tier4_fuzzy: f64,     // default 0.50
}
```

### `ConfidenceLadderRouter`

```rust
pub struct ConfidenceLadderRouter { /* private fields */ }

impl ConfidenceLadderRouter {
    /// Route a message through the confidence ladder.
    pub fn route(&self, message: &str) -> RouteDecision;

    /// Update weighted keywords at runtime (hot-reload from config).
    pub fn update_weighted_keywords(&mut self, keywords: Vec<WeightedKeyword>);

    /// Record LLM routing feedback for future weight updates.
    pub fn record_feedback(&mut self, query: &str, domain: &str, confidence: f64);

    pub fn thresholds(&self) -> &RouterThresholds;
    pub fn set_thresholds(&mut self, t: RouterThresholds);
}
```

### `RouterBuilder`

```rust
pub struct RouterBuilder { /* private fields */ }

impl RouterBuilder {
    pub fn new() -> Self;
    pub fn add_pattern(self, pattern: RoutePattern) -> Self;
    pub fn add_patterns(self, patterns: Vec<RoutePattern>) -> Self;
    pub fn add_regex(self, pattern: RegexPattern) -> Self;
    pub fn add_weighted_keyword(self, kw: WeightedKeyword) -> Self;
    pub fn add_weighted_keywords(self, kws: Vec<WeightedKeyword>) -> Self;
    pub fn add_fuzzy_anchor(self, anchor: FuzzyAnchor) -> Self;
    pub fn fuzzy_threshold(self, threshold: f64) -> Self;   // default 82.0 (0–100 scale)
    pub fn thresholds(self, t: RouterThresholds) -> Self;
    pub fn fallback_domain(self, domain: impl Into<String>) -> Self; // default "general"
    pub fn build(self) -> ConfidenceLadderRouter;
}
```

### Helper: `router_from_trigger_configs`

```rust
pub fn router_from_trigger_configs(
    configs: &[(String, TriggerConfig)],
) -> ConfidenceLadderRouter
```

Builds a `ConfidenceLadderRouter` from agent TOML-style trigger configs. Used by the `Orchestrator` at startup. Keyword length drives weight assignment (multi-word phrases → 0.95, long single words → 0.85, short words → 0.70). Uses relaxed thresholds (`tier1_ac = 0.30`, `tier3_weighted = 0.25`) to compensate for lower pattern density.

---

## Module: `events`

**File:** `crates/common/src/events.rs`

Every operation in the system emits a structured `Event`. Events feed the watchdog, budget manager, and session analytics.

### `EventType`

```rust
pub enum EventType {
    RoutingClassified,
    DeterministicExecuted,
    AgentTurnComplete,
    HandoffInitiated,
    HandoffValidated,
    HandoffRejected,
    BudgetThreshold,
    BudgetExhausted,
    WatchdogTriggered,
    KnowledgeQuery,
    SessionCompacted,
    Custom(String),
}
```

Each variant has an `as_str()` representation used in protobuf payloads (e.g. `"routing.classified"`, `"deterministic.executed"`).

### `Event`

```rust
pub struct Event {
    pub event_id: String,              // UUID v4
    pub timestamp: String,             // RFC 3339
    pub agent_id: String,
    pub session_id: String,
    pub thread_id: String,
    pub event_type: EventType,
    pub domain: String,
    pub payload: HashMap<String, String>,
    pub token_cost: u32,
    pub latency_ms: u32,
    pub was_deterministic: bool,
    pub distortion_flags: Vec<String>,
}

impl Event {
    pub fn new(event_type: EventType, agent_id: &str, session_id: &str, domain: &str) -> Self;
    pub fn deterministic(session_id: &str, domain: &str, function_name: &str) -> Self;
    pub fn routing(session_id: &str, domain: &str, message_preview: &str) -> Self;
    pub fn agent_turn(agent_id: &str, session_id: &str, domain: &str,
                      token_cost: u32, latency_ms: u32) -> Self;
    pub fn with_payload(self, key: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn to_proto(&self) -> proto::Event;
}
```

### `EventLog`

```rust
pub struct EventLog { /* private */ }

impl EventLog {
    pub fn new() -> Self;
    pub fn record(&mut self, event: Event);
    pub fn events(&self) -> &[Event];
    pub fn session_events(&self, session_id: &str) -> Vec<&Event>;
    pub fn total_tokens(&self) -> u32;
    pub fn call_breakdown(&self) -> (u32, u32);  // (deterministic, llm)
    pub fn events_of_type(&self, event_type: &EventType) -> Vec<&Event>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn clear(&mut self);
}
```

---

## Module: `handoff`

**File:** `crates/common/src/handoff.rs`

Typed validation of cross-agent handoff messages. Prevents specification-gap failures by enforcing required fields before delivery.

### `HandoffError`

```rust
pub enum HandoffError {
    MissingSourceAgent,
    MissingTargetAgent,
    SelfHandoff(String),           // source == target
    MissingTaskDescription,
    MissingUserIntent,
    UnknownTargetAgent(String),    // not in known_agents list
    FailureWithoutFlags,           // CompletionStatus::Failed but no distortion_flags
}
```

### `HandoffValidator`

```rust
pub struct HandoffValidator { /* private */ }

impl HandoffValidator {
    /// Create with a list of known (registered) agent names.
    pub fn new(known_agents: Vec<String>) -> Self;

    /// Validate a proto::AgentHandoff. Returns all errors (not just the first).
    pub fn validate(&self, handoff: &proto::AgentHandoff) -> Result<(), Vec<HandoffError>>;
}
```

---

## Module: `typed_ir`

**File:** `crates/common/src/typed_ir.rs`

**Design principle:** *"LLMs propose, Rust executes."* Every LLM output that leads to a state change must pass through a typed IR. No LLM call directly invokes a tool, writes a file, or changes routing weights.

### Routing Proposal

```rust
pub struct RoutingProposal {
    pub intent_class: String,
    pub confidence: f64,
    pub tool_chain: Vec<String>,
    pub preconditions: Vec<String>,
    pub fallback: String,   // "ask_user" | "next_tier" | "abort"
}

pub fn validate_routing_proposal(p: &RoutingProposal) -> Result<(), Vec<String>>;
```

### Execution Plan

```rust
pub struct ExecutionPlanIR {
    pub goal: String,
    pub steps: Vec<PlanStepIR>,
}

pub struct PlanStepIR {
    pub id: String,
    pub role: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub termination_condition: String,
    pub rollback: Option<String>,
}

pub fn validate_execution_plan(p: &ExecutionPlanIR) -> Result<(), Vec<String>>;
```

### Tool Ranking

```rust
pub struct ToolRanking {
    pub ranked_tools: Vec<RankedTool>,
    pub query: String,
}

pub struct RankedTool {
    pub tool: String,
    pub score: f64,         // 0.0–1.0
    pub rationale: String,
}

pub fn validate_tool_ranking(p: &ToolRanking) -> Result<(), Vec<String>>;
```

### Evaluation Signals

```rust
pub struct EvaluationSignal {
    pub structural: StructuralEval,
    pub semantic: SemanticEval,
    pub human: Option<HumanSignal>,
}

pub struct StructuralEval {
    pub schema_valid: bool,
    pub preconditions_met: bool,
    pub tool_calls_succeeded: bool,
    pub details: String,
}

impl StructuralEval {
    pub fn passed(&self) -> bool;  // all three fields true
}

pub struct SemanticEval {
    pub alignment: f64,                    // 0.0–1.0
    pub misalignment_reason: Option<String>,
}

pub struct HumanSignal {
    pub signal_type: HumanSignalType,
    pub delta: Option<String>,
}

pub enum HumanSignalType { Confirm, Correct, Abandon }
```

### Weight Delta

```rust
pub struct WeightDelta {
    pub task_type: String,
    pub route_stage: String,
    pub direction: WeightDirection,
    pub magnitude: WeightMagnitude,
    pub reason: String,
    pub supporting_episodes: Vec<String>,
}

pub enum WeightDirection { Increase, Decrease }
pub enum WeightMagnitude { Small, Medium, Large }
```

### Alignment Check

```rust
pub struct AlignmentCheck {
    pub action: String,
    pub check_results: AlignmentResults,
    pub decision: AlignmentDecision,
}

pub struct AlignmentResults {
    pub external_gate_required: bool,
    pub reversible: bool,
    pub touches_l3: bool,
    pub god_time_status: GodTimeStatus,
}

pub enum GodTimeStatus { Confirmed, DriftDetected, Unknown }

pub enum AlignmentDecision {
    Proceed,
    Block { reason: String },
    QueueForReview { reason: String },
}

pub fn validate_alignment_check(p: &AlignmentCheck) -> Result<(), Vec<String>>;
```

### Weight Update Gate

```rust
/// Returns true only when all three signals agree: structural passed,
/// semantic alignment > 0.5, and human signal is Confirm.
pub fn can_update_weights(signal: &EvaluationSignal) -> bool;
```

---

## Module: `proto`

**File:** Generated at build time from `proto/nanosistant.proto` via `tonic-build`.

```rust
pub mod proto {
    // gRPC service client/server stubs
    pub mod nano_claw_service_client { pub struct NanoClawServiceClient<T>; }
    pub mod nano_claw_service_server { pub trait NanoClawService; }

    // Core message types
    pub struct EdgeRequest {
        pub session_id: String,
        pub user_message: String,
        pub domain_hint: String,
    }

    pub struct EdgeResponse {
        pub session_id: String,
        pub response_text: String,
        pub responding_agent: String,
        pub events: Vec<Event>,
        pub budget: Option<BudgetStatus>,
        pub handoff: Option<AgentHandoff>,
        pub completion: i32,  // CompletionStatus enum
    }

    pub struct AgentHandoff {
        pub source_agent: String,
        pub target_agent: String,
        pub task_description: String,
        pub structured_data: HashMap<String, String>,
        pub context_keys: Vec<String>,
        pub user_intent: String,
        pub constraints: Vec<String>,
        pub source_completion: i32,  // CompletionStatus
        pub distortion_flags: Vec<String>,
    }

    pub struct BudgetStatus {
        pub tokens_used: u32,
        pub tokens_remaining: u32,
        pub estimated_cost_usd: f32,
        pub status: String,  // "green" | "amber" | "yellow" | "red"
    }

    pub struct Event {
        pub event_id: String,
        pub timestamp: String,
        pub agent_id: String,
        pub session_id: String,
        pub thread_id: String,
        pub event_type: String,
        pub domain: String,
        pub payload: HashMap<String, String>,
        pub token_cost: u32,
        pub latency_ms: u32,
        pub was_deterministic: bool,
        pub distortion_flags: Vec<String>,
    }

    pub enum CompletionStatus {
        Complete = 0,
        HandedOff = 1,
        Failed = 2,
        Aborted = 3,
    }
}
```

---

## Usage Example

```rust
use nstn_common::{
    RouterBuilder, RoutePattern, WeightedKeyword, FuzzyAnchor,
    try_deterministic_resolution, scale_degrees,
};

// Build a router
let router = RouterBuilder::new()
    .add_pattern(RoutePattern {
        pattern: "bpm".into(),
        domain: "music".into(),
        weight: 0.9,
        tags: vec![],
    })
    .add_weighted_keyword(WeightedKeyword {
        keyword: "verse".into(),
        domain: "music".into(),
        weight: 1.0,
    })
    .add_fuzzy_anchor(FuzzyAnchor {
        term: "refactor".into(),
        domain: "development".into(),
        weight: 0.85,
    })
    .fallback_domain("general")
    .build();

let decision = router.route("140 bpm bar duration");
println!("{:?}", decision.domain); // Some("music")

// Zero-token deterministic resolution
if let Some(answer) = try_deterministic_resolution("140 bpm bar duration") {
    println!("{answer}"); // "At 140 BPM (4/4): one bar = 1.714s"
}

// Music theory
let notes = scale_degrees("C", "major");
assert_eq!(notes, ["C", "D", "E", "F", "G", "A", "B"]);
```
