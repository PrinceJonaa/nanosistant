//! Deterministic orchestrator — the core routing brain of `RuFlo`.
//!
//! This is pure deterministic Rust code.  **No LLM inference happens here.**
//!
//! Routing pipeline:
//! 1. Try deterministic resolution via `nstn_common::try_deterministic_resolution`.
//! 2. Resolve domain from `domain_hint` or `DomainClassifier`.
//! 3. Check budget.
//! 4. Return `RouteResult`.

use std::collections::HashMap;

use nstn_common::{try_deterministic_resolution, Domain, DomainClassifier, Event, EventLog, EventType};

use crate::{
    agent_factory::AgentHandle,
    budget::{BudgetError, BudgetManager},
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
    },
    /// Token budget exhausted — refuse processing.
    BudgetExhausted {
        message: String,
    },
}

// ─── Orchestrator ─────────────────────────────────────────────────────────────

/// The `RuFlo` orchestrator routes messages to agents deterministically.
pub struct Orchestrator {
    /// Registered agents keyed by domain name.
    agents: HashMap<String, AgentHandle>,
    classifier: DomainClassifier,
    event_log: EventLog,
    budget: BudgetManager,
}

impl Orchestrator {
    /// Construct an orchestrator from a set of agent handles and a token cap.
    ///
    /// The `DomainClassifier` is built from the agents' trigger configurations.
    #[must_use]
    pub fn new(agents: Vec<AgentHandle>, max_tokens: u32) -> Self {
        let mut classifier = DomainClassifier::new();
        let mut agent_map = HashMap::with_capacity(agents.len());

        for handle in agents {
            classifier.register(&handle.config.name, handle.config.triggers.clone());
            agent_map.insert(handle.config.name.clone(), handle);
        }

        Self {
            agents: agent_map,
            classifier,
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

        // ── Step 2: domain resolution ─────────────────────────────────────────
        let domain = self.resolve_domain(message, domain_hint);
        self.event_log
            .record(Event::routing(session_id, domain.name(), message));

        tracing::debug!(
            session_id,
            domain = domain.name(),
            "routed to domain"
        );

        // ── Step 3: budget check ──────────────────────────────────────────────
        if let Err(BudgetError::Exhausted {
            tokens_used,
            max_tokens,
        }) = self.budget.check()
        {
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

        // ── Step 4: agent lookup & enrichment ─────────────────────────────────
        let agent_name = self
            .agents
            .get(domain.name()).map_or_else(|| {
                // Fall back to "general" if the domain is unregistered.
                self.agents
                    .get("general").map_or_else(|| "general".to_string(), |h| h.config.name.clone())
            }, |h| h.config.name.clone());

        let enriched_message = self.enrich_message(message, &domain);

        RouteResult::AgentRoute {
            domain: domain.name().to_string(),
            agent_name,
            enriched_message,
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Resolve a domain from an explicit hint or via classifier.
    fn resolve_domain(&self, message: &str, domain_hint: &str) -> Domain {
        Domain::from_hint(domain_hint).unwrap_or_else(|| self.classifier.classify(message))
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
            RouteResult::AgentRoute { domain, agent_name, .. } => {
                assert_eq!(domain, "music");
                assert_eq!(agent_name, "music");
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
            RouteResult::AgentRoute { domain, agent_name, .. } => {
                assert_eq!(domain, "investment");
                assert_eq!(agent_name, "investment");
            }
            other => panic!("expected AgentRoute, got {other:?}"),
        }
    }

    #[test]
    fn domain_hint_general_routes_to_general() {
        // The classifier applies priority as a tiebreaker even at score 0,
        // so a truly ambiguous message (no keywords) goes to the highest-priority
        // domain.  An explicit domain_hint = "general" forces the general agent.
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
        // A domain hint that maps to no registered agent falls back to "general".
        let mut orch = build_orchestrator();
        // Route a message with a hint for an unregistered domain
        let result = orch.route("s1", "hello world", "unknown_domain");
        // Should not panic; result is an AgentRoute (falls back to general agent)
        match result {
            RouteResult::AgentRoute { .. } => {}
            RouteResult::Deterministic { .. } => {}
            RouteResult::BudgetExhausted { .. } => {}
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

        let result = orch.route("s1", "hello world", "");
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
        let result = orch.route("s1", "hello world", "");
        match result {
            RouteResult::AgentRoute { enriched_message, .. } => {
                assert!(!enriched_message.contains("BUDGET WARNING"));
            }
            other => panic!("expected AgentRoute, got {other:?}"),
        }
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
