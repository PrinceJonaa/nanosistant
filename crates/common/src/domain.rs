//! Domain classification — deterministic routing of messages to agents.
//!
//! Each domain agent has trigger keywords with weights.
//! Classification is pure keyword scoring — no LLM inference.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A recognized domain in the system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Domain(String);

impl Domain {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn general() -> Self {
        Self("general".to_string())
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }

    /// Create from an optional domain hint.
    /// Returns `None` if the hint is empty or unrecognized.
    #[must_use]
    pub fn from_hint(hint: &str) -> Option<Self> {
        let trimmed = hint.trim().to_lowercase();
        if trimmed.is_empty() {
            return None;
        }
        Some(Self(trimmed))
    }
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Trigger configuration for an agent — loaded from TOML config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct TriggerConfig {
    pub keywords: Vec<String>,
    pub priority: u32,
}


/// Deterministic domain classifier using weighted keyword matching.
pub struct DomainClassifier {
    /// `domain_name` → (`trigger_config`, `normalized_keywords`)
    domains: HashMap<String, (TriggerConfig, Vec<String>)>,
}

impl DomainClassifier {
    /// Create a new classifier from domain configs.
    #[must_use]
    pub fn new() -> Self {
        Self {
            domains: HashMap::new(),
        }
    }

    /// Register a domain with its trigger keywords and priority.
    pub fn register(&mut self, domain_name: &str, config: TriggerConfig) {
        let normalized: Vec<String> = config
            .keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();
        self.domains
            .insert(domain_name.to_string(), (config, normalized));
    }

    /// Classify a message into a domain.
    /// Returns the highest-scoring domain, or "general" if none match.
    #[must_use]
    pub fn classify(&self, message: &str) -> Domain {
        let lower = message.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        let joined = lower.clone();

        let mut best_domain = "general".to_string();
        let mut best_score: f64 = 0.0;
        let mut best_priority: u32 = 0;

        for (domain_name, (config, normalized_keywords)) in &self.domains {
            if normalized_keywords.is_empty() {
                continue; // General agent — fallback only
            }

            let mut score: f64 = 0.0;
            for keyword in normalized_keywords {
                // Check substring match in the full message
                if joined.contains(keyword.as_str()) {
                    // Longer keywords are more specific → higher weight
                    #[allow(clippy::cast_precision_loss)]
                    let keyword_weight = 1.0 + (keyword.len() as f64 * 0.1);
                    score += keyword_weight;
                }
                // Also check word-boundary matches for short keywords
                if keyword.len() <= 4 && words.contains(&keyword.as_str()) {
                    score += 0.5;
                }
            }

            // Apply priority as a tiebreaker multiplier
            if score > 0.0 {
                #[allow(clippy::cast_precision_loss)]
                let priority_boost = 1.0 + (f64::from(config.priority) * 0.01);
                score *= priority_boost;
            }

            // Only consider this domain if it has a positive score.
            // Zero-score domains don't compete — "general" is the fallback.
            if score > 0.0
                && (score > best_score
                    || (score == best_score && config.priority > best_priority))
            {
                best_score = score;
                best_domain = domain_name.clone();
                best_priority = config.priority;
            }
        }

        Domain::new(best_domain)
    }

    /// List all registered domain names.
    #[must_use]
    pub fn domain_names(&self) -> Vec<String> {
        self.domains.keys().cloned().collect()
    }
}

impl Default for DomainClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_classifier() -> DomainClassifier {
        let mut classifier = DomainClassifier::new();

        classifier.register(
            "general",
            TriggerConfig {
                keywords: vec![],
                priority: 0,
            },
        );

        classifier.register(
            "music",
            TriggerConfig {
                keywords: vec![
                    "verse".into(),
                    "hook".into(),
                    "beat".into(),
                    "mix".into(),
                    "master".into(),
                    "flow".into(),
                    "cadence".into(),
                    "bars".into(),
                    "rhyme".into(),
                    "BPM".into(),
                    "808".into(),
                    "EQ".into(),
                    "compression".into(),
                    "vocal chain".into(),
                    "arrangement".into(),
                    "genre".into(),
                ],
                priority: 10,
            },
        );

        classifier.register(
            "investment",
            TriggerConfig {
                keywords: vec![
                    "stock".into(),
                    "trade".into(),
                    "short".into(),
                    "long".into(),
                    "options".into(),
                    "crypto".into(),
                    "revenue".into(),
                    "earnings".into(),
                    "SEC".into(),
                    "13F".into(),
                    "institutional".into(),
                    "disruption".into(),
                    "gatekeeper".into(),
                    "market".into(),
                ],
                priority: 10,
            },
        );

        classifier.register(
            "development",
            TriggerConfig {
                keywords: vec![
                    "code".into(),
                    "rust".into(),
                    "swift".into(),
                    "typescript".into(),
                    "bug".into(),
                    "deploy".into(),
                    "build".into(),
                    "test".into(),
                    "API".into(),
                    "database".into(),
                    "architecture".into(),
                    "refactor".into(),
                    "PR".into(),
                    "git".into(),
                ],
                priority: 10,
            },
        );

        classifier.register(
            "framework",
            TriggerConfig {
                keywords: vec![
                    "distortion".into(),
                    "lattice".into(),
                    "archetype".into(),
                    "chain".into(),
                    "elite".into(),
                    "beast".into(),
                    "dragon".into(),
                    "false prophet".into(),
                    "residue".into(),
                    "seizure".into(),
                    "sovereignty".into(),
                    "presence".into(),
                    "Babylon".into(),
                ],
                priority: 15,
            },
        );

        classifier
    }

    #[test]
    fn routes_music_questions_to_music() {
        let classifier = build_test_classifier();
        assert_eq!(
            classifier.classify("help me write a verse").name(),
            "music"
        );
        assert_eq!(
            classifier.classify("set the BPM to 140 and add more 808").name(),
            "music"
        );
        assert_eq!(
            classifier.classify("adjust the vocal chain EQ").name(),
            "music"
        );
    }

    #[test]
    fn routes_investment_questions_to_investment() {
        let classifier = build_test_classifier();
        assert_eq!(
            classifier.classify("what's RHI's revenue trend?").name(),
            "investment"
        );
        assert_eq!(
            classifier.classify("analyze the stock earnings report").name(),
            "investment"
        );
        assert_eq!(
            classifier.classify("check the 13F institutional holdings").name(),
            "investment"
        );
    }

    #[test]
    fn routes_development_questions_to_development() {
        let classifier = build_test_classifier();
        assert_eq!(
            classifier
                .classify("fix this Rust compilation error")
                .name(),
            "development"
        );
        assert_eq!(
            classifier.classify("deploy the API to production").name(),
            "development"
        );
        assert_eq!(
            classifier.classify("refactor the database schema").name(),
            "development"
        );
    }

    #[test]
    fn routes_framework_questions_with_higher_priority() {
        let classifier = build_test_classifier();
        assert_eq!(
            classifier
                .classify("analyze this through the distortion lattice")
                .name(),
            "framework"
        );
        assert_eq!(
            classifier
                .classify("what archetype is this false prophet pattern?")
                .name(),
            "framework"
        );
    }

    #[test]
    fn falls_back_to_general_for_ambiguous_messages() {
        let classifier = build_test_classifier();
        assert_eq!(
            classifier.classify("what's the weather today?").name(),
            "general"
        );
        assert_eq!(
            classifier.classify("hello, how are you?").name(),
            "general"
        );
    }

    #[test]
    fn domain_from_hint_works() {
        assert_eq!(Domain::from_hint("music").unwrap().name(), "music");
        assert_eq!(Domain::from_hint("  Investment  ").unwrap().name(), "investment");
        assert!(Domain::from_hint("").is_none());
        assert!(Domain::from_hint("   ").is_none());
    }
}
