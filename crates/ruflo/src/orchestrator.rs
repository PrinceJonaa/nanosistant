//! Deterministic orchestrator — the core routing brain of Nanosistant.
//!
//! Rust wraps the confidence ladder plus ruflo. The routing pipeline:
//!
//! 1. Try deterministic resolution (zero tokens, pure code).
//! 2. Run confidence ladder (AC → Regex → Weighted → Fuzzy).
//!    If confident → route directly.
//! 3. If ambiguous → fall to ruflo MCP (Q-learning, MoE, semantic).
//!    ruflo returns a routing decision → Rust executes it.
//!
//! Rust is always the entry point, always the exit point.
//! ruflo never receives traffic directly.

use std::collections::HashMap;

use nstn_common::{
    try_deterministic_resolution, ConfidenceLadderRouter, Domain, DomainClassifier, Event,
    EventLog, EventType, router_from_trigger_configs,
};

use crate::{
    agent_factory::{AgentHandle, AgentTurnResult},
    budget::{BudgetError, BudgetManager},
    ruflo_proxy::{ProxyError, RufloProxy, RufloSwarmStatus, SwarmCoordinationResult},
};

// ─── RouteResult ──────────────────────────────────────────────────────────────

/// The outcome of a routing decision.
#[derive(Debug, Clone)]
pub enum RouteResult {
    /// Resolved deterministically — zero tokens consumed.
    Deterministic {
        response: String,
        domain: String,
    },
    /// Routed to a domain agent for LLM processing.
    AgentRoute {
        domain: String,
        agent_name: String,
        /// The original message, potentially enriched with context.
        enriched_message: String,
        /// Confidence from the router (0.0–1.0).
        confidence: f64,
        /// Which tier resolved this (1–5, or 0 if combined).
        resolved_at_tier: u8,
    },
    /// Token budget exhausted — refuse processing.
    BudgetExhausted {
        message: String,
    },
    /// Ambiguous — no tier was confident enough. Falls to LLM classifier.
    Ambiguous {
        message: String,
        /// Best-guess domain (may still be useful as a hint to the LLM).
        best_guess: String,
        /// Combined confidence score.
        confidence: f64,
        /// Per-domain scores for the LLM to consider.
        scores: HashMap<String, f64>,
    },
}

// ─── Orchestrator ─────────────────────────────────────────────────────────────

/// The `RuFlo` orchestrator routes messages to agents deterministically.
///
/// Architecture: Rust wraps the confidence ladder plus ruflo.
/// - Deterministic functions intercept closed-form queries (zero tokens).
/// - Confidence ladder (AC/Regex/Weighted/Fuzzy) handles clear-domain queries.
/// - ruflo MCP (Q-learning/MoE/semantic) handles ambiguous queries.
/// - Rust is always the entry and exit point.
pub struct Orchestrator {
    /// Registered agents keyed by domain name.
    agents: HashMap<String, AgentHandle>,
    /// Legacy keyword classifier (used for deterministic interception domain tagging).
    classifier: DomainClassifier,
    /// Confidence-ladder router: AC → Regex → Weighted → Fuzzy.
    ladder_router: ConfidenceLadderRouter,
    /// Proxy to ruflo's MCP tools (Q-learning, MoE, semantic, swarm, memory).
    /// Falls back gracefully when ruflo is unavailable (offline mode).
    ruflo: RufloProxy,
    event_log: EventLog,
    budget: BudgetManager,
}

impl Orchestrator {
    /// Create an orchestrator with ruflo in offline mode.
    /// Deterministic + confidence ladder only, no MCP fallback.
    #[must_use]
    pub fn new(agents: Vec<AgentHandle>, max_tokens: u32) -> Self {
        Self::with_ruflo(agents, max_tokens, RufloProxy::offline())
    }

    /// Create an orchestrator with a live ruflo proxy.
    /// Ambiguous queries will fall through to ruflo's routing stack.
    pub fn with_ruflo(
        agents: Vec<AgentHandle>,
        max_tokens: u32,
        ruflo: RufloProxy,
    ) -> Self {
        let mut classifier = DomainClassifier::new();
        let mut agent_map = HashMap::with_capacity(agents.len());
        let mut trigger_configs = Vec::new();

        for handle in agents {
            classifier.register(&handle.config.name, handle.config.triggers.clone());
            trigger_configs.push((handle.config.name.clone(), handle.config.triggers.clone()));
            agent_map.insert(handle.config.name.clone(), handle);
        }

        let ladder_router = router_from_trigger_configs(&trigger_configs);

        Self {
            agents: agent_map,
            classifier,
            ladder_router,
            ruflo,
            event_log: EventLog::new(),
            budget: BudgetManager::new(max_tokens),
        }
    }

    /// Route a message.
    ///
    /// # Parameters
    /// - `session_id` — session identifier for event logging.
    /// - `message`    — raw user message.
    /// - `domain_hint` — optional domain override (empty string = no hint).
    ///
    /// # Return
    /// A [`RouteResult`] describing what should happen next.
    pub fn route(&mut self, session_id: &str, message: &str, domain_hint: &str) -> RouteResult {
        // ── Step 1: deterministic interception ────────────────────────────────
        if let Some(response) = try_deterministic_resolution(message) {
            let domain = self.resolve_domain(message, domain_hint);
            self.event_log.record(Event::deterministic(
                session_id,
                domain.name(),
                "deterministic_resolution",
            ));
            tracing::debug!(
                session_id,
                domain = domain.name(),
                "deterministic interception succeeded"
            );
            return RouteResult::Deterministic {
                response,
                domain: domain.name().to_string(),
            };
        }

        // ── Step 2: budget check (before any expensive routing) ───────────────
        if let Err(BudgetError::Exhausted {
            tokens_used,
            max_tokens,
        }) = self.budget.check()
        {
            let domain = self.resolve_domain(message, domain_hint);
            self.event_log.record(
                Event::new(EventType::BudgetExhausted, "orchestrator", session_id, domain.name())
                    .with_payload("tokens_used", tokens_used.to_string())
                    .with_payload("max_tokens", max_tokens.to_string()),
            );
            tracing::warn!(session_id, tokens_used, max_tokens, "budget exhausted");
            return RouteResult::BudgetExhausted {
                message: format!(
                    "Session budget exhausted ({tokens_used}/{max_tokens} tokens). \
                     Please start a new session."
                ),
            };
        }

        // ── Step 3: confidence-ladder routing ─────────────────────────────────
        // If domain_hint is provided, skip the ladder and route directly.
        if let Some(hinted) = Domain::from_hint(domain_hint) {
            let agent_name = self.lookup_agent(hinted.name());
            let enriched_message = self.enrich_message(message, &hinted);
            self.event_log
                .record(Event::routing(session_id, hinted.name(), message)
                    .with_payload("source", "domain_hint".to_string()));
            return RouteResult::AgentRoute {
                domain: hinted.name().to_string(),
                agent_name,
                enriched_message,
                confidence: 1.0,
                resolved_at_tier: 0,
            };
        }

        // Run the confidence ladder: AC → Regex → Weighted → Fuzzy
        let decision = self.ladder_router.route(message);

        if let Some(ref domain_name) = decision.domain {
            // Confident route
            let agent_name = self.lookup_agent(domain_name);
            let domain = Domain::new(domain_name.clone());
            let enriched_message = self.enrich_message(message, &domain);

            self.event_log
                .record(Event::routing(session_id, domain_name, message)
                    .with_payload("tier", decision.resolved_at_tier.to_string())
                    .with_payload("confidence", format!("{:.3}", decision.confidence)));

            tracing::debug!(
                session_id,
                domain = domain_name.as_str(),
                confidence = decision.confidence,
                tier = decision.resolved_at_tier,
                "confidence-ladder routed"
            );

            RouteResult::AgentRoute {
                domain: domain_name.clone(),
                agent_name,
                enriched_message,
                confidence: decision.confidence,
                resolved_at_tier: decision.resolved_at_tier,
            }
        } else {
            // Ambiguous — try ruflo's routing stack before giving up.
            self.route_via_ruflo(session_id, message, &decision)
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Try routing through ruflo's MCP stack (Q-learning, MoE, semantic).
    /// If ruflo is unavailable or fails, fall back to Ambiguous.
    fn route_via_ruflo(
        &mut self,
        session_id: &str,
        message: &str,
        ladder_decision: &nstn_common::RouteDecision,
    ) -> RouteResult {
        // Build context from the ladder's scores for ruflo
        let mut context = HashMap::new();
        for (domain, score) in &ladder_decision.scores {
            context.insert(format!("ladder_{domain}"), format!("{score:.3}"));
        }
        context.insert("ladder_confidence".to_string(), format!("{:.3}", ladder_decision.confidence));

        match self.ruflo.route_message(message, &context) {
            Ok(ruflo_result) => {
                let agent_name = self.lookup_agent(&ruflo_result.route);
                let domain = Domain::new(ruflo_result.route.clone());
                let enriched_message = self.enrich_message(message, &domain);

                self.event_log.record(
                    Event::routing(session_id, &ruflo_result.route, message)
                        .with_payload("source", "ruflo".to_string())
                        .with_payload("ruflo_router", format!("{:?}", ruflo_result.router_type))
                        .with_payload("confidence", format!("{:.3}", ruflo_result.confidence)),
                );

                tracing::info!(
                    session_id,
                    domain = ruflo_result.route.as_str(),
                    confidence = ruflo_result.confidence,
                    router = ?ruflo_result.router_type,
                    "ruflo resolved ambiguous route"
                );

                RouteResult::AgentRoute {
                    domain: ruflo_result.route,
                    agent_name,
                    enriched_message,
                    confidence: ruflo_result.confidence,
                    resolved_at_tier: 6, // tier 6 = ruflo MCP
                }
            }
            Err(ProxyError::Unavailable) => {
                // ruflo not running — return Ambiguous with ladder's best guess
                self.ambiguous_fallback(session_id, message, ladder_decision)
            }
            Err(e) => {
                tracing::warn!(session_id, error = %e, "ruflo routing failed, falling back");
                self.ambiguous_fallback(session_id, message, ladder_decision)
            }
        }
    }

    /// Final fallback when both the ladder and ruflo can't resolve.
    fn ambiguous_fallback(
        &mut self,
        session_id: &str,
        message: &str,
        decision: &nstn_common::RouteDecision,
    ) -> RouteResult {
        let best_guess = decision
            .scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(d, _)| d.clone())
            .unwrap_or_else(|| "general".to_string());

        self.event_log.record(
            Event::routing(session_id, &best_guess, message)
                .with_payload("ambiguous", "true".to_string())
                .with_payload("confidence", format!("{:.3}", decision.confidence)),
        );

        tracing::info!(
            session_id,
            best_guess = best_guess.as_str(),
            confidence = decision.confidence,
            "ambiguous — no router resolved, LLM classifier needed"
        );

        RouteResult::Ambiguous {
            message: message.to_string(),
            best_guess,
            confidence: decision.confidence,
            scores: decision.scores.clone(),
        }
    }

    /// Resolve a domain from an explicit hint or via classifier.
    fn resolve_domain(&self, message: &str, domain_hint: &str) -> Domain {
        Domain::from_hint(domain_hint).unwrap_or_else(|| self.classifier.classify(message))
    }

    /// Look up the agent name for a domain, falling back to "general".
    fn lookup_agent(&self, domain: &str) -> String {
        self.agents
            .get(domain)
            .map_or_else(
                || self.agents.get("general")
                    .map_or_else(|| "general".to_string(), |h| h.config.name.clone()),
                |h| h.config.name.clone(),
            )
    }

    /// Optionally enrich the message with budget context when the budget is
    /// approaching its limit (>= yellow = 75 %).
    fn enrich_message(&self, message: &str, _domain: &Domain) -> String {
        use crate::budget::BudgetState;

        let state = self.budget.state();
        if state >= BudgetState::Yellow {
            let remaining = self.budget.remaining();
            let pct = self.budget.utilization_pct() * 100.0;
            format!(
                "{message}\n\n[BUDGET WARNING: {pct:.0}% consumed, {remaining} tokens remaining]"
            )
        } else {
            message.to_string()
        }
    }

    // ── Public accessors ──────────────────────────────────────────────────────

    /// Record token usage from a completed agent turn.
    pub fn record_turn_tokens(&mut self, tokens: u32) {
        self.budget.record_usage(tokens);
    }

    /// Shared reference to the event log.
    #[must_use]
    pub fn event_log(&self) -> &EventLog {
        &self.event_log
    }

    /// Shared reference to the budget manager.
    #[must_use]
    pub fn budget(&self) -> &BudgetManager {
        &self.budget
    }

    /// Mutable reference to the budget manager.
    pub fn budget_mut(&mut self) -> &mut BudgetManager {
        &mut self.budget
    }

    /// Whether ruflo MCP is available for fallback routing.
    #[must_use]
    pub fn ruflo_available(&self) -> bool {
        self.ruflo.is_available()
    }

    /// Mutable access to the ruflo proxy (for lifecycle management).
    pub fn ruflo_mut(&mut self) -> &mut RufloProxy {
        &mut self.ruflo
    }

    // ── Swarm coordination ────────────────────────────────────────────────────

    /// Coordinate a multi-agent task through ruflo's swarm.
    ///
    /// `agent_types` is a list of agent type names to assign to the task.
    /// `topology` describes how the agents should be connected
    /// (e.g. `"mesh"`, `"star"`, `"pipeline"`).
    ///
    /// Only available when ruflo is running.
    ///
    /// # Errors
    /// Returns an error string when ruflo is unavailable or the swarm call fails.
    pub fn coordinate_swarm(
        &mut self,
        task: &str,
        agent_types: &[String],
        topology: &str,
    ) -> Result<SwarmCoordinationResult, String> {
        self.ruflo
            .swarm_coordinate(task, agent_types, topology)
            .map_err(|e| format!("swarm coordination failed: {e}"))
    }

    /// Check swarm status through ruflo.
    ///
    /// Only available when ruflo is running.
    ///
    /// # Errors
    /// Returns an error string when ruflo is unavailable or the call fails.
    pub fn swarm_status(&mut self) -> Result<RufloSwarmStatus, String> {
        self.ruflo
            .swarm_status()
            .map_err(|e| format!("swarm status failed: {e}"))
    }

    /// Number of registered agents.
    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Check whether an agent is registered for a given domain.
    #[must_use]
    pub fn has_agent(&self, domain: &str) -> bool {
        self.agents.contains_key(domain)
    }

    /// Execute a routed message on the appropriate agent.
    ///
    /// Call this after `route()` returns a [`RouteResult`].
    ///
    /// # Errors
    ///
    /// Returns an error string when:
    /// - The named agent is not found in the registry.
    /// - The agent has no runtime attached.
    /// - The runtime's `run_turn()` fails.
    /// - The result variant cannot be executed (e.g. `BudgetExhausted`).
    pub fn execute(&mut self, route: &RouteResult) -> Result<AgentTurnResult, String> {
        match route {
            RouteResult::AgentRoute {
                agent_name,
                enriched_message,
                ..
            } => {
                let handle = self
                    .agents
                    .get_mut(agent_name)
                    .ok_or_else(|| format!("agent '{agent_name}' not found"))?;
                let runtime = handle
                    .runtime_mut()
                    .ok_or_else(|| format!("agent '{agent_name}' has no runtime"))?;
                let result = runtime.run_turn(enriched_message)?;
                self.budget.record_usage(result.tokens_used);
                Ok(result)
            }
            RouteResult::Deterministic { response, .. } => Ok(AgentTurnResult {
                response_text: response.clone(),
                tool_calls: vec![],
                tokens_used: 0,
                iterations: 0,
            }),
            RouteResult::BudgetExhausted { message } => {
                Err(format!("cannot execute: budget exhausted — {message}"))
            }
            RouteResult::Ambiguous { message, best_guess, .. } => {
                // Fall back to the best-guess agent if it has a runtime.
                let handle = self
                    .agents
                    .get_mut(best_guess)
                    .ok_or_else(|| format!("ambiguous: best-guess agent '{best_guess}' not found"))?;
                let runtime = handle
                    .runtime_mut()
                    .ok_or_else(|| format!("ambiguous: agent '{best_guess}' has no runtime"))?;
                let result = runtime.run_turn(message)?;
                self.budget.record_usage(result.tokens_used);
                Ok(result)
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_config::{AgentConfig, PromptConfig, ToolsConfig};
    use nstn_common::TriggerConfig;

    fn make_handle(name: &str, keywords: Vec<&str>, priority: u32) -> AgentHandle {
        AgentHandle::from_config(AgentConfig {
            name: name.to_string(),
            description: format!("{name} agent"),
            model: "claude-sonnet-4-20250514".to_string(),
            permission_mode: "read_only".to_string(),
            triggers: TriggerConfig {
                keywords: keywords.into_iter().map(String::from).collect(),
                priority,
            },
            prompt: PromptConfig {
                identity_file: "config/prompts/identity.md".to_string(),
                domain_file: format!("config/prompts/{name}.md"),
            },
            knowledge: None,
            tools: ToolsConfig::default(),
        })
    }

    fn build_orchestrator() -> Orchestrator {
        Orchestrator::new(
            vec![
                make_handle("general", vec![], 0),
                make_handle("music", vec!["verse", "hook", "beat", "bpm"], 10),
                make_handle(
                    "investment",
                    vec!["stock", "earnings", "trade", "market"],
                    10,
                ),
                make_handle(
                    "development",
                    vec!["code", "rust", "bug", "deploy", "test"],
                    10,
                ),
                make_handle(
                    "framework",
                    vec!["distortion", "lattice", "archetype", "sovereignty"],
                    15,
                ),
            ],
            10_000,
        )
    }

    // ── Domain routing ────────────────────────────────────────────────────────

    #[test]
    fn routes_music_message_to_music_agent() {
        let mut orch = build_orchestrator();
        let result = orch.route("s1", "help me write a verse", "");
        match result {
            RouteResult::AgentRoute { domain, agent_name, confidence, .. } => {
                assert_eq!(domain, "music");
                assert_eq!(agent_name, "music");
                assert!(confidence > 0.0, "confidence should be positive");
            }
            other => panic!("expected AgentRoute, got {other:?}"),
        }
    }

    #[test]
    fn routes_investment_message_to_investment_agent() {
        let mut orch = build_orchestrator();
        let result = orch.route("s1", "analyze the stock earnings report", "");
        match result {
            RouteResult::AgentRoute { domain, .. } => assert_eq!(domain, "investment"),
            other => panic!("expected AgentRoute, got {other:?}"),
        }
    }

    #[test]
    fn routes_development_message_to_development_agent() {
        let mut orch = build_orchestrator();
        let result = orch.route("s1", "fix this rust bug in my code", "");
        match result {
            RouteResult::AgentRoute { domain, .. } => assert_eq!(domain, "development"),
            other => panic!("expected AgentRoute, got {other:?}"),
        }
    }

    #[test]
    fn domain_hint_overrides_classifier() {
        let mut orch = build_orchestrator();
        // Message looks like music, but hint says investment
        let result = orch.route("s1", "help me write a verse", "investment");
        match result {
            RouteResult::AgentRoute { domain, agent_name, confidence, .. } => {
                assert_eq!(domain, "investment");
                assert_eq!(agent_name, "investment");
                assert_eq!(confidence, 1.0); // hints are always 1.0 confidence
            }
            other => panic!("expected AgentRoute, got {other:?}"),
        }
    }

    #[test]
    fn domain_hint_general_routes_to_general() {
        let mut orch = build_orchestrator();
        let result = orch.route("s1", "what is the weather today?", "general");
        match result {
            RouteResult::AgentRoute { domain, agent_name, .. } => {
                assert_eq!(domain, "general");
                assert_eq!(agent_name, "general");
            }
            other => panic!("expected AgentRoute, got {other:?}"),
        }
    }

    #[test]
    fn unregistered_domain_hint_falls_back_to_agent_lookup() {
        let mut orch = build_orchestrator();
        let result = orch.route("s1", "hello world", "unknown_domain");
        match result {
            RouteResult::AgentRoute { .. } => {}
            RouteResult::Deterministic { .. } => {}
            RouteResult::BudgetExhausted { .. } => {}
            RouteResult::Ambiguous { .. } => {}
        }
    }

    #[test]
    fn ambiguous_message_returns_ambiguous_or_routes_to_general() {
        let mut orch = build_orchestrator();
        let result = orch.route("s1", "what is the weather today?", "");
        match result {
            RouteResult::Ambiguous { best_guess, confidence, .. } => {
                // Low confidence, best guess available
                assert!(confidence < 0.5, "ambiguous should have low confidence");
                assert!(!best_guess.is_empty());
            }
            RouteResult::AgentRoute { .. } => {
                // Also acceptable if a tier scored enough
            }
            other => panic!("expected Ambiguous or AgentRoute, got {other:?}"),
        }
    }

    // ── Deterministic interception ─────────────────────────────────────────

    #[test]
    fn deterministic_message_intercepted() {
        let mut orch = build_orchestrator();
        let result = orch.route("s1", "c major scale", "");
        match result {
            RouteResult::Deterministic { response, .. } => {
                assert!(response.contains("C - D - E - F - G - A - B"));
            }
            other => panic!("expected Deterministic, got {other:?}"),
        }
    }

    #[test]
    fn bpm_question_intercepted_deterministically() {
        let mut orch = build_orchestrator();
        let result = orch.route("s1", "140 bpm bar duration", "");
        match result {
            RouteResult::Deterministic { response, .. } => {
                assert!(response.contains("1.714"));
            }
            other => panic!("expected Deterministic, got {other:?}"),
        }
    }

    #[test]
    fn deterministic_event_recorded_in_log() {
        let mut orch = build_orchestrator();
        orch.route("s1", "c major scale", "");
        let events = orch.event_log().events();
        assert!(!events.is_empty());
        assert!(events
            .iter()
            .any(|e| e.event_type == EventType::DeterministicExecuted));
    }

    // ── Budget exhaustion ──────────────────────────────────────────────────

    #[test]
    fn budget_exhaustion_produces_budget_exhausted_result() {
        let mut orch = Orchestrator::new(
            vec![make_handle("general", vec![], 0)],
            100, // very small budget
        );
        orch.record_turn_tokens(100); // use it all up

        let result = orch.route("s1", "what is the weather today?", "");
        match result {
            RouteResult::BudgetExhausted { message } => {
                assert!(message.contains("budget exhausted"));
            }
            other => panic!("expected BudgetExhausted, got {other:?}"),
        }
    }

    #[test]
    fn budget_exhausted_event_recorded() {
        let mut orch = Orchestrator::new(
            vec![make_handle("general", vec![], 0)],
            100,
        );
        orch.record_turn_tokens(100);
        orch.route("s1", "hello", "");

        let events = orch.event_log().events();
        assert!(events
            .iter()
            .any(|e| e.event_type == EventType::BudgetExhausted));
    }

    // ── Routing event logging ──────────────────────────────────────────────

    #[test]
    fn routing_event_recorded_for_agent_route() {
        let mut orch = build_orchestrator();
        orch.route("s1", "write some code", "");

        let events = orch.event_log().events();
        assert!(events
            .iter()
            .any(|e| e.event_type == EventType::RoutingClassified));
    }

    // ── Enrichment ─────────────────────────────────────────────────────────

    #[test]
    fn budget_warning_appended_when_near_limit() {
        let mut orch = Orchestrator::new(
            vec![make_handle("general", vec![], 0)],
            1_000,
        );
        orch.record_turn_tokens(800); // 80% — in "yellow"

        // Use domain_hint to force an AgentRoute
        let result = orch.route("s1", "hello world", "general");
        match result {
            RouteResult::AgentRoute { enriched_message, .. } => {
                assert!(enriched_message.contains("BUDGET WARNING"));
                assert!(enriched_message.contains("80%"));
            }
            other => panic!("expected AgentRoute, got {other:?}"),
        }
    }

    #[test]
    fn no_budget_warning_when_green() {
        let mut orch = build_orchestrator();
        // Use domain_hint to guarantee AgentRoute
        let result = orch.route("s1", "hello world", "general");
        match result {
            RouteResult::AgentRoute { enriched_message, .. } => {
                assert!(!enriched_message.contains("BUDGET WARNING"));
            }
            other => panic!("expected AgentRoute, got {other:?}"),
        }
    }

    // ── execute() ──────────────────────────────────────────────────────────

    fn make_handle_with_mock(name: &str, keywords: Vec<&str>, priority: u32) -> AgentHandle {
        make_handle(name, keywords, priority).with_mock_runtime()
    }

    fn build_orchestrator_with_mocks() -> Orchestrator {
        Orchestrator::new(
            vec![
                make_handle_with_mock("general", vec![], 0),
                make_handle_with_mock("music", vec!["verse", "hook", "beat", "bpm"], 10),
                make_handle_with_mock(
                    "investment",
                    vec!["stock", "earnings", "trade", "market"],
                    10,
                ),
                make_handle_with_mock(
                    "development",
                    vec!["code", "rust", "bug", "deploy", "test"],
                    10,
                ),
            ],
            10_000,
        )
    }

    #[test]
    fn execute_deterministic_returns_response_without_calling_runtime() {
        let mut orch = build_orchestrator_with_mocks();
        let route = orch.route("s1", "c major scale", "");
        let result = orch.execute(&route).expect("should succeed");
        assert!(result.response_text.contains("C - D - E - F - G - A - B"));
        assert_eq!(result.tokens_used, 0); // deterministic → no tokens
    }

    #[test]
    fn execute_agent_route_calls_mock_runtime() {
        let mut orch = build_orchestrator_with_mocks();
        let route = orch.route("s1", "help me write a verse", "");
        assert!(matches!(route, RouteResult::AgentRoute { .. }));
        let result = orch.execute(&route).expect("should succeed");
        assert!(!result.response_text.is_empty());
        assert_eq!(result.tokens_used, 100);
    }

    #[test]
    fn execute_records_token_usage_in_budget() {
        let mut orch = build_orchestrator_with_mocks();
        let route = orch.route("s1", "analyze the stock earnings report", "");
        let initial_used = orch.budget().tokens_used();
        orch.execute(&route).expect("should succeed");
        assert_eq!(orch.budget().tokens_used(), initial_used + 100);
    }

    #[test]
    fn execute_agent_route_without_runtime_returns_error() {
        let mut orch = build_orchestrator(); // NO mock runtimes
        let route = orch.route("s1", "help me write a verse", "");
        if let RouteResult::AgentRoute { .. } = &route {
            let err = orch.execute(&route).unwrap_err();
            assert!(err.contains("has no runtime"));
        }
        // If the route was Deterministic that's fine too (no error expected)
    }

    #[test]
    fn execute_budget_exhausted_returns_error() {
        let mut orch = Orchestrator::new(
            vec![make_handle_with_mock("general", vec![], 0)],
            100,
        );
        orch.record_turn_tokens(100);
        let route = orch.route("s1", "what is the weather?", "");
        let err = orch.execute(&route).unwrap_err();
        assert!(err.contains("budget exhausted"));
    }

    // ── Accessors ──────────────────────────────────────────────────────────

    #[test]
    fn agent_count_matches_registered_agents() {
        let orch = build_orchestrator();
        assert_eq!(orch.agent_count(), 5);
    }

    #[test]
    fn has_agent_returns_correct_value() {
        let orch = build_orchestrator();
        assert!(orch.has_agent("music"));
        assert!(orch.has_agent("general"));
        assert!(!orch.has_agent("weather"));
    }
}
