//! `RuFlo` — the deterministic brain tier of Nanosistant.
//!
//! Contains the orchestrator, agent configuration system,
//! agent factory, budget manager, and watchdog.
//!
//! Everything in this crate is deterministic Rust — no LLM inference.

pub mod agent_config;
pub mod agent_factory;
pub mod budget;
pub mod orchestrator;
pub mod watchdog;

pub use agent_config::{load_agent_configs, AgentConfig, KnowledgeConfig, PromptConfig, ToolsConfig};
pub use agent_factory::AgentHandle;
pub use budget::{BudgetError, BudgetManager, BudgetState};
pub use orchestrator::{Orchestrator, RouteResult};
pub use watchdog::{AlertSeverity, Watchdog, WatchdogAlert, WatchdogPattern};
