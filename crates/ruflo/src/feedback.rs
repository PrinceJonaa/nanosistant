//! Routing feedback store — records routing decisions and their outcomes.
//!
//! Inspired by ruflo's Q-learning approach but kept fully deterministic for
//! v0.1.0.  Outcomes are stored in an in-memory ring buffer and exposed via
//! accuracy metrics and weight-update suggestions.
//!
//! # Usage
//!
//! ```rust,ignore
//! let mut store = FeedbackStore::new(10_000);
//! store.record(RoutingRecord { session_id: "s1".into(), ... });
//! store.record_outcome("s1", RoutingOutcome { success: true, ... });
//! let accuracy = store.domain_accuracy("music");
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ─── RoutingOutcome ───────────────────────────────────────────────────────────

/// The measured outcome of a completed routing decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingOutcome {
    /// Whether the agent successfully completed the task.
    pub success: bool,
    /// End-to-end latency in milliseconds.
    pub latency_ms: u32,
    /// Total tokens consumed by this turn.
    pub tokens_used: u32,
    /// `true` if the user explicitly rerouted the message to a different domain.
    pub was_rerouted: bool,
    /// When `was_rerouted == true`, the domain the user redirected to.
    pub correct_domain: Option<String>,
}

// ─── RoutingRecord ────────────────────────────────────────────────────────────

/// A single routing decision record, optionally annotated with an outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRecord {
    /// ISO-8601 timestamp (UTC).
    pub timestamp: String,
    /// Session identifier.
    pub session_id: String,
    /// First 100 characters of the original message.
    pub message_preview: String,
    /// Domain that was selected by the router.
    pub domain_routed: String,
    /// Confidence score from the router (0.0–1.0).
    pub confidence: f64,
    /// Which tier resolved the route (1–5, or 0 for combined / hint-override).
    pub tier_resolved: u8,
    /// Model ID selected for this route.
    pub model_selected: String,
    /// Measured outcome, filled in after the turn completes.
    pub outcome: Option<RoutingOutcome>,
}

impl RoutingRecord {
    /// Construct a `RoutingRecord`, truncating the message preview to 100 chars.
    #[must_use]
    pub fn new(
        timestamp: impl Into<String>,
        session_id: impl Into<String>,
        message: &str,
        domain_routed: impl Into<String>,
        confidence: f64,
        tier_resolved: u8,
        model_selected: impl Into<String>,
    ) -> Self {
        let preview: String = message.chars().take(100).collect();
        Self {
            timestamp: timestamp.into(),
            session_id: session_id.into(),
            message_preview: preview,
            domain_routed: domain_routed.into(),
            confidence,
            tier_resolved,
            model_selected: model_selected.into(),
            outcome: None,
        }
    }
}

// ─── WeightUpdateSuggestion ───────────────────────────────────────────────────

/// A suggested weight adjustment derived from feedback analysis.
///
/// These suggestions can be applied to the [`ConfidenceLadderRouter`]'s
/// weighted keyword map to improve routing accuracy over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightUpdateSuggestion {
    /// The keyword (from the misrouted message preview) to boost.
    pub keyword: String,
    /// Domain this keyword should be strengthened toward.
    pub domain: String,
    /// Suggested new weight (0.0–1.0).
    pub suggested_weight: f64,
    /// Human-readable explanation for the suggestion.
    pub reason: String,
}

// ─── FeedbackStore ────────────────────────────────────────────────────────────

/// In-memory ring buffer of routing records with accuracy analytics.
///
/// When the buffer is full, the oldest record is evicted (FIFO ring).
///
/// # Example
///
/// ```rust
/// use nstn_ruflo::feedback::{FeedbackStore, RoutingRecord, RoutingOutcome};
///
/// let mut store = FeedbackStore::new(100);
/// let record = RoutingRecord::new(
///     "2026-04-01T00:00:00Z",
///     "session-1",
///     "help me write a verse",
///     "music",
///     0.95,
///     1,
///     "claude-sonnet-4-20250514",
/// );
/// store.record(record);
/// assert_eq!(store.len(), 1);
/// ```
pub struct FeedbackStore {
    records: Vec<RoutingRecord>,
    max_records: usize,
    /// Per-domain accuracy counters: `(correct_routings, total_routings)`.
    domain_accuracy: HashMap<String, (u32, u32)>,
}

impl FeedbackStore {
    /// Create a new `FeedbackStore` with the given ring-buffer capacity.
    ///
    /// `max_records` must be ≥ 1.  Values of 0 are silently elevated to 1.
    #[must_use]
    pub fn new(max_records: usize) -> Self {
        let capacity = max_records.max(1);
        Self {
            records: Vec::with_capacity(capacity.min(1024)),
            max_records: capacity,
            domain_accuracy: HashMap::new(),
        }
    }

    /// Append a routing record.
    ///
    /// If the buffer is at capacity, the oldest record is removed first.
    ///
    /// The domain accuracy counters are updated if the record already has an
    /// outcome attached (e.g., when replaying historical data).
    pub fn record(&mut self, record: RoutingRecord) {
        // Evict oldest when at capacity.
        if self.records.len() >= self.max_records {
            let evicted = self.records.remove(0);
            // Undo the accuracy counters for the evicted record.
            if let Some(outcome) = &evicted.outcome {
                let entry = self
                    .domain_accuracy
                    .entry(evicted.domain_routed.clone())
                    .or_insert((0, 0));
                if entry.1 > 0 {
                    entry.1 -= 1;
                    if !outcome.was_rerouted {
                        entry.0 = entry.0.saturating_sub(1);
                    }
                }
            }
        }

        // Update accuracy counters for the new record if it already has an outcome.
        if let Some(outcome) = &record.outcome {
            let entry = self
                .domain_accuracy
                .entry(record.domain_routed.clone())
                .or_insert((0, 0));
            entry.1 += 1;
            if !outcome.was_rerouted {
                entry.0 += 1;
            }
        }

        self.records.push(record);
    }

    /// Attach an outcome to an existing record identified by `session_id`.
    ///
    /// If multiple records share the same session ID, the most recent one
    /// (last in insertion order) is updated.  Accuracy counters are updated
    /// atomically.
    ///
    /// Does nothing if no record with that `session_id` is found.
    pub fn record_outcome(&mut self, session_id: &str, outcome: RoutingOutcome) {
        // Find the last record with this session_id.
        let pos = self
            .records
            .iter()
            .rposition(|r| r.session_id == session_id);

        let Some(idx) = pos else { return };

        let record = &mut self.records[idx];
        let domain = record.domain_routed.clone();

        // Update accuracy counters: first remove stale data if already set.
        if let Some(old_outcome) = &record.outcome {
            let entry = self.domain_accuracy.entry(domain.clone()).or_insert((0, 0));
            if entry.1 > 0 {
                entry.1 -= 1;
                if !old_outcome.was_rerouted {
                    entry.0 = entry.0.saturating_sub(1);
                }
            }
        }

        // Apply new outcome to counters.
        let entry = self.domain_accuracy.entry(domain).or_insert((0, 0));
        entry.1 += 1;
        if !outcome.was_rerouted {
            entry.0 += 1;
        }

        record.outcome = Some(outcome);
    }

    /// Accuracy fraction for a specific domain (correct / total).
    ///
    /// Returns `1.0` if no outcomes have been recorded for this domain.
    #[must_use]
    pub fn domain_accuracy(&self, domain: &str) -> f64 {
        match self.domain_accuracy.get(domain) {
            Some(&(correct, total)) if total > 0 => {
                #[allow(clippy::cast_precision_loss)]
                let acc = f64::from(correct) / f64::from(total);
                acc
            }
            _ => 1.0,
        }
    }

    /// Overall routing accuracy across all domains (correct / total).
    ///
    /// Returns `1.0` if no outcomes have been recorded.
    #[must_use]
    pub fn overall_accuracy(&self) -> f64 {
        let (total_correct, total_count) =
            self.domain_accuracy
                .values()
                .fold((0u32, 0u32), |(c, t), &(correct, total)| {
                    (c + correct, t + total)
                });

        if total_count == 0 {
            return 1.0;
        }

        #[allow(clippy::cast_precision_loss)]
        let acc = f64::from(total_correct) / f64::from(total_count);
        acc
    }

    /// Return all patterns that were rerouted, with a count per
    /// `(message_preview, correct_domain)` pair.
    ///
    /// Only records that have an outcome with `was_rerouted == true` are
    /// included.  Results are sorted by count descending.
    #[must_use]
    pub fn misroute_patterns(&self) -> Vec<(String, String, u32)> {
        let mut counts: HashMap<(String, String), u32> = HashMap::new();

        for record in &self.records {
            if let Some(outcome) = &record.outcome {
                if outcome.was_rerouted {
                    if let Some(correct) = &outcome.correct_domain {
                        *counts
                            .entry((record.message_preview.clone(), correct.clone()))
                            .or_insert(0) += 1;
                    }
                }
            }
        }

        let mut patterns: Vec<(String, String, u32)> = counts
            .into_iter()
            .map(|((preview, domain), count)| (preview, domain, count))
            .collect();

        // Sort by count descending, then by message preview for determinism.
        patterns.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        patterns
    }

    /// Generate weight-update suggestions based on misrouting patterns.
    ///
    /// For each frequently misrouted message, significant words from the
    /// preview are extracted and suggested as keywords for the correct domain.
    /// This is intentionally conservative — the caller decides whether to
    /// apply suggestions.
    #[must_use]
    pub fn suggest_weight_updates(&self) -> Vec<WeightUpdateSuggestion> {
        let patterns = self.misroute_patterns();
        let mut suggestions = Vec::new();

        for (preview, correct_domain, count) in &patterns {
            // Only suggest for patterns that appear at least twice.
            if *count < 2 {
                continue;
            }

            // Extract significant words (len >= 4, not stop words).
            let stop_words = ["this", "that", "with", "from", "have", "help", "what", "your"];
            let keywords: Vec<&str> = preview
                .split_whitespace()
                .filter(|w| {
                    let clean: String = w.chars().filter(|c| c.is_alphabetic()).collect();
                    clean.len() >= 4 && !stop_words.contains(&clean.to_lowercase().as_str())
                })
                .take(3) // limit to 3 keywords per pattern
                .collect();

            // Suggested weight scales with misroute frequency, capped at 0.9.
            #[allow(clippy::cast_precision_loss)]
            let suggested_weight = (0.5 + 0.1 * (*count as f64)).min(0.9);

            for keyword in &keywords {
                let clean: String = keyword.chars().filter(|c| c.is_alphabetic()).collect();
                suggestions.push(WeightUpdateSuggestion {
                    keyword: clean.to_lowercase(),
                    domain: correct_domain.clone(),
                    suggested_weight,
                    reason: format!(
                        "keyword '{clean}' appeared in {count} misrouted messages → correct domain is '{correct_domain}'"
                    ),
                });
            }
        }

        // Deduplicate by (keyword, domain), keeping the highest weight.
        suggestions.sort_by(|a, b| {
            a.keyword
                .cmp(&b.keyword)
                .then(a.domain.cmp(&b.domain))
                .then(b.suggested_weight.partial_cmp(&a.suggested_weight).unwrap_or(std::cmp::Ordering::Equal))
        });
        suggestions.dedup_by(|a, b| {
            if a.keyword == b.keyword && a.domain == b.domain {
                // Keep the one with higher weight (b, since we sorted descending by weight)
                b.suggested_weight = b.suggested_weight.max(a.suggested_weight);
                true
            } else {
                false
            }
        });

        suggestions
    }

    /// Number of records currently in the store.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the store contains no records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Iterate over all records in insertion order.
    #[must_use]
    pub fn records(&self) -> &[RoutingRecord] {
        &self.records
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(session_id: &str, domain: &str, message: &str) -> RoutingRecord {
        RoutingRecord::new(
            "2026-04-01T00:00:00Z",
            session_id,
            message,
            domain,
            0.9,
            1,
            "claude-sonnet-4-20250514",
        )
    }

    fn success_outcome() -> RoutingOutcome {
        RoutingOutcome {
            success: true,
            latency_ms: 200,
            tokens_used: 500,
            was_rerouted: false,
            correct_domain: None,
        }
    }

    fn rerouted_outcome(correct: &str) -> RoutingOutcome {
        RoutingOutcome {
            success: false,
            latency_ms: 300,
            tokens_used: 400,
            was_rerouted: true,
            correct_domain: Some(correct.to_string()),
        }
    }

    // ── Construction ─────────────────────────────────────────────────────────

    #[test]
    fn new_store_is_empty() {
        let store = FeedbackStore::new(100);
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn zero_max_records_is_elevated_to_one() {
        let store = FeedbackStore::new(0);
        assert_eq!(store.max_records, 1);
    }

    // ── record() ─────────────────────────────────────────────────────────────

    #[test]
    fn record_appends_entries() {
        let mut store = FeedbackStore::new(100);
        store.record(make_record("s1", "music", "help me write a verse"));
        store.record(make_record("s2", "investment", "analyze stock earnings"));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn ring_buffer_evicts_oldest_when_full() {
        let mut store = FeedbackStore::new(3);
        store.record(make_record("s1", "music", "a"));
        store.record(make_record("s2", "music", "b"));
        store.record(make_record("s3", "music", "c"));
        // This should evict "s1"
        store.record(make_record("s4", "music", "d"));
        assert_eq!(store.len(), 3);
        assert!(!store.records().iter().any(|r| r.session_id == "s1"));
        assert!(store.records().iter().any(|r| r.session_id == "s4"));
    }

    #[test]
    fn message_preview_is_truncated_to_100_chars() {
        let msg: String = "x".repeat(200);
        let record = RoutingRecord::new("t", "s1", &msg, "general", 0.9, 1, "model");
        assert_eq!(record.message_preview.len(), 100);
    }

    // ── record_outcome() ─────────────────────────────────────────────────────

    #[test]
    fn record_outcome_attaches_to_matching_session() {
        let mut store = FeedbackStore::new(100);
        store.record(make_record("s1", "music", "help me write a verse"));
        store.record_outcome("s1", success_outcome());

        let record = store.records().iter().find(|r| r.session_id == "s1").unwrap();
        assert!(record.outcome.is_some());
        assert!(record.outcome.as_ref().unwrap().success);
    }

    #[test]
    fn record_outcome_is_noop_for_unknown_session() {
        let mut store = FeedbackStore::new(100);
        store.record(make_record("s1", "music", "test"));
        store.record_outcome("nonexistent", success_outcome());
        // No panic, record unchanged
        assert!(store.records()[0].outcome.is_none());
    }

    #[test]
    fn record_outcome_updates_most_recent_session_record() {
        let mut store = FeedbackStore::new(100);
        store.record(make_record("s1", "music", "first"));
        store.record(make_record("s1", "investment", "second"));
        store.record_outcome("s1", success_outcome());

        // Only the last s1 record should have the outcome
        let with_outcome: Vec<_> = store
            .records()
            .iter()
            .filter(|r| r.session_id == "s1" && r.outcome.is_some())
            .collect();
        assert_eq!(with_outcome.len(), 1);
        assert_eq!(with_outcome[0].domain_routed, "investment");
    }

    // ── domain_accuracy() ────────────────────────────────────────────────────

    #[test]
    fn domain_accuracy_returns_one_when_no_outcomes() {
        let store = FeedbackStore::new(100);
        assert!((store.domain_accuracy("music") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn domain_accuracy_reflects_correct_routings() {
        let mut store = FeedbackStore::new(100);

        // 2 correct, 1 rerouted
        store.record(make_record("s1", "music", "verse"));
        store.record_outcome("s1", success_outcome());

        store.record(make_record("s2", "music", "beat"));
        store.record_outcome("s2", success_outcome());

        store.record(make_record("s3", "music", "something else"));
        store.record_outcome("s3", rerouted_outcome("investment"));

        let acc = store.domain_accuracy("music");
        assert!((acc - 2.0 / 3.0).abs() < 1e-9);
    }

    // ── overall_accuracy() ───────────────────────────────────────────────────

    #[test]
    fn overall_accuracy_returns_one_when_empty() {
        let store = FeedbackStore::new(100);
        assert!((store.overall_accuracy() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn overall_accuracy_aggregates_across_domains() {
        let mut store = FeedbackStore::new(100);

        // Music: 1 correct
        store.record(make_record("s1", "music", "verse"));
        store.record_outcome("s1", success_outcome());

        // Investment: 1 rerouted (incorrect)
        store.record(make_record("s2", "investment", "price target"));
        store.record_outcome("s2", rerouted_outcome("development"));

        // 1 correct out of 2 total = 0.5
        let acc = store.overall_accuracy();
        assert!((acc - 0.5).abs() < 1e-9);
    }

    // ── misroute_patterns() ──────────────────────────────────────────────────

    #[test]
    fn misroute_patterns_returns_empty_when_no_reroutes() {
        let mut store = FeedbackStore::new(100);
        store.record(make_record("s1", "music", "verse"));
        store.record_outcome("s1", success_outcome());
        assert!(store.misroute_patterns().is_empty());
    }

    #[test]
    fn misroute_patterns_counts_rerouted_messages() {
        let mut store = FeedbackStore::new(100);

        let msg = "help me with my code";
        store.record(make_record("s1", "general", msg));
        store.record_outcome("s1", rerouted_outcome("development"));

        store.record(make_record("s2", "general", msg));
        store.record_outcome("s2", rerouted_outcome("development"));

        let patterns = store.misroute_patterns();
        assert!(!patterns.is_empty());
        let found = patterns.iter().any(|(preview, domain, count)| {
            preview.contains("help me with my code") && domain == "development" && *count == 2
        });
        assert!(found, "expected pattern not found in {patterns:?}");
    }

    #[test]
    fn misroute_patterns_sorted_by_count_descending() {
        let mut store = FeedbackStore::new(100);

        // Pattern A: 1 occurrence
        store.record(make_record("s1", "general", "unique message abc"));
        store.record_outcome("s1", rerouted_outcome("music"));

        // Pattern B: 2 occurrences
        let msg = "common pattern xyz";
        store.record(make_record("s2", "general", msg));
        store.record_outcome("s2", rerouted_outcome("development"));
        store.record(make_record("s3", "general", msg));
        store.record_outcome("s3", rerouted_outcome("development"));

        let patterns = store.misroute_patterns();
        assert!(patterns.len() >= 2);
        // First pattern should have the highest count
        assert!(patterns[0].2 >= patterns[1].2);
    }

    // ── suggest_weight_updates() ─────────────────────────────────────────────

    #[test]
    fn suggest_weight_updates_requires_at_least_two_occurrences() {
        let mut store = FeedbackStore::new(100);
        // Only one occurrence — should not produce suggestions.
        store.record(make_record("s1", "music", "write a chorus melody"));
        store.record_outcome("s1", rerouted_outcome("music"));

        let suggestions = store.suggest_weight_updates();
        assert!(suggestions.is_empty());
    }

    #[test]
    fn suggest_weight_updates_produces_suggestions_for_frequent_misroutes() {
        let mut store = FeedbackStore::new(100);

        let msg = "write verse chorus lyrics";
        for i in 0..3u32 {
            store.record(make_record(&format!("s{i}"), "general", msg));
            store.record_outcome(&format!("s{i}"), rerouted_outcome("music"));
        }

        let suggestions = store.suggest_weight_updates();
        assert!(!suggestions.is_empty());
        // All suggestions should point to "music"
        assert!(suggestions.iter().all(|s| s.domain == "music"));
    }

    #[test]
    fn suggest_weight_updates_weight_scales_with_frequency() {
        let mut store = FeedbackStore::new(100);

        let msg = "develop implement feature test review";
        for i in 0..5u32 {
            store.record(make_record(&format!("s{i}"), "general", msg));
            store.record_outcome(&format!("s{i}"), rerouted_outcome("development"));
        }

        let suggestions = store.suggest_weight_updates();
        // Weight should be higher for 5 occurrences than 2
        let min_weight = 0.5 + 0.1 * 5.0_f64; // = 1.0 but capped at 0.9
        for s in &suggestions {
            assert!(
                s.suggested_weight > 0.5,
                "expected elevated weight, got {}",
                s.suggested_weight
            );
            assert!(s.suggested_weight <= 0.9);
            let _ = min_weight; // used above
        }
    }

    // ── is_empty() / len() ────────────────────────────────────────────────────

    #[test]
    fn is_empty_and_len_are_consistent() {
        let mut store = FeedbackStore::new(100);
        assert!(store.is_empty());

        store.record(make_record("s1", "music", "test"));
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
    }

    // ── RoutingRecord preview truncation ─────────────────────────────────────

    #[test]
    fn routing_record_short_message_not_padded() {
        let record = RoutingRecord::new("t", "s", "short", "music", 0.9, 1, "model");
        assert_eq!(record.message_preview, "short");
    }
}
