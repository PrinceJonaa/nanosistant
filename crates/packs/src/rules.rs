//! TOML-interpreted rule format for operator-tier packs.
//!
//! Operators define deterministic rules in `rules.toml` inside a pack
//! directory.  Rules fire when query keywords match, and the formula
//! produces a structured response without any LLM involvement.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════
// RuleSet — top-level rules.toml structure
// ═══════════════════════════════════════

/// The complete `rules.toml` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    pub meta: RuleMeta,
    pub rules: Vec<Rule>,
}

/// Metadata about the rule set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMeta {
    /// Rules schema version.
    pub version: String,
    /// Name of the pack these rules belong to.
    pub pack_name: String,
    /// Operator must flip this to `true` before the rule set will be loaded.
    pub approved: bool,
}

// ═══════════════════════════════════════
// Rule
// ═══════════════════════════════════════

/// A single deterministic rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier within this pack, e.g. `"bpm-to-seconds"`.
    pub id: String,
    /// Human-readable description shown in UIs and logs.
    pub description: String,
    /// Option A: keyword matching — the rule fires if *any* keyword appears
    /// in the lowercased query.
    pub trigger_keywords: Vec<String>,
    /// Option C: semantic description for embedding-similarity routing.
    pub semantic_hint: Option<String>,
    /// The deterministic formula/response executed when this rule fires.
    pub formula: RuleFormula,
    /// Confidence score (0.0–1.0) reported when this rule fires.
    pub confidence: f64,
    /// Illustrative input/output pairs — also used as inline tests.
    pub examples: Vec<RuleExample>,
    /// Who proposed this rule: `"dreamer"` or `"operator"`.
    pub proposed_by: String,
    /// Operator must flip to `true` before the rule will fire.
    pub approved: bool,
}

// ═══════════════════════════════════════
// RuleFormula
// ═══════════════════════════════════════

/// The deterministic computation performed when a rule fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleFormula {
    /// Always return the same string.
    Static { response: String },

    /// Arithmetic expression with named variable substitution.
    ///
    /// `expr` uses operators `+`, `-`, `*`, `/`, `^`, and parentheses.
    /// `variables` names the variables that the evaluator should extract from
    /// the query (e.g. `["x"]` → numeric value extracted from the query).
    Arithmetic { expr: String, variables: Vec<String> },

    /// Keyword → value lookup table.
    Lookup { table: HashMap<String, String> },

    /// Weighted keyword score: `sum(weights for matching keywords) / total_weight`.
    WeightedScore { weights: HashMap<String, f64> },

    /// Score-range → label classification.
    ///
    /// Thresholds are (minimum_score, label) pairs sorted ascending.
    /// The first threshold whose score ≤ weighted_score is selected.
    Classification { thresholds: Vec<(f64, String)> },

    /// String template with named slots filled from extracted values.
    ///
    /// Example: `"At {bpm} BPM: {result} seconds per bar"`.
    Template { template: String, slots: Vec<String> },
}

// ═══════════════════════════════════════
// RuleExample
// ═══════════════════════════════════════

/// A single input/output example for a rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleExample {
    /// Sample query that should trigger this rule.
    pub input: String,
    /// Expected output string.
    pub expected_output: String,
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_toml() -> &'static str {
        r#"
[meta]
version = "1.0"
pack_name = "music-theory"
approved = true

[[rules]]
id = "bpm-bar-duration"
description = "Calculate bar duration in seconds from BPM"
trigger_keywords = ["bpm", "bar", "duration"]
semantic_hint = "How long is one bar at a given tempo in seconds"
confidence = 0.95
proposed_by = "dreamer"
approved = true
examples = [
  { input = "how long is a bar at 120 bpm", expected_output = "2.000 seconds per bar" },
  { input = "bar duration 140 bpm", expected_output = "1.714 seconds per bar" },
]

[rules.formula]
Arithmetic = { expr = "60 / x * 4", variables = ["x"] }

[[rules]]
id = "key-lookup"
description = "Look up a musical key"
trigger_keywords = ["key", "major", "minor"]
confidence = 0.80
proposed_by = "operator"
approved = true
examples = []

[rules.formula]
Lookup = { table = { "C major" = "C D E F G A B", "A minor" = "A B C D E F G" } }
"#
    }

    #[test]
    fn parses_ruleset_from_toml() {
        let rs: RuleSet = toml::from_str(sample_toml()).unwrap();
        assert_eq!(rs.meta.pack_name, "music-theory");
        assert!(rs.meta.approved);
        assert_eq!(rs.rules.len(), 2);
    }

    #[test]
    fn arithmetic_formula_parses() {
        let rs: RuleSet = toml::from_str(sample_toml()).unwrap();
        match &rs.rules[0].formula {
            RuleFormula::Arithmetic { expr, variables } => {
                assert_eq!(expr, "60 / x * 4");
                assert_eq!(variables, &["x"]);
            }
            _ => panic!("expected Arithmetic"),
        }
    }

    #[test]
    fn lookup_formula_parses() {
        let rs: RuleSet = toml::from_str(sample_toml()).unwrap();
        match &rs.rules[1].formula {
            RuleFormula::Lookup { table } => {
                assert_eq!(table.get("C major").map(String::as_str), Some("C D E F G A B"));
            }
            _ => panic!("expected Lookup"),
        }
    }

    #[test]
    fn rule_examples_parse() {
        let rs: RuleSet = toml::from_str(sample_toml()).unwrap();
        assert_eq!(rs.rules[0].examples.len(), 2);
        assert_eq!(rs.rules[0].examples[0].expected_output, "2.000 seconds per bar");
    }

    #[test]
    fn ruleset_roundtrips_via_json() {
        let rs: RuleSet = toml::from_str(sample_toml()).unwrap();
        let json = serde_json::to_string(&rs).unwrap();
        let back: RuleSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.rules.len(), rs.rules.len());
    }

    #[test]
    fn unapproved_rule_can_be_parsed() {
        let toml_str = r#"
[meta]
version = "1.0"
pack_name = "test"
approved = false

[[rules]]
id = "pending"
description = "Not approved yet"
trigger_keywords = ["test"]
confidence = 0.5
proposed_by = "dreamer"
approved = false
examples = []

[rules.formula]
Static = { response = "pending" }
"#;
        let rs: RuleSet = toml::from_str(toml_str).unwrap();
        assert!(!rs.meta.approved);
        assert!(!rs.rules[0].approved);
    }
}
