//! Nanosistant deterministic module system.
//!
//! Organised into three layers:
//! - `universal/` — domain-agnostic math: logic, graph theory, information theory, probability
//! - `domain/`    — domain-specific pure functions: music, finance, data, time, text, code, geo,
//!                  physics, health, social
//! - `operator/`  — dispatcher stubs populated by the pack system

pub mod universal;
pub mod domain;
pub mod operator;

// Re-export universal modules at top level for ergonomic access
pub use universal::logic;
pub use universal::graph;
pub use universal::information;
pub use universal::probability;

// Re-export domain modules at top level
pub use domain::music;
pub use domain::finance;
pub use domain::data;
pub use domain::time;
pub use domain::text;
pub use domain::code;
pub use domain::geo;
pub use domain::physics;
pub use domain::health;
pub use domain::social;
