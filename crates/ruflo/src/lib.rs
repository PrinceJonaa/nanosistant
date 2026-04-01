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
pub mod grpc_server;
pub mod mcp_bridge;
pub mod orchestrator;
pub mod ruflo_proxy;
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
pub use watchdog::{AlertSeverity, Watchdog, WatchdogAlert, WatchdogPattern};
