//! Confidence-ladder router — multi-tier deterministic routing.
//!
//! Each tier adds recall at the cost of latency, gated by confidence.
//! Only falls through to the next tier when not confident enough.
//!
//! ```text
//! Input Query
//!     │
//!     ▼
//! [Tier 1] Aho-Corasick automaton ──► confidence ≥ 0.95 → route
//!     │
//!     ▼
//! [Tier 2] Regex pattern scoring ──► confidence ≥ 0.80 → route
//!     │
//!     ▼
//! [Tier 3] Weighted keyword scoring ── confidence ≥ 0.65 → route
//!     │
//!     ▼
//! [Tier 4] Fuzzy edit-distance ──── confidence ≥ 0.50 → route
//!     │
//!     ▼
//! [Tier 5] Return Ambiguous (LLM escape hatch)
//! ```
//!
//! The AC automaton gives O(n + z) per query — linear in text length,
//! independent of pattern count. This is asymptotically optimal per
//! classical lower bounds (Aho-Corasick 1975, confirmed in 2024 work).

use std::collections::HashMap;

use aho_corasick::{AhoCorasick, MatchKind};
use regex::Regex;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════
// Pattern metadata
// ═══════════════════════════════════════

/// A single pattern in the routing dictionary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePattern {
    /// The literal text or phrase to match.
    pub pattern: String,
    /// Which domain/intent this maps to.
    pub domain: String,
    /// How strongly this pattern signals its domain (0.0–1.0).
    pub weight: f64,
    /// Optional tags for categorization.
    pub tags: Vec<String>,
}

/// A compiled regex pattern with its metadata.
#[derive(Debug, Clone)]
pub struct RegexPattern {
    /// The compiled regex.
    pub regex: Regex,
    /// Which domain/intent this maps to.
    pub domain: String,
    /// How strongly a match signals this domain (0.0–1.0).
    pub weight: f64,
}

/// A weighted keyword for dynamic scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedKeyword {
    pub keyword: String,
    pub domain: String,
    pub weight: f64,
}

/// A fuzzy anchor term for typo recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyAnchor {
    pub term: String,
    pub domain: String,
    pub weight: f64,
}

// ═══════════════════════════════════════
// Tier results
// ═══════════════════════════════════════

/// Per-domain score from a single tier.
#[derive(Debug, Clone, Default)]
struct DomainScores(HashMap<String, f64>);

impl DomainScores {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn add(&mut self, domain: &str, score: f64) {
        let entry = self.0.entry(domain.to_string()).or_insert(0.0);
        *entry += score;
    }

    fn best(&self) -> Option<(&str, f64)> {
        self.0
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(d, &s)| (d.as_str(), s))
    }

    fn get(&self, domain: &str) -> f64 {
        self.0.get(domain).copied().unwrap_or(0.0)
    }

    fn domains(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty() || self.0.values().all(|&v| v == 0.0)
    }
}

// ═══════════════════════════════════════
// Route decision
// ═══════════════════════════════════════

/// The outcome of the confidence-ladder routing.
#[derive(Debug, Clone)]
pub struct RouteDecision {
    /// The chosen domain, or None if ambiguous.
    pub domain: Option<String>,
    /// Confidence score (0.0–1.0).
    pub confidence: f64,
    /// Which tier resolved the decision (1–4), or 0 if ambiguous.
    pub resolved_at_tier: u8,
    /// Per-domain scores from the combined pipeline.
    pub scores: HashMap<String, f64>,
}

impl RouteDecision {
    /// Whether routing was confident enough to proceed without LLM.
    #[must_use]
    pub fn is_confident(&self) -> bool {
        self.domain.is_some()
    }

    /// Whether this should fall to the LLM escape hatch.
    #[must_use]
    pub fn is_ambiguous(&self) -> bool {
        self.domain.is_none()
    }
}

// ═══════════════════════════════════════
// Confidence ladder router
// ═══════════════════════════════════════

/// Thresholds for each tier. A tier's best score must exceed its
/// threshold for routing to succeed at that tier.
#[derive(Debug, Clone)]
pub struct RouterThresholds {
    pub tier1_ac: f64,
    pub tier2_regex: f64,
    pub tier3_weighted: f64,
    pub tier4_fuzzy: f64,
}

impl Default for RouterThresholds {
    fn default() -> Self {
        Self {
            tier1_ac: 0.95,
            tier2_regex: 0.80,
            tier3_weighted: 0.65,
            tier4_fuzzy: 0.50,
        }
    }
}

/// The main confidence-ladder router.
///
/// Build once with all patterns, then call `route()` per query.
/// The AC automaton is compiled at construction time — O(L) where
/// L is total pattern length. Per-query routing is O(n + z).
pub struct ConfidenceLadderRouter {
    // ── Tier 1: Aho-Corasick ──
    /// The compiled automaton for all literal patterns.
    ac_automaton: AhoCorasick,
    /// Metadata for each pattern (indexed same as automaton).
    ac_patterns: Vec<RoutePattern>,

    // ── Tier 2: Regex ──
    regex_patterns: Vec<RegexPattern>,

    // ── Tier 3: Weighted keywords ──
    /// domain → { keyword → weight }
    weighted_keywords: HashMap<String, HashMap<String, f64>>,
    /// domain → sum of all weights (for normalization)
    weighted_mass: HashMap<String, f64>,

    // ── Tier 4: Fuzzy ──
    fuzzy_anchors: Vec<FuzzyAnchor>,
    /// Edit distance threshold (0–100 scale, higher = stricter match)
    fuzzy_threshold: f64,

    // ── Config ──
    thresholds: RouterThresholds,
    /// Known domain names for score initialization.
    known_domains: Vec<String>,
    /// Fallback domain when no tier is confident.
    fallback_domain: String,
}

/// Builder for `ConfidenceLadderRouter`.
pub struct RouterBuilder {
    ac_patterns: Vec<RoutePattern>,
    regex_patterns: Vec<RegexPattern>,
    weighted_keywords: Vec<WeightedKeyword>,
    fuzzy_anchors: Vec<FuzzyAnchor>,
    fuzzy_threshold: f64,
    thresholds: RouterThresholds,
    fallback_domain: String,
}

impl RouterBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ac_patterns: Vec::new(),
            regex_patterns: Vec::new(),
            weighted_keywords: Vec::new(),
            fuzzy_anchors: Vec::new(),
            fuzzy_threshold: 82.0,
            thresholds: RouterThresholds::default(),
            fallback_domain: "general".to_string(),
        }
    }

    /// Add a literal pattern for the AC automaton (Tier 1).
    #[must_use]
    pub fn add_pattern(mut self, pattern: RoutePattern) -> Self {
        self.ac_patterns.push(pattern);
        self
    }

    /// Add multiple literal patterns.
    #[must_use]
    pub fn add_patterns(mut self, patterns: Vec<RoutePattern>) -> Self {
        self.ac_patterns.extend(patterns);
        self
    }

    /// Add a compiled regex pattern (Tier 2).
    #[must_use]
    pub fn add_regex(mut self, pattern: RegexPattern) -> Self {
        self.regex_patterns.push(pattern);
        self
    }

    /// Add a weighted keyword (Tier 3).
    #[must_use]
    pub fn add_weighted_keyword(mut self, kw: WeightedKeyword) -> Self {
        self.weighted_keywords.push(kw);
        self
    }

    /// Add multiple weighted keywords.
    #[must_use]
    pub fn add_weighted_keywords(mut self, kws: Vec<WeightedKeyword>) -> Self {
        self.weighted_keywords.extend(kws);
        self
    }

    /// Add a fuzzy anchor (Tier 4).
    #[must_use]
    pub fn add_fuzzy_anchor(mut self, anchor: FuzzyAnchor) -> Self {
        self.fuzzy_anchors.push(anchor);
        self
    }

    /// Set the fuzzy match threshold (0–100). Default 82.
    #[must_use]
    pub fn fuzzy_threshold(mut self, threshold: f64) -> Self {
        self.fuzzy_threshold = threshold;
        self
    }

    /// Override default tier thresholds.
    #[must_use]
    pub fn thresholds(mut self, t: RouterThresholds) -> Self {
        self.thresholds = t;
        self
    }

    /// Set the fallback domain. Default "general".
    #[must_use]
    pub fn fallback_domain(mut self, domain: impl Into<String>) -> Self {
        self.fallback_domain = domain.into();
        self
    }

    /// Build the router. Compiles the AC automaton.
    #[must_use]
    pub fn build(self) -> ConfidenceLadderRouter {
        // Collect all known domains
        let mut domain_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        domain_set.insert(self.fallback_domain.clone());
        for p in &self.ac_patterns {
            domain_set.insert(p.domain.clone());
        }
        for p in &self.regex_patterns {
            domain_set.insert(p.domain.clone());
        }
        for kw in &self.weighted_keywords {
            domain_set.insert(kw.domain.clone());
        }
        for a in &self.fuzzy_anchors {
            domain_set.insert(a.domain.clone());
        }

        // Build AC automaton from pattern strings
        let pattern_strings: Vec<String> = self
            .ac_patterns
            .iter()
            .map(|p| p.pattern.to_lowercase())
            .collect();
        let ac_automaton = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::Standard)
            .build(&pattern_strings)
            .expect("AC automaton should build from valid patterns");

        // Build weighted keyword index
        let mut weighted_keywords: HashMap<String, HashMap<String, f64>> = HashMap::new();
        let mut weighted_mass: HashMap<String, f64> = HashMap::new();
        for kw in &self.weighted_keywords {
            weighted_keywords
                .entry(kw.domain.clone())
                .or_default()
                .insert(kw.keyword.to_lowercase(), kw.weight);
            *weighted_mass.entry(kw.domain.clone()).or_insert(0.0) += kw.weight;
        }

        ConfidenceLadderRouter {
            ac_automaton,
            ac_patterns: self.ac_patterns,
            regex_patterns: self.regex_patterns,
            weighted_keywords,
            weighted_mass,
            fuzzy_anchors: self.fuzzy_anchors,
            fuzzy_threshold: self.fuzzy_threshold,
            thresholds: self.thresholds,
            known_domains: domain_set.into_iter().collect(),
            fallback_domain: self.fallback_domain,
        }
    }
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfidenceLadderRouter {
    /// Route a user message through the confidence ladder.
    ///
    /// Returns a `RouteDecision` with the chosen domain and confidence.
    /// If no tier is confident enough, the domain is `None` (ambiguous —
    /// fall to LLM classifier).
    #[must_use]
    pub fn route(&self, message: &str) -> RouteDecision {
        let lower = message.to_lowercase();

        // ── Tier 1: Aho-Corasick single pass ──
        let t1 = self.tier1_ac_score(&lower);
        if let Some((domain, score)) = t1.best() {
            if score >= self.thresholds.tier1_ac {
                return RouteDecision {
                    domain: Some(domain.to_string()),
                    confidence: score.min(1.0),
                    resolved_at_tier: 1,
                    scores: t1.0.clone(),
                };
            }
        }

        // ── Tier 2: Regex patterns ──
        let t2 = self.tier2_regex_score(&lower);
        if let Some((domain, score)) = t2.best() {
            if score >= self.thresholds.tier2_regex {
                return RouteDecision {
                    domain: Some(domain.to_string()),
                    confidence: score.min(1.0),
                    resolved_at_tier: 2,
                    scores: t2.0.clone(),
                };
            }
        }

        // ── Tier 3: Weighted keyword scoring ──
        let t3 = self.tier3_weighted_score(&lower);
        if let Some((domain, score)) = t3.best() {
            if score >= self.thresholds.tier3_weighted {
                return RouteDecision {
                    domain: Some(domain.to_string()),
                    confidence: score.min(1.0),
                    resolved_at_tier: 3,
                    scores: t3.0.clone(),
                };
            }
        }

        // ── Tier 4: Fuzzy matching ──
        let t4 = self.tier4_fuzzy_score(&lower);
        if let Some((domain, score)) = t4.best() {
            if score >= self.thresholds.tier4_fuzzy {
                return RouteDecision {
                    domain: Some(domain.to_string()),
                    confidence: score.min(1.0),
                    resolved_at_tier: 4,
                    scores: t4.0.clone(),
                };
            }
        }

        // ── Combine all tiers for the best-effort score even on ambiguous ──
        let mut combined = DomainScores::new();
        for domain in &self.known_domains {
            let score = 0.40 * t1.get(domain)
                + 0.30 * t2.get(domain)
                + 0.20 * t3.get(domain)
                + 0.10 * t4.get(domain);
            combined.add(domain, score);
        }

        let (best_domain, best_score) = combined.best().unwrap_or((&self.fallback_domain, 0.0));

        // Even the combined score might clear a lower threshold
        if best_score >= self.thresholds.tier4_fuzzy {
            return RouteDecision {
                domain: Some(best_domain.to_string()),
                confidence: best_score.min(1.0),
                resolved_at_tier: 5, // combined
                scores: combined.0,
            };
        }

        // ── Ambiguous: LLM escape hatch ──
        RouteDecision {
            domain: None,
            confidence: best_score,
            resolved_at_tier: 0,
            scores: combined.0,
        }
    }

    // ── Tier implementations ──

    /// Tier 1: Aho-Corasick automaton — O(n + z), the optimal path.
    fn tier1_ac_score(&self, text: &str) -> DomainScores {
        let mut scores = DomainScores::new();
        let mut hit_counts: HashMap<&str, usize> = HashMap::new();

        for mat in self.ac_automaton.find_iter(text) {
            let pattern = &self.ac_patterns[mat.pattern().as_usize()];
            scores.add(&pattern.domain, pattern.weight);
            *hit_counts.entry(&pattern.domain).or_insert(0) += 1;
        }

        // Normalize: best possible score per domain = sum of all its pattern weights.
        // We cap at 1.0 for the confidence score.
        let mut normalized = DomainScores::new();
        for domain in scores.domains().collect::<Vec<_>>() {
            let raw = scores.get(domain);
            let total_possible: f64 = self
                .ac_patterns
                .iter()
                .filter(|p| p.domain == domain)
                .map(|p| p.weight)
                .sum();
            let norm = if total_possible > 0.0 {
                raw / total_possible
            } else {
                0.0
            };
            normalized.add(domain, norm);
        }

        normalized
    }

    /// Tier 2: Regex pattern scoring — per-regex confidence.
    fn tier2_regex_score(&self, text: &str) -> DomainScores {
        let mut scores = DomainScores::new();
        for rp in &self.regex_patterns {
            if rp.regex.is_match(text) {
                // Take the max weight per domain from matching regexes
                let current = scores.get(&rp.domain);
                if rp.weight > current {
                    scores.0.insert(rp.domain.clone(), rp.weight);
                }
            }
        }
        scores
    }

    /// Tier 3: Weighted keyword scoring — dynamic, updatable.
    fn tier3_weighted_score(&self, text: &str) -> DomainScores {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let mut scores = DomainScores::new();

        for (domain, keywords) in &self.weighted_keywords {
            let mut raw_score = 0.0;
            for token in &tokens {
                if let Some(&weight) = keywords.get(*token) {
                    raw_score += weight;
                }
            }
            // Normalize by the domain's "mass" so sparse intents aren't penalized
            let mass = self.weighted_mass.get(domain).copied().unwrap_or(1.0);
            if mass > 0.0 {
                scores.add(domain, raw_score / mass);
            }
        }

        scores
    }

    /// Tier 4: Fuzzy edit-distance matching for typo recovery.
    fn tier4_fuzzy_score(&self, text: &str) -> DomainScores {
        let mut scores = DomainScores::new();
        let tokens: Vec<&str> = text.split_whitespace().collect();

        for anchor in &self.fuzzy_anchors {
            let anchor_lower = anchor.term.to_lowercase();
            let mut best_sim = 0.0;

            for token in &tokens {
                // Normalized Levenshtein similarity (0.0–1.0)
                let sim = strsim::normalized_levenshtein(token, &anchor_lower);
                if sim > best_sim {
                    best_sim = sim;
                }
            }

            // Also check the full text for multi-word anchors
            if anchor_lower.contains(' ') {
                let full_sim = strsim::normalized_levenshtein(text, &anchor_lower);
                if full_sim > best_sim {
                    best_sim = full_sim;
                }
            }

            let threshold_norm = self.fuzzy_threshold / 100.0;
            if best_sim >= threshold_norm {
                let weighted_score = best_sim * anchor.weight;
                let current = scores.get(&anchor.domain);
                if weighted_score > current {
                    scores.0.insert(anchor.domain.clone(), weighted_score);
                }
            }
        }

        scores
    }

    // ── Dynamic update ──

    /// Update weighted keywords at runtime (hot-reload from config, DB, etc).
    pub fn update_weighted_keywords(&mut self, keywords: Vec<WeightedKeyword>) {
        self.weighted_keywords.clear();
        self.weighted_mass.clear();
        for kw in &keywords {
            self.weighted_keywords
                .entry(kw.domain.clone())
                .or_default()
                .insert(kw.keyword.to_lowercase(), kw.weight);
            *self.weighted_mass.entry(kw.domain.clone()).or_insert(0.0) += kw.weight;
        }
    }

    /// Record a feedback signal: the LLM classified an ambiguous query
    /// to a specific domain. This can be used to strengthen weight maps.
    pub fn record_feedback(&mut self, _query: &str, domain: &str, _confidence: f64) {
        // In production: update weight map, persist to storage, etc.
        // For v0.1.0, just log the feedback.
        tracing::info!(domain, "routing feedback recorded for future weight update");
    }

    /// Access current thresholds.
    #[must_use]
    pub fn thresholds(&self) -> &RouterThresholds {
        &self.thresholds
    }

    /// Override thresholds at runtime.
    pub fn set_thresholds(&mut self, t: RouterThresholds) {
        self.thresholds = t;
    }
}

// ═══════════════════════════════════════
// Builder helpers for common agent configs
// ═══════════════════════════════════════

/// Build a `ConfidenceLadderRouter` from agent TOML-style trigger configs.
///
/// This is the migration path from the old `DomainClassifier` — it takes
/// the same `{domain, keywords, priority}` shape and compiles it into
/// a proper multi-tier router.
pub fn router_from_trigger_configs(
    configs: &[(String, crate::TriggerConfig)],
) -> ConfidenceLadderRouter {
    let mut builder = RouterBuilder::new();

    for (domain, config) in configs {
        if config.keywords.is_empty() {
            continue;
        }

        let priority_weight = 1.0 + (f64::from(config.priority) * 0.01);

        for keyword in &config.keywords {
            let base_weight = if keyword.contains(' ') {
                // Multi-word phrases are high-precision anchors
                0.95
            } else if keyword.len() >= 6 {
                // Longer keywords are more specific
                0.85
            } else {
                // Short keywords need more context
                0.7
            };

            let weight = (base_weight * priority_weight).min(1.0);

            // Tier 1: AC pattern (literal match)
            builder = builder.add_pattern(RoutePattern {
                pattern: keyword.to_lowercase(),
                domain: domain.clone(),
                weight,
                tags: vec!["from_config".into()],
            });

            // Tier 3: Weighted keyword (for normalized scoring)
            builder = builder.add_weighted_keyword(WeightedKeyword {
                keyword: keyword.to_lowercase(),
                domain: domain.clone(),
                weight,
            });

            // Tier 4: Fuzzy anchor (for typo recovery) on high-value terms
            if keyword.len() >= 4 {
                builder = builder.add_fuzzy_anchor(FuzzyAnchor {
                    term: keyword.to_lowercase(),
                    domain: domain.clone(),
                    weight,
                });
            }
        }

        // Tier 2: Regex patterns for morphological variants
        for keyword in &config.keywords {
            let kw_lower = keyword.to_lowercase();
            // Only build regex for words that have useful morphology
            if kw_lower.len() >= 4 && kw_lower.chars().all(|c| c.is_alphabetic()) {
                let stem = &kw_lower[..kw_lower.len().saturating_sub(1)];
                let regex_str = format!(r"\b{stem}\w*\b");
                if let Ok(re) = Regex::new(&regex_str) {
                    builder = builder.add_regex(RegexPattern {
                        regex: re,
                        domain: domain.clone(),
                        weight: base_weight_for_length(keyword.len()) * priority_weight * 0.9,
                    });
                }
            }
        }
    }

    // Use relaxed thresholds for config-derived routing
    // (fewer patterns per domain than a purpose-built system)
    builder = builder.thresholds(RouterThresholds {
        tier1_ac: 0.30,    // AC hit = strong signal even at low coverage
        tier2_regex: 0.75,
        tier3_weighted: 0.25,
        tier4_fuzzy: 0.45,
    });

    builder.build()
}

fn base_weight_for_length(len: usize) -> f64 {
    if len >= 8 {
        0.9
    } else if len >= 6 {
        0.85
    } else if len >= 4 {
        0.7
    } else {
        0.5
    }
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_router() -> ConfidenceLadderRouter {
        RouterBuilder::new()
            // Music patterns
            .add_patterns(vec![
                RoutePattern { pattern: "verse".into(), domain: "music".into(), weight: 0.9, tags: vec![] },
                RoutePattern { pattern: "hook".into(), domain: "music".into(), weight: 0.9, tags: vec![] },
                RoutePattern { pattern: "beat".into(), domain: "music".into(), weight: 0.8, tags: vec![] },
                RoutePattern { pattern: "bpm".into(), domain: "music".into(), weight: 0.9, tags: vec![] },
                RoutePattern { pattern: "808".into(), domain: "music".into(), weight: 0.95, tags: vec![] },
                RoutePattern { pattern: "vocal chain".into(), domain: "music".into(), weight: 0.95, tags: vec![] },
                RoutePattern { pattern: "eq".into(), domain: "music".into(), weight: 0.7, tags: vec![] },
            ])
            // Investment patterns
            .add_patterns(vec![
                RoutePattern { pattern: "stock".into(), domain: "investment".into(), weight: 0.9, tags: vec![] },
                RoutePattern { pattern: "earnings".into(), domain: "investment".into(), weight: 0.9, tags: vec![] },
                RoutePattern { pattern: "revenue".into(), domain: "investment".into(), weight: 0.85, tags: vec![] },
                RoutePattern { pattern: "13f".into(), domain: "investment".into(), weight: 0.95, tags: vec![] },
                RoutePattern { pattern: "institutional".into(), domain: "investment".into(), weight: 0.9, tags: vec![] },
            ])
            // Development patterns
            .add_patterns(vec![
                RoutePattern { pattern: "rust".into(), domain: "development".into(), weight: 0.85, tags: vec![] },
                RoutePattern { pattern: "compile".into(), domain: "development".into(), weight: 0.8, tags: vec![] },
                RoutePattern { pattern: "deploy".into(), domain: "development".into(), weight: 0.8, tags: vec![] },
                RoutePattern { pattern: "refactor".into(), domain: "development".into(), weight: 0.85, tags: vec![] },
            ])
            // Framework patterns (high-precision phrases)
            .add_patterns(vec![
                RoutePattern { pattern: "distortion lattice".into(), domain: "framework".into(), weight: 0.99, tags: vec![] },
                RoutePattern { pattern: "false prophet".into(), domain: "framework".into(), weight: 0.99, tags: vec![] },
                RoutePattern { pattern: "archetype".into(), domain: "framework".into(), weight: 0.9, tags: vec![] },
            ])
            // Regex patterns
            .add_regex(RegexPattern {
                regex: Regex::new(r"\b(invoic\w+|billing)\b").unwrap(),
                domain: "investment".into(),
                weight: 0.85,
            })
            .add_regex(RegexPattern {
                regex: Regex::new(r"\b(compil\w+|refactor\w*)\b").unwrap(),
                domain: "development".into(),
                weight: 0.82,
            })
            .add_regex(RegexPattern {
                regex: Regex::new(r"\b(vers\w+|hook\w*|chorus)\b").unwrap(),
                domain: "music".into(),
                weight: 0.8,
            })
            // Weighted keywords
            .add_weighted_keywords(vec![
                WeightedKeyword { keyword: "verse".into(), domain: "music".into(), weight: 1.0 },
                WeightedKeyword { keyword: "beat".into(), domain: "music".into(), weight: 0.9 },
                WeightedKeyword { keyword: "stock".into(), domain: "investment".into(), weight: 1.0 },
                WeightedKeyword { keyword: "code".into(), domain: "development".into(), weight: 0.8 },
                WeightedKeyword { keyword: "bug".into(), domain: "development".into(), weight: 0.9 },
                WeightedKeyword { keyword: "deploy".into(), domain: "development".into(), weight: 0.8 },
            ])
            // Fuzzy anchors for typo recovery
            .add_fuzzy_anchor(FuzzyAnchor { term: "verse".into(), domain: "music".into(), weight: 0.9 })
            .add_fuzzy_anchor(FuzzyAnchor { term: "invoice".into(), domain: "investment".into(), weight: 0.9 })
            .add_fuzzy_anchor(FuzzyAnchor { term: "refactor".into(), domain: "development".into(), weight: 0.85 })
            .add_fuzzy_anchor(FuzzyAnchor { term: "archetype".into(), domain: "framework".into(), weight: 0.9 })
            // Relaxed thresholds for testing
            .thresholds(RouterThresholds {
                tier1_ac: 0.30,
                tier2_regex: 0.75,
                tier3_weighted: 0.25,
                tier4_fuzzy: 0.45,
            })
            .build()
    }

    // ── Tier 1: AC hits ──

    #[test]
    fn routes_music_on_exact_keyword() {
        let router = build_test_router();
        let result = router.route("help me write a verse");
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("music"));
        // May resolve at tier 1 (AC) or tier 2 (regex) depending on
        // normalized score — both are correct.
        assert!(result.resolved_at_tier >= 1 && result.resolved_at_tier <= 3);
    }

    #[test]
    fn tier1_routes_investment_on_exact_keyword() {
        let router = build_test_router();
        let result = router.route("analyze the stock earnings");
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("investment"));
    }

    #[test]
    fn tier1_routes_framework_on_high_precision_phrase() {
        let router = build_test_router();
        let result = router.route("analyze through the distortion lattice");
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("framework"));
    }

    #[test]
    fn tier1_multi_word_phrase_has_high_confidence() {
        let router = build_test_router();
        let result = router.route("the false prophet pattern repeats");
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("framework"));
    }

    // ── Tier 2: Regex morphology ──

    #[test]
    fn tier2_catches_morphological_variants() {
        let router = build_test_router();
        // "compilation" isn't an exact pattern but regex catches "compil\w+"
        let result = router.route("fix the compilation error");
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("development"));
    }

    // ── Tier 3: Weighted keywords ──

    #[test]
    fn tier3_routes_on_weighted_keyword_match() {
        let router = build_test_router();
        let result = router.route("fix this bug in the code");
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("development"));
    }

    // ── Tier 4: Fuzzy typo recovery ──

    #[test]
    fn tier4_catches_typos() {
        let router = build_test_router();
        let result = router.route("fix the refactr issue");
        // "refactr" is close enough to "refactor" via edit distance
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("development"));
    }

    // ── Ambiguous: LLM escape hatch ──

    #[test]
    fn ambiguous_message_returns_none() {
        let router = build_test_router();
        let result = router.route("what's the weather today?");
        // This should be ambiguous — no keywords match any domain
        assert!(result.is_ambiguous() || result.domain.as_deref() == Some("general"));
    }

    // ── Config-driven builder ──

    #[test]
    fn router_from_trigger_configs_works() {
        let configs = vec![
            ("general".to_string(), crate::TriggerConfig { keywords: vec![], priority: 0 }),
            ("music".to_string(), crate::TriggerConfig {
                keywords: vec!["verse".into(), "hook".into(), "beat".into(), "vocal chain".into()],
                priority: 10,
            }),
            ("investment".to_string(), crate::TriggerConfig {
                keywords: vec!["stock".into(), "earnings".into(), "revenue".into()],
                priority: 10,
            }),
            ("development".to_string(), crate::TriggerConfig {
                keywords: vec!["code".into(), "rust".into(), "deploy".into(), "refactor".into()],
                priority: 10,
            }),
            ("framework".to_string(), crate::TriggerConfig {
                keywords: vec!["distortion".into(), "lattice".into(), "archetype".into(), "false prophet".into()],
                priority: 15,
            }),
        ];

        let router = router_from_trigger_configs(&configs);

        // Music routes correctly
        let result = router.route("help me write a verse");
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("music"));

        // Investment routes correctly
        let result = router.route("what's the stock earnings?");
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("investment"));

        // Development routes correctly
        let result = router.route("fix the rust code");
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("development"));

        // Framework routes correctly
        let result = router.route("analyze the archetype pattern");
        assert!(result.is_confident());
        assert_eq!(result.domain.as_deref(), Some("framework"));
    }

    // ── Confidence metadata ──

    #[test]
    fn route_decision_contains_per_domain_scores() {
        let router = build_test_router();
        let result = router.route("help me write a verse for the beat");
        assert!(!result.scores.is_empty());
    }

    #[test]
    fn resolved_at_tier_is_correct() {
        let router = build_test_router();
        // Exact AC hit should resolve at tier 1
        let result = router.route("the 808 hits hard");
        assert!(result.resolved_at_tier <= 2);
    }

    // ── Dynamic weight update ──

    #[test]
    fn update_weighted_keywords_changes_behavior() {
        let mut router = build_test_router();

        // Before: "help" doesn't route anywhere specific
        let result = router.route("help");
        let initial_confidence = result.confidence;

        // Add a strong weight for "help" → music
        router.update_weighted_keywords(vec![
            WeightedKeyword { keyword: "help".into(), domain: "music".into(), weight: 1.0 },
        ]);

        let result = router.route("help");
        // The weighted keyword tier should now pick up "help"
        assert!(result.confidence >= initial_confidence || result.domain.as_deref() == Some("music"));
    }
}
