//! `nstn-packs` — the Nanosistant pack system.
//!
//! Packs are directories or archives that extend Nanosistant with
//! deterministic domain-specific and operator-specific functions without
//! requiring Rust compilation or LLM calls.
//!
//! ## Quick overview
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`manifest`] | `pack.toml` schema — identity, tier, routing hints |
//! | [`rules`]    | `rules.toml` schema — TOML-interpreted deterministic rules |
//! | [`evaluator`] | Zero-LLM rule executor (keyword match + arithmetic) |
//! | [`loader`]   | Pack discovery and loading from directories |
//! | [`registry`] | In-memory catalogue with persistence and usage tracking |

pub mod manifest;
pub mod rules;
pub mod evaluator;
pub mod loader;
pub mod registry;

// ── Top-level re-exports ─────────────────────────────────────────────────────

// Manifest
pub use manifest::{PackManifest, PackMeta, PackTier, RoutingMeta};

// Rules
pub use rules::{Rule, RuleExample, RuleFormula, RuleMeta, RuleSet};

// Evaluator
pub use evaluator::RuleEvaluator;

// Loader
pub use loader::{LoadedPack, PackError, PackEvalResult, PackLoader, RoutingMethod};

// Registry
pub use registry::{PackRegistry, RegistryEntry, RegistryStats};
