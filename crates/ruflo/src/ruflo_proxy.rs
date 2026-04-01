//! Typed proxy to ruflo's routing and orchestration tools.
//!
//! This module provides a clean Rust interface over ruflo's MCP tools.
//! The orchestrator calls these methods when the confidence ladder
//! returns `Ambiguous` — ruflo's Q-learning, MoE, and semantic routers
//! then make the routing decision.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mcp_bridge::{BridgeError, McpBridge};

// ═══════════════════════════════════════
// Errors
// ═══════════════════════════════════════

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("bridge error: {0}")]
    Bridge(#[from] BridgeError),

    #[error("ruflo returned unexpected result: {0}")]
    UnexpectedResult(String),

    #[error("ruflo is not available (offline mode)")]
    Unavailable,
}

// ═══════════════════════════════════════
// Ruflo routing result types
// ═══════════════════════════════════════

/// Result from ruflo's routing stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RufloRouteResult {
    /// The domain/agent ruflo selected.
    pub route: String,
    /// Confidence score (0.0–1.0).
    pub confidence: f64,
    /// Which ruflo router made the decision.
    pub router_type: RufloRouterType,
    /// The model tier ruflo recommends.
    pub model_recommendation: Option<String>,
    /// Q-values or MoE weights if available.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Which ruflo routing algorithm resolved the query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RufloRouterType {
    /// Q-learning reinforcement learning router.
    QLearning,
    /// Mixture of Experts gating network.
    MixtureOfExperts,
    /// Semantic embedding similarity.
    Semantic,
    /// Model complexity router (haiku/sonnet/opus).
    ModelRouter,
    /// Intent router plugin.
    IntentRouter,
    /// Fallback / unknown.
    Fallback,
}

/// Result from ruflo's model selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RufloModelSelection {
    pub model: String,
    pub complexity_score: f64,
    pub cost_multiplier: f64,
    pub reason: String,
}

/// Swarm coordination status from ruflo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RufloSwarmStatus {
    pub active_agents: u32,
    pub topology: String,
    pub consensus_algorithm: String,
    pub tasks_queued: u32,
    pub tasks_completed: u32,
}

// ═══════════════════════════════════════
// Ruflo proxy
// ═══════════════════════════════════════

/// Typed proxy to ruflo's orchestration capabilities.
///
/// Methods on this struct map to specific ruflo MCP tools.
/// When ruflo is unavailable (offline mode), all methods return
/// `ProxyError::Unavailable` and the orchestrator falls back to
/// its own routing logic.
pub struct RufloProxy {
    bridge: McpBridge,
}

impl RufloProxy {
    /// Create a proxy backed by a live MCP bridge.
    #[must_use]
    pub fn new(bridge: McpBridge) -> Self {
        Self { bridge }
    }

    /// Create an offline proxy (all calls return Unavailable).
    #[must_use]
    pub fn offline() -> Self {
        Self {
            bridge: McpBridge::offline(),
        }
    }

    /// Whether ruflo is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.bridge.is_running()
    }

    /// Start the ruflo backend.
    pub fn start(&mut self) -> Result<(), ProxyError> {
        self.bridge.start().map_err(ProxyError::Bridge)
    }

    /// Stop the ruflo backend.
    pub fn stop(&mut self) -> Result<(), ProxyError> {
        self.bridge.stop().map_err(ProxyError::Bridge)
    }

    // ── Routing ──

    /// Ask ruflo to route a message using its full routing stack
    /// (Q-learning → MoE → semantic → intent → fallback).
    ///
    /// This is the primary fallback when the confidence ladder
    /// returns Ambiguous.
    pub fn route_message(
        &mut self,
        message: &str,
        context: &HashMap<String, String>,
    ) -> Result<RufloRouteResult, ProxyError> {
        if !self.bridge.is_running() {
            return Err(ProxyError::Unavailable);
        }

        let args = serde_json::json!({
            "message": message,
            "context": context,
        });

        let result = self.bridge.call_tool("hooks_route", args)?;

        // Parse the text content from the MCP result
        let text = result
            .content
            .first()
            .and_then(|c| c.text.as_ref())
            .ok_or_else(|| ProxyError::UnexpectedResult("empty content".into()))?;

        serde_json::from_str(text).map_err(|e| ProxyError::UnexpectedResult(e.to_string()))
    }

    /// Ask ruflo to select the optimal model for a given task.
    pub fn select_model(
        &mut self,
        message: &str,
        domain: &str,
    ) -> Result<RufloModelSelection, ProxyError> {
        if !self.bridge.is_running() {
            return Err(ProxyError::Unavailable);
        }

        let args = serde_json::json!({
            "task": message,
            "domain": domain,
        });

        let result = self.bridge.call_tool("model_route", args)?;

        let text = result
            .content
            .first()
            .and_then(|c| c.text.as_ref())
            .ok_or_else(|| ProxyError::UnexpectedResult("empty content".into()))?;

        serde_json::from_str(text).map_err(|e| ProxyError::UnexpectedResult(e.to_string()))
    }

    // ── Memory / Knowledge ──

    /// Store a routing outcome in ruflo's memory for learning.
    pub fn record_routing_outcome(
        &mut self,
        message: &str,
        routed_to: &str,
        success: bool,
    ) -> Result<(), ProxyError> {
        if !self.bridge.is_running() {
            return Err(ProxyError::Unavailable);
        }

        let args = serde_json::json!({
            "key": format!("routing:{}", &message[..message.len().min(50)]),
            "value": {
                "message": message,
                "routed_to": routed_to,
                "success": success,
            },
        });

        self.bridge.call_tool("hooks_remember", args)?;
        Ok(())
    }

    /// Recall similar past routing decisions from ruflo's memory.
    pub fn recall_similar_routing(
        &mut self,
        message: &str,
    ) -> Result<Vec<serde_json::Value>, ProxyError> {
        if !self.bridge.is_running() {
            return Err(ProxyError::Unavailable);
        }

        let args = serde_json::json!({
            "query": message,
            "limit": 5,
        });

        let result = self.bridge.call_tool("hooks_recall", args)?;

        let text = result
            .content
            .first()
            .and_then(|c| c.text.as_ref())
            .ok_or_else(|| ProxyError::UnexpectedResult("empty content".into()))?;

        serde_json::from_str(text).map_err(|e| ProxyError::UnexpectedResult(e.to_string()))
    }

    // ── Swarm ──

    /// Get the current swarm status from ruflo.
    pub fn swarm_status(&mut self) -> Result<RufloSwarmStatus, ProxyError> {
        if !self.bridge.is_running() {
            return Err(ProxyError::Unavailable);
        }

        let result = self.bridge.call_tool("swarm_status", serde_json::json!({}))?;

        let text = result
            .content
            .first()
            .and_then(|c| c.text.as_ref())
            .ok_or_else(|| ProxyError::UnexpectedResult("empty content".into()))?;

        serde_json::from_str(text).map_err(|e| ProxyError::UnexpectedResult(e.to_string()))
    }

    // ── Raw tool access ──

    /// Call any ruflo MCP tool by name (escape hatch for tools
    /// not yet wrapped in a typed method).
    pub fn call_raw(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, ProxyError> {
        if !self.bridge.is_running() {
            return Err(ProxyError::Unavailable);
        }

        let result = self.bridge.call_tool(tool_name, arguments)?;

        let text = result
            .content
            .first()
            .and_then(|c| c.text.as_ref())
            .cloned()
            .unwrap_or_default();

        serde_json::from_str(&text).or_else(|_| Ok(serde_json::json!({ "raw": text })))
    }

    /// List all available ruflo tools.
    #[must_use]
    pub fn available_tools(&self) -> Vec<String> {
        self.bridge.tools().keys().cloned().collect()
    }

    /// Number of available ruflo tools.
    #[must_use]
    pub fn tool_count(&self) -> usize {
        self.bridge.tools().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_proxy_returns_unavailable() {
        let mut proxy = RufloProxy::offline();
        assert!(!proxy.is_available());

        let result = proxy.route_message("test", &HashMap::new());
        assert!(matches!(result.unwrap_err(), ProxyError::Unavailable));

        let result = proxy.select_model("test", "music");
        assert!(matches!(result.unwrap_err(), ProxyError::Unavailable));
    }

    #[test]
    fn offline_proxy_has_no_tools() {
        let proxy = RufloProxy::offline();
        assert_eq!(proxy.tool_count(), 0);
        assert!(proxy.available_tools().is_empty());
    }

    #[test]
    fn ruflo_router_type_serializes() {
        let rt = RufloRouterType::QLearning;
        let json = serde_json::to_string(&rt).unwrap();
        assert!(json.contains("QLearning"));
    }

    #[test]
    fn ruflo_route_result_deserializes() {
        let json = r#"{
            "route": "music",
            "confidence": 0.87,
            "router_type": "MixtureOfExperts",
            "model_recommendation": "sonnet",
            "metadata": {"top_k": [0.87, 0.12]}
        }"#;
        let result: RufloRouteResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.route, "music");
        assert_eq!(result.router_type, RufloRouterType::MixtureOfExperts);
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn ruflo_model_selection_deserializes() {
        let json = r#"{
            "model": "claude-sonnet-4-20250514",
            "complexity_score": 0.45,
            "cost_multiplier": 0.2,
            "reason": "moderate complexity, balanced cost"
        }"#;
        let sel: RufloModelSelection = serde_json::from_str(json).unwrap();
        assert_eq!(sel.model, "claude-sonnet-4-20250514");
        assert!(sel.complexity_score < 0.5);
    }
}
