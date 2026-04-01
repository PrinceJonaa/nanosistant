//! Watchdog — pattern detection on event streams.
//!
//! Detects anti-patterns that indicate agent dysfunction, budget blindness,
//! or specification failures.  All detection is deterministic — no LLM.

use nstn_common::{Event, EventType};

use crate::budget::BudgetManager;

// ─── Types ────────────────────────────────────────────────────────────────────

/// Patterns the watchdog monitors for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchdogPattern {
    /// Same problem described 3+ times across turns.
    StuckLoop,
    /// > 40 % of session tokens on non-productive operations.
    TokenWaste,
    /// 2+ handoff rejections in a session.
    HandoffFailure,
    /// Running past 75 % budget without any awareness event.
    BudgetBlindness,
    /// Same output structure 3+ times.
    SpecRepetition,
}

impl WatchdogPattern {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StuckLoop => "stuck_loop",
            Self::TokenWaste => "token_waste",
            Self::HandoffFailure => "handoff_failure",
            Self::BudgetBlindness => "budget_blindness",
            Self::SpecRepetition => "spec_repetition",
        }
    }
}

impl std::fmt::Display for WatchdogPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Alert severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// A triggered watchdog alert.
#[derive(Debug, Clone)]
pub struct WatchdogAlert {
    pub pattern: WatchdogPattern,
    pub description: String,
    pub severity: AlertSeverity,
}

impl WatchdogAlert {
    fn new(pattern: WatchdogPattern, description: impl Into<String>, severity: AlertSeverity) -> Self {
        Self {
            pattern,
            description: description.into(),
            severity,
        }
    }
}

// ─── Watchdog ─────────────────────────────────────────────────────────────────

/// Stateless watchdog that inspects an event slice and budget state.
pub struct Watchdog;

impl Watchdog {
    /// Analyse an event stream for dysfunction patterns.
    ///
    /// Returns a (possibly empty) list of `WatchdogAlert`s.
    #[must_use]
    pub fn check(events: &[Event], budget: &BudgetManager) -> Vec<WatchdogAlert> {
        let mut alerts = Vec::new();

        if let Some(alert) = Self::detect_stuck_loop(events) {
            alerts.push(alert);
        }
        if let Some(alert) = Self::detect_token_waste(events) {
            alerts.push(alert);
        }
        if let Some(alert) = Self::detect_handoff_failure(events) {
            alerts.push(alert);
        }
        if let Some(alert) = Self::detect_budget_blindness(events, budget) {
            alerts.push(alert);
        }
        if let Some(alert) = Self::detect_spec_repetition(events) {
            alerts.push(alert);
        }

        alerts
    }

    // ── StuckLoop ─────────────────────────────────────────────────────────────
    //
    // Heuristic: if the same domain appears in 3+ consecutive routing events
    // with no deterministic or handoff event in between, the agent is looping.

    fn detect_stuck_loop(events: &[Event]) -> Option<WatchdogAlert> {
        // Collect routing events in order.
        let routing_events: Vec<&Event> = events
            .iter()
            .filter(|e| e.event_type == EventType::RoutingClassified)
            .collect();

        if routing_events.len() < 3 {
            return None;
        }

        // Check last 3 routing events for same domain.
        let last = &routing_events[routing_events.len().saturating_sub(3)..];
        let first_domain = &last[0].domain;
        let all_same = last.iter().all(|e| &e.domain == first_domain);

        // Check that there was no resolved deterministic event in the window
        // between the first and last of the three routing events.
        let first_ts = &last[0].timestamp;
        let last_ts = &last[last.len() - 1].timestamp;

        let has_deterministic_break = events.iter().any(|e| {
            e.was_deterministic
                && matches!(e.event_type, EventType::DeterministicExecuted | EventType::HandoffValidated)
                && e.timestamp >= *first_ts
                && e.timestamp <= *last_ts
        });

        if all_same && !has_deterministic_break {
            Some(WatchdogAlert::new(
                WatchdogPattern::StuckLoop,
                format!(
                    "Domain '{first_domain}' appears in 3+ consecutive routing events with no resolution"
                ),
                AlertSeverity::Critical,
            ))
        } else {
            None
        }
    }

    // ── TokenWaste ────────────────────────────────────────────────────────────
    //
    // If >40% of total tokens were used by events flagged with
    // non-productive distortion flags (e.g. "hallucination", "retried", etc.)
    // OR non-agent events that consumed tokens, alert.

    fn detect_token_waste(events: &[Event]) -> Option<WatchdogAlert> {
        let total_tokens: u32 = events.iter().map(|e| e.token_cost).sum();
        if total_tokens == 0 {
            return None;
        }

        // Count tokens used on non-productive operations:
        // Events with distortion flags or events that are not AgentTurnComplete
        // but still consumed tokens (e.g. repeated KnowledgeQuery failures).
        let waste_tokens: u32 = events
            .iter()
            .filter(|e| {
                !e.distortion_flags.is_empty()
                    || (e.token_cost > 0 && matches!(e.event_type, EventType::KnowledgeQuery))
            })
            .map(|e| e.token_cost)
            .sum();

        #[allow(clippy::cast_precision_loss)]
        let waste_pct = f64::from(waste_tokens) / f64::from(total_tokens);

        if waste_pct > 0.40 {
            Some(WatchdogAlert::new(
                WatchdogPattern::TokenWaste,
                format!(
                    "{:.0}% of session tokens ({waste_tokens}/{total_tokens}) used on non-productive operations",
                    waste_pct * 100.0
                ),
                AlertSeverity::Warning,
            ))
        } else {
            None
        }
    }

    // ── HandoffFailure ────────────────────────────────────────────────────────
    //
    // 2+ HandoffRejected events in the same session → critical.

    fn detect_handoff_failure(events: &[Event]) -> Option<WatchdogAlert> {
        let rejections = events
            .iter()
            .filter(|e| e.event_type == EventType::HandoffRejected)
            .count();

        if rejections >= 2 {
            Some(WatchdogAlert::new(
                WatchdogPattern::HandoffFailure,
                format!("{rejections} handoff rejections detected in session"),
                AlertSeverity::Critical,
            ))
        } else {
            None
        }
    }

    // ── BudgetBlindness ───────────────────────────────────────────────────────
    //
    // Budget >75% but no BudgetThreshold event has been recorded.

    fn detect_budget_blindness(events: &[Event], budget: &BudgetManager) -> Option<WatchdogAlert> {
        let utilization = budget.utilization_pct();
        if utilization < 0.75 {
            return None;
        }

        let has_threshold_event = events
            .iter()
            .any(|e| e.event_type == EventType::BudgetThreshold);

        if has_threshold_event {
            None
        } else {
            Some(WatchdogAlert::new(
                WatchdogPattern::BudgetBlindness,
                format!(
                    "Budget at {:.0}% but no BudgetThreshold event recorded — agent may be unaware",
                    utilization * 100.0
                ),
                AlertSeverity::Warning,
            ))
        }
    }

    // ── SpecRepetition ────────────────────────────────────────────────────────
    //
    // 3+ AgentTurnComplete events with the same "output_structure" payload key.

    fn detect_spec_repetition(events: &[Event]) -> Option<WatchdogAlert> {
        let turn_events: Vec<&Event> = events
            .iter()
            .filter(|e| e.event_type == EventType::AgentTurnComplete)
            .collect();

        if turn_events.len() < 3 {
            return None;
        }

        // Group by output_structure value.
        let mut structure_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for event in &turn_events {
            if let Some(structure) = event.payload.get("output_structure") {
                *structure_counts.entry(structure.as_str()).or_insert(0) += 1;
            }
        }

        if let Some((structure, count)) = structure_counts.iter().find(|(_, &c)| c >= 3) {
            Some(WatchdogAlert::new(
                WatchdogPattern::SpecRepetition,
                format!(
                    "Output structure '{structure}' produced {count} times — potential specification repetition"
                ),
                AlertSeverity::Warning,
            ))
        } else {
            None
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nstn_common::{Event, EventType};

    fn routing_event(domain: &str) -> Event {
        Event::routing("s1", domain, "test message")
    }

    fn agent_turn_event(domain: &str, tokens: u32) -> Event {
        Event::agent_turn("agent", "s1", domain, tokens, 100)
    }

    fn handoff_rejected_event(domain: &str) -> Event {
        let mut e = Event::new(EventType::HandoffRejected, "orchestrator", "s1", domain);
        e.was_deterministic = false;
        e
    }

    fn budget_threshold_event() -> Event {
        Event::new(EventType::BudgetThreshold, "orchestrator", "s1", "general")
    }

    fn agent_turn_with_structure(structure: &str) -> Event {
        let mut e = agent_turn_event("general", 100);
        e.payload.insert("output_structure".to_string(), structure.to_string());
        e
    }

    fn agent_turn_with_flags(flags: Vec<&str>, tokens: u32) -> Event {
        let mut e = agent_turn_event("general", tokens);
        e.distortion_flags = flags.into_iter().map(String::from).collect();
        e
    }

    // ── StuckLoop ─────────────────────────────────────────────────────────────

    #[test]
    fn detects_stuck_loop_three_same_domain() {
        let events = vec![
            routing_event("music"),
            routing_event("music"),
            routing_event("music"),
        ];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(alerts.iter().any(|a| a.pattern == WatchdogPattern::StuckLoop));
    }

    #[test]
    fn no_stuck_loop_with_different_domains() {
        let events = vec![
            routing_event("music"),
            routing_event("investment"),
            routing_event("music"),
        ];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(!alerts.iter().any(|a| a.pattern == WatchdogPattern::StuckLoop));
    }

    #[test]
    fn no_stuck_loop_with_only_two_events() {
        let events = vec![routing_event("music"), routing_event("music")];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(!alerts.iter().any(|a| a.pattern == WatchdogPattern::StuckLoop));
    }

    // ── TokenWaste ────────────────────────────────────────────────────────────

    #[test]
    fn detects_token_waste_above_threshold() {
        let mut events = vec![
            agent_turn_event("music", 500), // productive
        ];
        // >40% waste via distortion flags
        events.push(agent_turn_with_flags(vec!["hallucination"], 400));

        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(alerts.iter().any(|a| a.pattern == WatchdogPattern::TokenWaste));
    }

    #[test]
    fn no_token_waste_when_below_threshold() {
        let events = vec![
            agent_turn_event("music", 900),
            agent_turn_with_flags(vec!["minor_issue"], 50), // <40%
        ];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(!alerts.iter().any(|a| a.pattern == WatchdogPattern::TokenWaste));
    }

    #[test]
    fn no_token_waste_with_no_tokens() {
        let events: Vec<Event> = vec![];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(!alerts.iter().any(|a| a.pattern == WatchdogPattern::TokenWaste));
    }

    // ── HandoffFailure ────────────────────────────────────────────────────────

    #[test]
    fn detects_handoff_failure_two_rejections() {
        let events = vec![
            handoff_rejected_event("music"),
            handoff_rejected_event("music"),
        ];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(alerts.iter().any(|a| a.pattern == WatchdogPattern::HandoffFailure));
        assert!(alerts
            .iter()
            .find(|a| a.pattern == WatchdogPattern::HandoffFailure)
            .unwrap()
            .severity == AlertSeverity::Critical);
    }

    #[test]
    fn no_handoff_failure_with_one_rejection() {
        let events = vec![handoff_rejected_event("music")];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(!alerts.iter().any(|a| a.pattern == WatchdogPattern::HandoffFailure));
    }

    // ── BudgetBlindness ───────────────────────────────────────────────────────

    #[test]
    fn detects_budget_blindness_above_75pct_no_threshold_event() {
        let events: Vec<Event> = vec![];
        let mut bm = BudgetManager::new(1_000);
        bm.record_usage(800); // 80%

        let alerts = Watchdog::check(&events, &bm);
        assert!(alerts.iter().any(|a| a.pattern == WatchdogPattern::BudgetBlindness));
    }

    #[test]
    fn no_budget_blindness_when_threshold_event_present() {
        let events = vec![budget_threshold_event()];
        let mut bm = BudgetManager::new(1_000);
        bm.record_usage(800); // 80%

        let alerts = Watchdog::check(&events, &bm);
        assert!(!alerts.iter().any(|a| a.pattern == WatchdogPattern::BudgetBlindness));
    }

    #[test]
    fn no_budget_blindness_when_under_75pct() {
        let events: Vec<Event> = vec![];
        let mut bm = BudgetManager::new(1_000);
        bm.record_usage(700); // 70%

        let alerts = Watchdog::check(&events, &bm);
        assert!(!alerts.iter().any(|a| a.pattern == WatchdogPattern::BudgetBlindness));
    }

    // ── SpecRepetition ────────────────────────────────────────────────────────

    #[test]
    fn detects_spec_repetition_three_same_structure() {
        let events = vec![
            agent_turn_with_structure("json_report"),
            agent_turn_with_structure("json_report"),
            agent_turn_with_structure("json_report"),
        ];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(alerts.iter().any(|a| a.pattern == WatchdogPattern::SpecRepetition));
    }

    #[test]
    fn no_spec_repetition_with_varying_structures() {
        let events = vec![
            agent_turn_with_structure("json_report"),
            agent_turn_with_structure("markdown_list"),
            agent_turn_with_structure("prose"),
        ];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(!alerts.iter().any(|a| a.pattern == WatchdogPattern::SpecRepetition));
    }

    #[test]
    fn no_spec_repetition_without_output_structure_payload() {
        let events = vec![
            agent_turn_event("general", 100),
            agent_turn_event("general", 100),
            agent_turn_event("general", 100),
        ];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(!alerts.iter().any(|a| a.pattern == WatchdogPattern::SpecRepetition));
    }

    // ── Multiple patterns ─────────────────────────────────────────────────────

    #[test]
    fn multiple_patterns_can_fire_simultaneously() {
        let mut events = vec![
            routing_event("music"),
            routing_event("music"),
            routing_event("music"),
            handoff_rejected_event("music"),
            handoff_rejected_event("music"),
        ];
        events.push(agent_turn_with_flags(vec!["hallucination"], 800));
        events.push(agent_turn_event("music", 100));

        let mut bm = BudgetManager::new(1_000);
        bm.record_usage(800);

        let alerts = Watchdog::check(&events, &bm);
        let patterns: Vec<_> = alerts.iter().map(|a| &a.pattern).collect();

        assert!(patterns.contains(&&WatchdogPattern::StuckLoop));
        assert!(patterns.contains(&&WatchdogPattern::HandoffFailure));
        assert!(patterns.contains(&&WatchdogPattern::TokenWaste));
        assert!(patterns.contains(&&WatchdogPattern::BudgetBlindness));
    }

    #[test]
    fn empty_events_produces_no_alerts() {
        let events: Vec<Event> = vec![];
        let bm = BudgetManager::new(10_000);
        let alerts = Watchdog::check(&events, &bm);
        assert!(alerts.is_empty());
    }

    #[test]
    fn watchdog_pattern_display() {
        assert_eq!(WatchdogPattern::StuckLoop.to_string(), "stuck_loop");
        assert_eq!(WatchdogPattern::TokenWaste.to_string(), "token_waste");
        assert_eq!(WatchdogPattern::HandoffFailure.to_string(), "handoff_failure");
        assert_eq!(WatchdogPattern::BudgetBlindness.to_string(), "budget_blindness");
        assert_eq!(WatchdogPattern::SpecRepetition.to_string(), "spec_repetition");
    }
}
