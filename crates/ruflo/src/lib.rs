//! `RuFlo` — the brain tier of Nanosistant.
//!
//! Rust wraps the confidence ladder plus ruflo:
//! - Deterministic functions (zero tokens)
//! - Confidence ladder (AC/Regex/Weighted/Fuzzy)
//! - ruflo MCP fallback (Q-learning/MoE/semantic via stdio JSON-RPC)
//!
//! Also: agent configuration, budget management, watchdog.

pub mod agent_config;
pub mod agent_factory;
pub mod budget;
pub mod dreamer;
pub mod dreamer_applier;
pub mod evaluators;
pub mod external_mirror;
pub mod god_time;
pub mod grpc_server;
pub mod mcp_bridge;
pub mod memory;
pub mod orchestrator;
pub mod ruflo_proxy;
pub mod session_store;
pub mod watchdog;

pub use agent_config::{load_agent_configs, AgentConfig, KnowledgeConfig, PromptConfig, ToolsConfig};
pub use agent_factory::{AgentFactory, AgentHandle, AgentRuntime, AgentTurnResult, MockAgentRuntime};
pub use budget::{BudgetError, BudgetManager, BudgetState};
pub use grpc_server::NanoClawGrpcService;
pub use mcp_bridge::{BridgeConfig, McpBridge};
pub use orchestrator::{Orchestrator, RouteResult};
pub use ruflo_proxy::{
    RufloProxy, RufloRouteResult, RufloRouterType, RufloModelSelection, RufloSwarmStatus,
    SwarmAgentHandle, SwarmAgentStatus, SwarmCoordinationResult,
};
pub use session_store::{PersistedSession, SessionMessage, SessionStore};
pub use memory::{
    EpisodicStore, ExecutionPlan, IdentityPolicy, L1Event, L3PatchProposal,
    LessonCard, LessonInstruction, MemorySystem, Outcome, PatchStatus, PlanStep,
    ProposedChange, RiskLevel, SemanticMemory, StepStatus, TaskType, ToolCallRecord,
    ToolCallStatus, WorkingContext,
};
pub use watchdog::{AlertSeverity, Watchdog, WatchdogAlert, WatchdogPattern};
pub use god_time::{GodTimeCheckResult, check_god_time};
pub use evaluators::{HumanSignalCollector, StructuralEvaluator, evaluate_for_learning};
pub use dreamer::{
    ClassificationConfidence, DreamerInput, DreamerLessonCard, DreamingReport,
    FailureClassification, HealthSignal, InsufficientEvidence, LensAnalysis, OrchestratorTask,
    Partition, Priority, RoutingWeightHint, SystemMode, TaskContext, ToolDescriptionFix,
    validate_dreaming_report,
};
pub use dreamer_applier::{ApplyResult, DreamerApplier};
pub use external_mirror::{ExternalMirror, MirrorNotification, NotificationType};
