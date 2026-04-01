pub mod deterministic;
pub mod domain;
pub mod events;
pub mod handoff;
pub mod router;

/// Generated protobuf + tonic types from nanosistant.proto
#[allow(clippy::all, clippy::pedantic)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/nanosistant.rs"));
}

pub use deterministic::*;
pub use domain::{Domain, DomainClassifier, TriggerConfig};
pub use events::{Event, EventLog, EventType};
pub use handoff::{HandoffError, HandoffValidator};
pub use router::{
    ConfidenceLadderRouter, FuzzyAnchor, RegexPattern, RouteDecision, RoutePattern,
    RouterBuilder, RouterThresholds, WeightedKeyword, router_from_trigger_configs,
};
