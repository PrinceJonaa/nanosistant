//! Three heterogeneous evaluators for the self-learning loop.
//!
//! Structural (Rust) × Semantic (LLM) × Human (Jona) = weighted update signal.
//!
//! ## Design
//!
//! - [`StructuralEvaluator`]: deterministic, zero LLM tokens, runs in Rust.
//! - [`HumanSignalCollector`]: stores signals from Jona (keyed by session_id).
//! - [`evaluate_for_learning`]: combines all three signals → optional `Vec<WeightDelta>`.

use std::collections::HashMap;

use nstn_common::typed_ir::{
    HumanSignal, HumanSignalType, SemanticEval, StructuralEval, WeightDelta, WeightDirection,
    WeightMagnitude,
};

use crate::memory::{ToolCallRecord, ToolCallStatus};

// ═══════════════════════════════════════
// Structural Evaluator
// ═══════════════════════════════════════

/// Structural evaluator — deterministic, runs in Rust.
///
/// Compares expected vs actual outputs by string equality and checks whether
/// all tool calls succeeded.
pub struct StructuralEvaluator;

impl StructuralEvaluator {
    /// Evaluate structural correctness.
    ///
    /// - `schema_valid`: true when every string in `expected_outputs` appears
    ///   in `actual_outputs` (subset check — actual may contain extras).
    /// - `preconditions_met`: true when `expected_outputs` is non-empty and
    ///   `actual_outputs` is also non-empty.
    /// - `tool_calls_succeeded`: true when no tool call has status `Error` or
    ///   `Timeout`.
    /// - `details`: human-readable summary of discrepancies.
    #[must_use]
    pub fn evaluate(
        expected_outputs: &[String],
        actual_outputs: &[String],
        tool_calls: &[ToolCallRecord],
    ) -> StructuralEval {
        let mut issues: Vec<String> = Vec::new();

        // ── Preconditions: both lists non-empty ──────────────────────────────
        let preconditions_met = !expected_outputs.is_empty() && !actual_outputs.is_empty();
        if !preconditions_met {
            if expected_outputs.is_empty() {
                issues.push("expected_outputs is empty".to_string());
            }
            if actual_outputs.is_empty() {
                issues.push("actual_outputs is empty".to_string());
            }
        }

        // ── Schema valid: all expected appear in actual ───────────────────────
        let missing: Vec<&String> = expected_outputs
            .iter()
            .filter(|exp| !actual_outputs.iter().any(|act| act == *exp))
            .collect();

        let schema_valid = missing.is_empty();
        if !schema_valid {
            for m in &missing {
                issues.push(format!("expected output '{}' not found in actuals", m));
            }
        }

        // ── Tool calls: no errors or timeouts ────────────────────────────────
        let failed_tools: Vec<&ToolCallRecord> = tool_calls
            .iter()
            .filter(|t| {
                matches!(t.status, ToolCallStatus::Error | ToolCallStatus::Timeout)
            })
            .collect();

        let tool_calls_succeeded = failed_tools.is_empty();
        if !tool_calls_succeeded {
            for t in &failed_tools {
                issues.push(format!(
                    "tool '{}' failed with status {:?}",
                    t.tool, t.status
                ));
            }
        }

        let details = if issues.is_empty() {
            "all structural checks passed".to_string()
        } else {
            issues.join("; ")
        };

        StructuralEval {
            schema_valid,
            preconditions_met,
            tool_calls_succeeded,
            details,
        }
    }
}

// ═══════════════════════════════════════
// Human Signal Collector
// ═══════════════════════════════════════

/// Human signal collector — stores signals from Jona.
///
/// Keyed by `session_id`. Later signals overwrite earlier ones for the same
/// session (most recent signal wins).
pub struct HumanSignalCollector {
    signals: Vec<(String, HumanSignal)>, // (session_id, signal)
}

impl HumanSignalCollector {
    /// Create a new, empty collector.
    #[must_use]
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
        }
    }

    /// Record a human signal for `session_id`.
    ///
    /// If a signal already exists for this session, it is replaced.
    pub fn record(&mut self, session_id: &str, signal: HumanSignal) {
        // Replace existing entry for this session_id, or push new.
        if let Some(entry) = self.signals.iter_mut().find(|(id, _)| id == session_id) {
            entry.1 = signal;
        } else {
            self.signals.push((session_id.to_string(), signal));
        }
    }

    /// Retrieve the most recent signal for `session_id`, if any.
    #[must_use]
    pub fn get(&self, session_id: &str) -> Option<&HumanSignal> {
        self.signals
            .iter()
            .rfind(|(id, _)| id == session_id)
            .map(|(_, signal)| signal)
    }

    /// Number of recorded signals.
    #[must_use]
    pub fn len(&self) -> usize {
        self.signals.len()
    }

    /// Whether no signals have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.signals.is_empty()
    }
}

impl Default for HumanSignalCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════
// Evaluate for Learning
// ═══════════════════════════════════════

/// Combine all three signals into a weight update decision.
///
/// Returns `Some(deltas)` only when all conditions are met:
///
/// 1. Human signal is `Some` and its type is `Confirm`.
/// 2. Structural evaluation passed (all three checks true).
/// 3. Semantic alignment > 0.5.
///
/// When conditions are met, produces one or more [`WeightDelta`] proposals:
///
/// - A positive delta for the task type (direction = Increase).
/// - If semantic alignment is very high (≥ 0.9), the magnitude is `Large`;
///   if ≥ 0.7, `Medium`; otherwise `Small`.
///
/// Returns `None` when any condition fails (blocking weight update).
#[must_use]
pub fn evaluate_for_learning(
    structural: &StructuralEval,
    semantic: &SemanticEval,
    human: Option<&HumanSignal>,
) -> Option<Vec<WeightDelta>> {
    // Gate 1: human signal must be present and Confirm
    let human = human?;
    if human.signal_type != HumanSignalType::Confirm {
        return None;
    }

    // Gate 2: structural must have passed
    if !structural.schema_valid || !structural.preconditions_met || !structural.tool_calls_succeeded
    {
        return None;
    }

    // Gate 3: semantic alignment must be > 0.5
    if semantic.alignment <= 0.5 {
        return None;
    }

    // All gates passed — determine magnitude from alignment score.
    let magnitude = if semantic.alignment >= 0.9 {
        WeightMagnitude::Large
    } else if semantic.alignment >= 0.7 {
        WeightMagnitude::Medium
    } else {
        WeightMagnitude::Small
    };

    let delta = WeightDelta {
        task_type: "general".to_string(),
        route_stage: "tier1".to_string(),
        direction: WeightDirection::Increase,
        magnitude,
        reason: format!(
            "human confirmed, structural passed, semantic alignment = {:.2}",
            semantic.alignment
        ),
        supporting_episodes: Vec::new(),
    };

    Some(vec![delta])
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use nstn_common::typed_ir::{HumanSignal, HumanSignalType, SemanticEval, StructuralEval};
    use crate::memory::{ToolCallRecord, ToolCallStatus};

    // ── StructuralEvaluator ──────────────────────────────────────────────────

    fn ok_tool(tool: &str) -> ToolCallRecord {
        ToolCallRecord {
            tool: tool.to_string(),
            input_summary: "input".to_string(),
            output_summary: "output".to_string(),
            status: ToolCallStatus::Ok,
        }
    }

    fn err_tool(tool: &str) -> ToolCallRecord {
        ToolCallRecord {
            tool: tool.to_string(),
            input_summary: "input".to_string(),
            output_summary: "error".to_string(),
            status: ToolCallStatus::Error,
        }
    }

    fn timeout_tool(tool: &str) -> ToolCallRecord {
        ToolCallRecord {
            tool: tool.to_string(),
            input_summary: "input".to_string(),
            output_summary: "timeout".to_string(),
            status: ToolCallStatus::Timeout,
        }
    }

    #[test]
    fn structural_all_pass() {
        let expected = vec!["binary".to_string(), "report.json".to_string()];
        let actual = vec!["binary".to_string(), "report.json".to_string(), "extra.log".to_string()];
        let tools = vec![ok_tool("bash"), ok_tool("file_ops")];

        let eval = StructuralEvaluator::evaluate(&expected, &actual, &tools);
        assert!(eval.schema_valid);
        assert!(eval.preconditions_met);
        assert!(eval.tool_calls_succeeded);
        assert!(eval.details.contains("passed"));
    }

    #[test]
    fn structural_missing_expected_output() {
        let expected = vec!["binary".to_string(), "report.json".to_string()];
        let actual = vec!["binary".to_string()]; // report.json missing
        let tools = vec![ok_tool("bash")];

        let eval = StructuralEvaluator::evaluate(&expected, &actual, &tools);
        assert!(!eval.schema_valid);
        assert!(eval.details.contains("report.json"));
    }

    #[test]
    fn structural_empty_expected_outputs() {
        let eval = StructuralEvaluator::evaluate(&[], &["result".to_string()], &[]);
        assert!(!eval.preconditions_met);
        assert!(eval.details.contains("expected_outputs is empty"));
    }

    #[test]
    fn structural_empty_actual_outputs() {
        let eval = StructuralEvaluator::evaluate(&["result".to_string()], &[], &[]);
        assert!(!eval.preconditions_met);
        assert!(eval.details.contains("actual_outputs is empty"));
    }

    #[test]
    fn structural_tool_error_fails() {
        let expected = vec!["result".to_string()];
        let actual = vec!["result".to_string()];
        let tools = vec![ok_tool("bash"), err_tool("file_ops")];

        let eval = StructuralEvaluator::evaluate(&expected, &actual, &tools);
        assert!(!eval.tool_calls_succeeded);
        assert!(eval.details.contains("file_ops"));
    }

    #[test]
    fn structural_tool_timeout_fails() {
        let expected = vec!["result".to_string()];
        let actual = vec!["result".to_string()];
        let tools = vec![timeout_tool("bash")];

        let eval = StructuralEvaluator::evaluate(&expected, &actual, &tools);
        assert!(!eval.tool_calls_succeeded);
        assert!(eval.details.contains("bash"));
    }

    #[test]
    fn structural_no_tool_calls_is_ok() {
        let expected = vec!["result".to_string()];
        let actual = vec!["result".to_string()];

        let eval = StructuralEvaluator::evaluate(&expected, &actual, &[]);
        assert!(eval.tool_calls_succeeded);
    }

    // ── HumanSignalCollector ─────────────────────────────────────────────────

    #[test]
    fn collector_starts_empty() {
        let c = HumanSignalCollector::new();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn collector_record_and_get() {
        let mut c = HumanSignalCollector::new();
        c.record("session-1", HumanSignal {
            signal_type: HumanSignalType::Confirm,
            delta: None,
        });
        let sig = c.get("session-1").unwrap();
        assert_eq!(sig.signal_type, HumanSignalType::Confirm);
    }

    #[test]
    fn collector_get_missing_returns_none() {
        let c = HumanSignalCollector::new();
        assert!(c.get("nonexistent").is_none());
    }

    #[test]
    fn collector_later_signal_overwrites_earlier() {
        let mut c = HumanSignalCollector::new();
        c.record("session-1", HumanSignal {
            signal_type: HumanSignalType::Confirm,
            delta: None,
        });
        c.record("session-1", HumanSignal {
            signal_type: HumanSignalType::Abandon,
            delta: Some("changed my mind".to_string()),
        });
        let sig = c.get("session-1").unwrap();
        assert_eq!(sig.signal_type, HumanSignalType::Abandon);
        // Length stays 1 since same session
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn collector_multiple_sessions() {
        let mut c = HumanSignalCollector::new();
        c.record("s1", HumanSignal { signal_type: HumanSignalType::Confirm, delta: None });
        c.record("s2", HumanSignal { signal_type: HumanSignalType::Correct, delta: Some("use bash".to_string()) });
        assert_eq!(c.len(), 2);
        assert_eq!(c.get("s1").unwrap().signal_type, HumanSignalType::Confirm);
        assert_eq!(c.get("s2").unwrap().signal_type, HumanSignalType::Correct);
    }

    // ── evaluate_for_learning ────────────────────────────────────────────────

    fn passing_structural() -> StructuralEval {
        StructuralEval {
            schema_valid: true,
            preconditions_met: true,
            tool_calls_succeeded: true,
            details: "all good".to_string(),
        }
    }

    fn passing_semantic(alignment: f64) -> SemanticEval {
        SemanticEval {
            alignment,
            misalignment_reason: None,
        }
    }

    fn confirm_signal() -> HumanSignal {
        HumanSignal {
            signal_type: HumanSignalType::Confirm,
            delta: None,
        }
    }

    #[test]
    fn evaluate_all_conditions_met_returns_delta() {
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.8),
            Some(&confirm_signal()),
        );
        assert!(result.is_some());
        let deltas = result.unwrap();
        assert!(!deltas.is_empty());
        assert_eq!(deltas[0].direction, WeightDirection::Increase);
    }

    #[test]
    fn evaluate_no_human_signal_returns_none() {
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.8),
            None,
        );
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_human_correct_returns_none() {
        let human = HumanSignal {
            signal_type: HumanSignalType::Correct,
            delta: Some("adjust".to_string()),
        };
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.8),
            Some(&human),
        );
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_human_abandon_returns_none() {
        let human = HumanSignal {
            signal_type: HumanSignalType::Abandon,
            delta: None,
        };
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.8),
            Some(&human),
        );
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_structural_schema_invalid_returns_none() {
        let mut structural = passing_structural();
        structural.schema_valid = false;
        let result = evaluate_for_learning(
            &structural,
            &passing_semantic(0.8),
            Some(&confirm_signal()),
        );
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_structural_preconditions_failed_returns_none() {
        let mut structural = passing_structural();
        structural.preconditions_met = false;
        let result = evaluate_for_learning(
            &structural,
            &passing_semantic(0.8),
            Some(&confirm_signal()),
        );
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_structural_tool_calls_failed_returns_none() {
        let mut structural = passing_structural();
        structural.tool_calls_succeeded = false;
        let result = evaluate_for_learning(
            &structural,
            &passing_semantic(0.8),
            Some(&confirm_signal()),
        );
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_semantic_at_threshold_returns_none() {
        // Exactly 0.5 is not > 0.5
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.5),
            Some(&confirm_signal()),
        );
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_semantic_just_above_threshold_returns_delta() {
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.51),
            Some(&confirm_signal()),
        );
        assert!(result.is_some());
    }

    #[test]
    fn evaluate_magnitude_small_for_low_alignment() {
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.6), // 0.5 < 0.6 < 0.7 → Small
            Some(&confirm_signal()),
        ).unwrap();
        assert_eq!(result[0].magnitude, WeightMagnitude::Small);
    }

    #[test]
    fn evaluate_magnitude_medium_for_mid_alignment() {
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.75), // 0.7 ≤ 0.75 < 0.9 → Medium
            Some(&confirm_signal()),
        ).unwrap();
        assert_eq!(result[0].magnitude, WeightMagnitude::Medium);
    }

    #[test]
    fn evaluate_magnitude_large_for_high_alignment() {
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.95), // ≥ 0.9 → Large
            Some(&confirm_signal()),
        ).unwrap();
        assert_eq!(result[0].magnitude, WeightMagnitude::Large);
    }

    #[test]
    fn evaluate_magnitude_large_at_exact_0_9() {
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.9),
            Some(&confirm_signal()),
        ).unwrap();
        assert_eq!(result[0].magnitude, WeightMagnitude::Large);
    }

    #[test]
    fn evaluate_delta_reason_contains_alignment() {
        let result = evaluate_for_learning(
            &passing_structural(),
            &passing_semantic(0.78),
            Some(&confirm_signal()),
        ).unwrap();
        assert!(result[0].reason.contains("0.78"));
    }
}
