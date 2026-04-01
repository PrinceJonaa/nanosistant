//! Event system — structured logging for all system activity.
//!
//! Every operation (deterministic, LLM, routing, handoff) produces an Event.
//! Events feed into the watchdog, budget manager, and session analytics.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of event in the system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// Orchestrator classified the domain for a message.
    RoutingClassified,
    /// A deterministic function was executed (zero tokens).
    DeterministicExecuted,
    /// An LLM agent completed a turn.
    AgentTurnComplete,
    /// An agent handoff was initiated.
    HandoffInitiated,
    /// A handoff was validated successfully.
    HandoffValidated,
    /// A handoff validation failed.
    HandoffRejected,
    /// Budget threshold was crossed.
    BudgetThreshold,
    /// Budget was exhausted.
    BudgetExhausted,
    /// Watchdog pattern triggered.
    WatchdogTriggered,
    /// Knowledge query executed.
    KnowledgeQuery,
    /// Session compacted.
    SessionCompacted,
    /// Custom event.
    Custom(String),
}

impl EventType {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::RoutingClassified => "routing.classified",
            Self::DeterministicExecuted => "deterministic.executed",
            Self::AgentTurnComplete => "agent.turn_complete",
            Self::HandoffInitiated => "handoff.initiated",
            Self::HandoffValidated => "handoff.validated",
            Self::HandoffRejected => "handoff.rejected",
            Self::BudgetThreshold => "budget.threshold",
            Self::BudgetExhausted => "budget.exhausted",
            Self::WatchdogTriggered => "watchdog.triggered",
            Self::KnowledgeQuery => "knowledge.query",
            Self::SessionCompacted => "session.compacted",
            Self::Custom(name) => name.as_str(),
        }
    }
}

/// A structured event recording system activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub timestamp: String,
    pub agent_id: String,
    pub session_id: String,
    pub thread_id: String,
    pub event_type: EventType,
    pub domain: String,
    pub payload: std::collections::HashMap<String, String>,
    pub token_cost: u32,
    pub latency_ms: u32,
    pub was_deterministic: bool,
    pub distortion_flags: Vec<String>,
}

impl Event {
    /// Create a new event with the given type and agent.
    #[must_use]
    pub fn new(event_type: EventType, agent_id: &str, session_id: &str, domain: &str) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
            thread_id: String::new(),
            event_type,
            domain: domain.to_string(),
            payload: std::collections::HashMap::new(),
            token_cost: 0,
            latency_ms: 0,
            was_deterministic: false,
            distortion_flags: Vec::new(),
        }
    }

    /// Create a deterministic execution event (zero tokens).
    #[must_use]
    pub fn deterministic(session_id: &str, domain: &str, function_name: &str) -> Self {
        let mut event = Self::new(
            EventType::DeterministicExecuted,
            "orchestrator",
            session_id,
            domain,
        );
        event.was_deterministic = true;
        event.token_cost = 0;
        event
            .payload
            .insert("function".to_string(), function_name.to_string());
        event
    }

    /// Create a routing classification event.
    #[must_use]
    pub fn routing(session_id: &str, domain: &str, message_preview: &str) -> Self {
        let mut event = Self::new(
            EventType::RoutingClassified,
            "orchestrator",
            session_id,
            domain,
        );
        event.was_deterministic = true;
        event.token_cost = 0;
        let preview = if message_preview.len() > 100 {
            format!("{}...", &message_preview[..100])
        } else {
            message_preview.to_string()
        };
        event
            .payload
            .insert("message_preview".to_string(), preview);
        event
    }

    /// Create an agent turn completion event.
    #[must_use]
    pub fn agent_turn(
        agent_id: &str,
        session_id: &str,
        domain: &str,
        token_cost: u32,
        latency_ms: u32,
    ) -> Self {
        let mut event = Self::new(
            EventType::AgentTurnComplete,
            agent_id,
            session_id,
            domain,
        );
        event.token_cost = token_cost;
        event.latency_ms = latency_ms;
        event.was_deterministic = false;
        event
    }

    /// Convert to protobuf Event.
    #[must_use]
    pub fn to_proto(&self) -> crate::proto::Event {
        crate::proto::Event {
            event_id: self.event_id.clone(),
            timestamp: self.timestamp.clone(),
            agent_id: self.agent_id.clone(),
            session_id: self.session_id.clone(),
            thread_id: self.thread_id.clone(),
            event_type: self.event_type.as_str().to_string(),
            domain: self.domain.clone(),
            payload: self.payload.clone(),
            token_cost: self.token_cost,
            latency_ms: self.latency_ms,
            was_deterministic: self.was_deterministic,
            distortion_flags: self.distortion_flags.clone(),
        }
    }

    /// Add a payload entry.
    pub fn with_payload(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.payload.insert(key.into(), value.into());
        self
    }
}

/// In-memory event log with aggregation capabilities.
#[derive(Debug, Clone, Default)]
pub struct EventLog {
    events: Vec<Event>,
}

impl EventLog {
    #[must_use]
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Record an event.
    pub fn record(&mut self, event: Event) {
        self.events.push(event);
    }

    /// Get all events.
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Get events for a specific session.
    #[must_use]
    pub fn session_events(&self, session_id: &str) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.session_id == session_id)
            .collect()
    }

    /// Total tokens used across all events.
    #[must_use]
    pub fn total_tokens(&self) -> u32 {
        self.events.iter().map(|e| e.token_cost).sum()
    }

    /// Count of deterministic vs LLM calls.
    #[must_use]
    pub fn call_breakdown(&self) -> (u32, u32) {
        let det = self.events.iter().filter(|e| e.was_deterministic).count() as u32;
        let llm = self.events.iter().filter(|e| !e.was_deterministic).count() as u32;
        (det, llm)
    }

    /// Get events of a specific type.
    #[must_use]
    pub fn events_of_type(&self, event_type: &EventType) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| &e.event_type == event_type)
            .collect()
    }

    /// Number of events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the log is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_creation_and_proto_conversion() {
        let event = Event::deterministic("session-1", "music", "bpm_to_bar_duration");
        assert_eq!(event.event_type, EventType::DeterministicExecuted);
        assert!(event.was_deterministic);
        assert_eq!(event.token_cost, 0);

        let proto = event.to_proto();
        assert_eq!(proto.event_type, "deterministic.executed");
        assert!(proto.was_deterministic);
    }

    #[test]
    fn event_log_aggregation() {
        let mut log = EventLog::new();

        log.record(Event::deterministic("s1", "music", "scale_degrees"));
        log.record(Event::agent_turn("music", "s1", "music", 500, 200));
        log.record(Event::deterministic("s1", "music", "bpm_to_bar_duration"));

        assert_eq!(log.len(), 3);
        assert_eq!(log.total_tokens(), 500);

        let (det, llm) = log.call_breakdown();
        assert_eq!(det, 2);
        assert_eq!(llm, 1);
    }

    #[test]
    fn event_log_session_filter() {
        let mut log = EventLog::new();
        log.record(Event::deterministic("s1", "music", "fn1"));
        log.record(Event::deterministic("s2", "dev", "fn2"));
        log.record(Event::deterministic("s1", "music", "fn3"));

        assert_eq!(log.session_events("s1").len(), 2);
        assert_eq!(log.session_events("s2").len(), 1);
        assert_eq!(log.session_events("s3").len(), 0);
    }

    #[test]
    fn routing_event_truncates_long_messages() {
        let long_message = "a".repeat(200);
        let event = Event::routing("s1", "general", &long_message);
        let preview = event.payload.get("message_preview").unwrap();
        assert!(preview.len() < 110); // 100 + "..."
    }
}
