//! Model router — selects the optimal model tier for a given task.
//!
//! Ports the ruflo upstream TypeScript model-routing concept to Rust.
//! Routing is fully deterministic — no LLM inference.
//!
//! # Selection logic
//!
//! | Condition | Tier |
//! |---|---|
//! | `router_confidence ≥ 0.95` **and** `complexity < 0.30` | Fast (haiku) |
//! | `router_confidence ≥ 0.70` **and** `complexity < 0.65` | Balanced (sonnet) |
//! | `complexity ≥ 0.65` **or** `domain == "framework"` | Powerful (opus) |
//! | (default) | Balanced (sonnet) |
//!
//! The `model` field from the agent TOML is treated as a **floor** — the
//! router never downgrades below the configured model.

use serde::{Deserialize, Serialize};

// ─── Model constants ──────────────────────────────────────────────────────────

const MODEL_HAIKU: &str = "claude-haiku-4-20250514";
const MODEL_SONNET: &str = "claude-sonnet-4-20250514";
const MODEL_OPUS: &str = "claude-opus-4-20250514";

// ─── ModelTier ────────────────────────────────────────────────────────────────

/// Available model tiers, ordered by capability.
///
/// The numeric discriminant order matches ascending capability and cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ModelTier {
    /// Fast, cheap — simple queries, deterministic fallbacks.
    ///
    /// Maps to `claude-haiku`.
    Fast,
    /// Balanced — most tasks.
    ///
    /// Maps to `claude-sonnet`.
    Balanced,
    /// Powerful — deep reasoning, complex analysis.
    ///
    /// Maps to `claude-opus`.
    Powerful,
}

impl ModelTier {
    /// Human-readable label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Powerful => "powerful",
        }
    }

    /// Parse a tier from a model ID string.
    ///
    /// Returns the lowest tier that is at least as capable as the model.
    #[must_use]
    pub fn from_model_id(model_id: &str) -> Self {
        let lower = model_id.to_lowercase();
        if lower.contains("opus") {
            Self::Powerful
        } else if lower.contains("sonnet") {
            Self::Balanced
        } else {
            // haiku, or unknown → default to fast (least restrictive floor)
            Self::Fast
        }
    }
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── ModelProfile ─────────────────────────────────────────────────────────────

/// Characteristics of a specific model configuration.
#[derive(Debug, Clone)]
pub struct ModelProfile {
    pub tier: ModelTier,
    pub model_id: String,
    /// Relative cost compared to Powerful (Powerful = 1.0).
    pub cost_multiplier: f64,
    /// Relative speed compared to Powerful (Powerful = 1.0, higher = faster).
    pub speed_multiplier: f64,
    /// Maximum complexity score this tier should handle (0.0–1.0).
    pub max_complexity: f64,
}

impl ModelProfile {
    fn fast() -> Self {
        Self {
            tier: ModelTier::Fast,
            model_id: MODEL_HAIKU.to_string(),
            cost_multiplier: 0.04,
            speed_multiplier: 3.0,
            max_complexity: 0.30,
        }
    }

    fn balanced() -> Self {
        Self {
            tier: ModelTier::Balanced,
            model_id: MODEL_SONNET.to_string(),
            cost_multiplier: 0.20,
            speed_multiplier: 1.5,
            max_complexity: 0.65,
        }
    }

    fn powerful() -> Self {
        Self {
            tier: ModelTier::Powerful,
            model_id: MODEL_OPUS.to_string(),
            cost_multiplier: 1.0,
            speed_multiplier: 1.0,
            max_complexity: 1.0,
        }
    }
}

// ─── ComplexityIndicators ─────────────────────────────────────────────────────

/// Keyword indicator lists for complexity scoring.
#[derive(Debug, Clone)]
pub struct ComplexityIndicators {
    /// High-complexity keywords — push score toward 1.0.
    pub high: Vec<String>,
    /// Medium-complexity keywords — push score toward 0.5.
    pub medium: Vec<String>,
    /// Low-complexity keywords — push score toward 0.0.
    pub low: Vec<String>,
}

impl Default for ComplexityIndicators {
    fn default() -> Self {
        Self {
            high: vec![
                "architect".to_string(),
                "design".to_string(),
                "security".to_string(),
                "audit".to_string(),
                "complex".to_string(),
                "algorithm".to_string(),
                "refactor".to_string(),
                "optimize".to_string(),
                "analyze".to_string(),
                "analysis".to_string(),
                "reasoning".to_string(),
                "strategy".to_string(),
                "framework".to_string(),
                "cryptography".to_string(),
                "distributed".to_string(),
                "concurrency".to_string(),
                "performance".to_string(),
                "tradeoff".to_string(),
                "scalability".to_string(),
            ],
            medium: vec![
                "implement".to_string(),
                "feature".to_string(),
                "fix".to_string(),
                "test".to_string(),
                "review".to_string(),
                "write".to_string(),
                "create".to_string(),
                "build".to_string(),
                "debug".to_string(),
                "update".to_string(),
                "add".to_string(),
                "change".to_string(),
                "improve".to_string(),
                "migrate".to_string(),
                "integrate".to_string(),
            ],
            low: vec![
                "list".to_string(),
                "show".to_string(),
                "what is".to_string(),
                "help".to_string(),
                "hello".to_string(),
                "hi".to_string(),
                "thanks".to_string(),
                "simple".to_string(),
                "quick".to_string(),
                "example".to_string(),
                "sample".to_string(),
                "explain".to_string(),
                "describe".to_string(),
            ],
        }
    }
}

// ─── ModelSelection ───────────────────────────────────────────────────────────

/// The outcome of a model routing decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    pub tier: ModelTier,
    pub model_id: String,
    pub complexity_score: f64,
    pub reason: String,
}

// ─── ModelRouter ──────────────────────────────────────────────────────────────

/// Routes tasks to the optimal model tier based on complexity and confidence.
///
/// Build once with [`ModelRouter::new`], then call [`ModelRouter::select_model`]
/// per request.  Use [`ModelRouter::record_outcome`] to accumulate outcome data
/// (used for future adaptive tuning).
pub struct ModelRouter {
    profiles: Vec<ModelProfile>,
    complexity_indicators: ComplexityIndicators,
    /// Cumulative outcome records for adaptive learning.
    outcomes: Vec<RoutingOutcomeRecord>,
}

/// Internal record of a routing outcome for learning purposes.
#[derive(Debug, Clone)]
struct RoutingOutcomeRecord {
    selection: ModelSelection,
    success: bool,
    latency_ms: u32,
    tokens_used: u32,
}

impl ModelRouter {
    /// Construct a `ModelRouter` with default profiles and complexity indicators.
    #[must_use]
    pub fn new() -> Self {
        Self {
            profiles: vec![
                ModelProfile::fast(),
                ModelProfile::balanced(),
                ModelProfile::powerful(),
            ],
            complexity_indicators: ComplexityIndicators::default(),
            outcomes: Vec::new(),
        }
    }

    /// Score a message's complexity on a scale of 0.0 (trivial) to 1.0 (maximum).
    ///
    /// Uses keyword matching against the high/medium/low indicator lists.
    /// Scores are normalized so that the overall score reflects the dominant
    /// signal rather than raw match count.
    #[must_use]
    pub fn score_complexity(&self, message: &str) -> f64 {
        let lower = message.to_lowercase();

        // Count matches in each category, weighted by signal strength.
        let mut high_hits = 0u32;
        let mut medium_hits = 0u32;
        let mut low_hits = 0u32;

        for kw in &self.complexity_indicators.high {
            if lower.contains(kw.as_str()) {
                high_hits += 1;
            }
        }
        for kw in &self.complexity_indicators.medium {
            if lower.contains(kw.as_str()) {
                medium_hits += 1;
            }
        }
        for kw in &self.complexity_indicators.low {
            if lower.contains(kw.as_str()) {
                low_hits += 1;
            }
        }

        let total = high_hits + medium_hits + low_hits;
        if total == 0 {
            // No indicators: assume medium complexity as a safe default.
            return 0.5;
        }

        // Weighted average: high=1.0, medium=0.5, low=0.1
        #[allow(clippy::cast_precision_loss)]
        let weighted_sum = f64::from(high_hits) * 1.0
            + f64::from(medium_hits) * 0.5
            + f64::from(low_hits) * 0.1;
        #[allow(clippy::cast_precision_loss)]
        let max_possible = f64::from(total) * 1.0;

        (weighted_sum / max_possible).clamp(0.0, 1.0)
    }

    /// Select the optimal model tier for a message.
    ///
    /// # Parameters
    /// - `message`           — the raw user message.
    /// - `router_confidence` — the confidence score from the upstream
    ///   [`ConfidenceLadderRouter`] (0.0–1.0).
    /// - `domain`            — the domain routed to.
    ///
    /// The `config_model_floor` parameter is applied separately by
    /// [`Self::apply_floor`]; pass the config model string to
    /// [`Self::select_model_with_floor`] to enforce it in one call.
    #[must_use]
    pub fn select_model(&self, message: &str, router_confidence: f64, domain: &str) -> ModelSelection {
        self.select_model_with_floor(message, router_confidence, domain, "")
    }

    /// Select the optimal model tier, enforcing a minimum tier from the
    /// agent config's `model` field.
    ///
    /// If `config_model` is non-empty, the selected tier will never be lower
    /// than the tier implied by that model ID.
    #[must_use]
    pub fn select_model_with_floor(
        &self,
        message: &str,
        router_confidence: f64,
        domain: &str,
        config_model: &str,
    ) -> ModelSelection {
        let complexity = self.score_complexity(message);

        let (tier, raw_reason) = self.pick_tier(router_confidence, complexity, domain);

        // Apply the config-model floor: never downgrade below the configured tier.
        let (final_tier, reason) = if !config_model.is_empty() {
            let floor_tier = ModelTier::from_model_id(config_model);
            if floor_tier > tier {
                (
                    floor_tier,
                    format!(
                        "{raw_reason}; elevated to {} by agent config floor ({})",
                        floor_tier.as_str(),
                        config_model
                    ),
                )
            } else {
                (tier, raw_reason)
            }
        } else {
            (tier, raw_reason)
        };

        let model_id = self.model_id_for_tier(final_tier);

        ModelSelection {
            tier: final_tier,
            model_id,
            complexity_score: complexity,
            reason,
        }
    }

    /// Record the outcome of a routing decision for future learning.
    ///
    /// In v0.1.0 this accumulates data in memory; future versions can use
    /// this to adapt thresholds or weight maps.
    pub fn record_outcome(
        &mut self,
        selection: &ModelSelection,
        success: bool,
        latency_ms: u32,
        tokens_used: u32,
    ) {
        self.outcomes.push(RoutingOutcomeRecord {
            selection: selection.clone(),
            success,
            latency_ms,
            tokens_used,
        });
    }

    /// Number of recorded outcomes.
    #[must_use]
    pub fn outcome_count(&self) -> usize {
        self.outcomes.len()
    }

    /// Success rate across all recorded outcomes (0.0–1.0).
    ///
    /// Returns `None` if no outcomes have been recorded.
    #[must_use]
    pub fn success_rate(&self) -> Option<f64> {
        if self.outcomes.is_empty() {
            return None;
        }
        #[allow(clippy::cast_precision_loss)]
        let rate = self.outcomes.iter().filter(|o| o.success).count() as f64
            / self.outcomes.len() as f64;
        Some(rate)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Core tier selection logic.
    fn pick_tier(&self, router_confidence: f64, complexity: f64, domain: &str) -> (ModelTier, String) {
        // Fast path: very high confidence + trivial complexity
        if router_confidence >= 0.95 && complexity < 0.30 {
            return (
                ModelTier::Fast,
                format!(
                    "high confidence ({router_confidence:.2}) and low complexity ({complexity:.2})"
                ),
            );
        }

        // Powerful: complex tasks or special domains
        if complexity >= 0.65 || domain == "framework" {
            return (
                ModelTier::Powerful,
                if domain == "framework" {
                    format!("framework domain requires powerful model (complexity={complexity:.2})")
                } else {
                    format!("high complexity score ({complexity:.2})")
                },
            );
        }

        // Balanced: moderate confidence + non-complex
        if router_confidence >= 0.70 && complexity < 0.65 {
            return (
                ModelTier::Balanced,
                format!(
                    "moderate confidence ({router_confidence:.2}) and medium complexity ({complexity:.2})"
                ),
            );
        }

        // Default
        (
            ModelTier::Balanced,
            format!("default balanced selection (confidence={router_confidence:.2}, complexity={complexity:.2})"),
        )
    }

    /// Return the canonical model ID for a tier.
    fn model_id_for_tier(&self, tier: ModelTier) -> String {
        self.profiles
            .iter()
            .find(|p| p.tier == tier)
            .map_or_else(|| MODEL_SONNET.to_string(), |p| p.model_id.clone())
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn router() -> ModelRouter {
        ModelRouter::new()
    }

    // ── Complexity scoring ────────────────────────────────────────────────────

    #[test]
    fn complexity_score_is_low_for_simple_messages() {
        let r = router();
        let score = r.score_complexity("hello, help me list the files");
        // "hello", "help", "list" are all low-complexity indicators
        assert!(score < 0.4, "expected low score, got {score}");
    }

    #[test]
    fn complexity_score_is_high_for_complex_messages() {
        let r = router();
        let score = r.score_complexity("architect a security audit algorithm for distributed concurrency");
        assert!(score > 0.7, "expected high score, got {score}");
    }

    #[test]
    fn complexity_score_is_medium_for_mixed_messages() {
        let r = router();
        let score = r.score_complexity("implement a feature and fix the test");
        assert!(
            (0.2..0.8).contains(&score),
            "expected medium score, got {score}"
        );
    }

    #[test]
    fn complexity_score_defaults_to_half_when_no_keywords() {
        let r = router();
        let score = r.score_complexity("zxqwerty random gobbledygook");
        assert!(
            (score - 0.5).abs() < f64::EPSILON,
            "expected 0.5 for no-match, got {score}"
        );
    }

    #[test]
    fn complexity_score_is_clamped_to_0_1() {
        let r = router();
        // Pile on high-complexity keywords
        let msg = "architect design security audit complex algorithm optimize analyze reasoning strategy";
        let score = r.score_complexity(msg);
        assert!(score <= 1.0);
        assert!(score >= 0.0);
    }

    // ── Tier selection ────────────────────────────────────────────────────────

    #[test]
    fn selects_fast_for_high_confidence_low_complexity() {
        let r = router();
        let sel = r.select_model("hello list files", 0.97, "general");
        assert_eq!(sel.tier, ModelTier::Fast);
        assert!(sel.model_id.contains("haiku"));
    }

    #[test]
    fn selects_powerful_for_high_complexity() {
        let r = router();
        let sel = r.select_model(
            "architect a distributed security algorithm with complex concurrency analysis",
            0.8,
            "development",
        );
        assert_eq!(sel.tier, ModelTier::Powerful);
        assert!(sel.model_id.contains("opus"));
    }

    #[test]
    fn selects_powerful_for_framework_domain() {
        let r = router();
        // Even with low complexity, framework domain → Powerful
        let sel = r.select_model("show me examples", 0.99, "framework");
        assert_eq!(sel.tier, ModelTier::Powerful);
    }

    #[test]
    fn selects_balanced_for_moderate_confidence() {
        let r = router();
        let sel = r.select_model("fix this bug in my code", 0.75, "development");
        assert_eq!(sel.tier, ModelTier::Balanced);
        assert!(sel.model_id.contains("sonnet"));
    }

    #[test]
    fn default_is_balanced() {
        let r = router();
        // Low confidence but medium complexity — should hit the default
        let sel = r.select_model("implement something", 0.4, "general");
        assert_eq!(sel.tier, ModelTier::Balanced);
    }

    // ── Config model floor ────────────────────────────────────────────────────

    #[test]
    fn floor_enforces_opus_when_config_says_opus() {
        let r = router();
        // Selection without floor would be Fast
        let sel = r.select_model_with_floor(
            "hello list files",
            0.99,
            "general",
            "claude-opus-4-20250514",
        );
        assert_eq!(sel.tier, ModelTier::Powerful);
        assert!(sel.reason.contains("floor"));
    }

    #[test]
    fn floor_does_not_downgrade_higher_selection() {
        let r = router();
        // Selection without floor is Powerful; floor is Balanced — no downgrade.
        let sel = r.select_model_with_floor(
            "architect security audit algorithm",
            0.8,
            "development",
            "claude-sonnet-4-20250514",
        );
        assert_eq!(sel.tier, ModelTier::Powerful);
        assert!(!sel.reason.contains("floor"));
    }

    #[test]
    fn empty_config_model_applies_no_floor() {
        let r = router();
        let sel = r.select_model_with_floor("hello list files", 0.99, "general", "");
        assert_eq!(sel.tier, ModelTier::Fast);
    }

    // ── Outcome recording ─────────────────────────────────────────────────────

    #[test]
    fn record_outcome_increments_count() {
        let mut r = router();
        assert_eq!(r.outcome_count(), 0);

        let sel = r.select_model("hello", 0.99, "general");
        r.record_outcome(&sel, true, 120, 500);
        assert_eq!(r.outcome_count(), 1);
    }

    #[test]
    fn success_rate_returns_none_when_empty() {
        let r = router();
        assert!(r.success_rate().is_none());
    }

    #[test]
    fn success_rate_computes_correctly() {
        let mut r = router();
        let sel = r.select_model("hello", 0.99, "general");

        r.record_outcome(&sel.clone(), true, 100, 100);
        r.record_outcome(&sel.clone(), true, 100, 100);
        r.record_outcome(&sel, false, 200, 200);

        let rate = r.success_rate().unwrap();
        assert!((rate - 2.0 / 3.0).abs() < 1e-9);
    }

    // ── ModelTier helpers ─────────────────────────────────────────────────────

    #[test]
    fn model_tier_from_model_id_parses_correctly() {
        assert_eq!(ModelTier::from_model_id("claude-opus-4-20250514"), ModelTier::Powerful);
        assert_eq!(ModelTier::from_model_id("claude-sonnet-4-20250514"), ModelTier::Balanced);
        assert_eq!(ModelTier::from_model_id("claude-haiku-4-20250514"), ModelTier::Fast);
        assert_eq!(ModelTier::from_model_id("unknown-model"), ModelTier::Fast);
    }

    #[test]
    fn model_tier_ordering_is_ascending_capability() {
        assert!(ModelTier::Fast < ModelTier::Balanced);
        assert!(ModelTier::Balanced < ModelTier::Powerful);
    }

    #[test]
    fn model_selection_contains_complexity_score_and_reason() {
        let r = router();
        let sel = r.select_model("implement a feature", 0.8, "development");
        assert!(sel.complexity_score >= 0.0 && sel.complexity_score <= 1.0);
        assert!(!sel.reason.is_empty());
    }
}
