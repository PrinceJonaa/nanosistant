// Legacy unified module (kept for backward compat)
pub mod deterministic;

// New structured det/ module system (v0.7)
pub mod det;

// Domain deterministic modules — re-export shims for backward compat
pub mod det_music;
pub mod det_finance;
pub mod det_data;
pub mod det_time;
pub mod det_text;
pub mod det_code;
pub mod det_geo;
pub mod function_proposal;

pub mod domain;
pub mod events;
pub mod handoff;
pub mod router;
pub mod typed_ir;

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
