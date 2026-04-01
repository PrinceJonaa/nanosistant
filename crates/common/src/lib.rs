pub mod deterministic;
pub mod domain;
pub mod events;
pub mod handoff;

/// Generated protobuf types from nanosistant.proto
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/nanosistant.rs"));
}

pub use deterministic::*;
pub use domain::{Domain, DomainClassifier, TriggerConfig};
pub use events::{Event, EventLog, EventType};
pub use handoff::{HandoffError, HandoffValidator};
