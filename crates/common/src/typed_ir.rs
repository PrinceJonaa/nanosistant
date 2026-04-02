//! Typed Intermediate Representations — the boundary between LLM proposals and execution.
//!
//! Design principle: LLMs propose, Rust executes. Every LLM output that leads to
//! a state change must pass through a typed IR. No LLM call directly invokes a
//! tool, writes a file, or changes routing weights.

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════
// Routing Proposal
// ═══════════════════════════════════════

/// LLM proposes how to route a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingProposal {
    pub intent_class: String,
    pub confidence: f64,
    pub tool_chain: Vec<String>,
    pub preconditions: Vec<String>,
    /// Must be one of: "ask_user", "next_tier", "abort"
    pub fallback: String,
}

// ═══════════════════════════════════════
// Execution Plan (IR form — no StepStatus)
// ═══════════════════════════════════════

/// LLM proposes an execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlanIR {
    pub goal: String,
    pub steps: Vec<PlanStepIR>,
}

/// A single proposed step — no runtime status attached.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStepIR {
    pub id: String,
    pub role: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub termination_condition: String,
    pub rollback: Option<String>,
}

// ═══════════════════════════════════════
// Tool Ranking
// ═══════════════════════════════════════

/// LLM ranks tools by relevance instead of calling them directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRanking {
    pub ranked_tools: Vec<RankedTool>,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedTool {
    pub tool: String,
    pub score: f64,
    pub rationale: String,
}

// ═══════════════════════════════════════
// Evaluation Signals
// ═══════════════════════════════════════

/// Three-signal evaluation from heterogeneous evaluators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationSignal {
    pub structural: StructuralEval,
    pub semantic: SemanticEval,
    pub human: Option<HumanSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralEval {
    pub schema_valid: bool,
    pub preconditions_met: bool,
    pub tool_calls_succeeded: bool,
    pub details: String,
}

impl StructuralEval {
    /// Returns true if all structural checks passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.schema_valid && self.preconditions_met && self.tool_calls_succeeded
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEval {
    /// Alignment score in range 0.0–1.0.
    pub alignment: f64,
    pub misalignment_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanSignal {
    pub signal_type: HumanSignalType,
    pub delta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HumanSignalType {
    Confirm,
    Correct,
    Abandon,
}

// ═══════════════════════════════════════
// Weight Delta
// ═══════════════════════════════════════

/// Typed weight update delta — never applied by LLM directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightDelta {
    pub task_type: String,
    pub route_stage: String,
    pub direction: WeightDirection,
    pub magnitude: WeightMagnitude,
    pub reason: String,
    pub supporting_episodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WeightDirection {
    Increase,
    Decrease,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WeightMagnitude {
    Small,
    Medium,
    Large,
}

// ═══════════════════════════════════════
// Alignment Check
// ═══════════════════════════════════════

/// Alignment check before non-trivial actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentCheck {
    pub action: String,
    pub check_results: AlignmentResults,
    pub decision: AlignmentDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentResults {
    pub external_gate_required: bool,
    pub reversible: bool,
    pub touches_l3: bool,
    pub god_time_status: GodTimeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GodTimeStatus {
    Confirmed,
    DriftDetected,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlignmentDecision {
    Proceed,
    Block { reason: String },
    QueueForReview { reason: String },
}

// ═══════════════════════════════════════
// Validation Functions
// ═══════════════════════════════════════

/// Validate a [`RoutingProposal`]. Returns errors if invalid.
///
/// Checks:
/// - `confidence` is in [0.0, 1.0]
/// - `tool_chain` is non-empty
/// - `fallback` is one of "ask_user", "next_tier", "abort"
pub fn validate_routing_proposal(p: &RoutingProposal) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if p.confidence < 0.0 || p.confidence > 1.0 {
        errors.push(format!(
            "confidence must be in [0.0, 1.0], got {}",
            p.confidence
        ));
    }

    if p.tool_chain.is_empty() {
        errors.push("tool_chain must not be empty".to_string());
    }

    let valid_fallbacks = ["ask_user", "next_tier", "abort"];
    if !valid_fallbacks.contains(&p.fallback.as_str()) {
        errors.push(format!(
            "fallback must be one of {:?}, got '{}'",
            valid_fallbacks, p.fallback
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate an [`ExecutionPlanIR`]. Returns errors if invalid.
///
/// Checks:
/// - `steps` is non-empty
/// - each step has non-empty `id`, `role`, and `description`
pub fn validate_execution_plan(p: &ExecutionPlanIR) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if p.steps.is_empty() {
        errors.push("execution plan must have at least one step".to_string());
    }

    for (i, step) in p.steps.iter().enumerate() {
        if step.id.is_empty() {
            errors.push(format!("step[{i}] has empty id"));
        }
        if step.role.is_empty() {
            errors.push(format!("step[{i}] has empty role"));
        }
        if step.description.is_empty() {
            errors.push(format!("step[{i}] has empty description"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a [`ToolRanking`]. Returns errors if invalid.
///
/// Checks:
/// - `ranked_tools` is non-empty
/// - each tool's `score` is in [0.0, 1.0]
pub fn validate_tool_ranking(p: &ToolRanking) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if p.ranked_tools.is_empty() {
        errors.push("ranked_tools must not be empty".to_string());
    }

    for (i, ranked) in p.ranked_tools.iter().enumerate() {
        if ranked.score < 0.0 || ranked.score > 1.0 {
            errors.push(format!(
                "ranked_tools[{i}].score must be in [0.0, 1.0], got {}",
                ranked.score
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate an [`AlignmentCheck`]. Returns errors if invalid.
///
/// Checks:
/// - `god_time_status` is a known variant (always valid — enforced by enum)
/// - `action` is non-empty
/// - If `external_gate_required` is true and decision is `Proceed`, that is
///   flagged as suspicious (external gate required but no block/review).
pub fn validate_alignment_check(p: &AlignmentCheck) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if p.action.is_empty() {
        errors.push("action must not be empty".to_string());
    }

    // If god_time_status is DriftDetected, decision must not be Proceed.
    if p.check_results.god_time_status == GodTimeStatus::DriftDetected {
        if let AlignmentDecision::Proceed = p.decision {
            errors.push(
                "decision cannot be Proceed when god_time_status is DriftDetected".to_string(),
            );
        }
    }

    // If external gate required, decision should not be Proceed.
    if p.check_results.external_gate_required {
        if let AlignmentDecision::Proceed = p.decision {
            errors.push(
                "decision cannot be Proceed when external_gate_required is true".to_string(),
            );
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ═══════════════════════════════════════
// Weight Update Gate
// ═══════════════════════════════════════

/// Check if all three evaluation signals agree enough to update weights.
///
/// All conditions must hold:
/// 1. All three signals present (human is `Some`)
/// 2. Structural evaluation passed (schema valid, preconditions met, tool calls succeeded)
/// 3. Semantic alignment > 0.5
/// 4. Human signal type is `Confirm`
#[must_use]
pub fn can_update_weights(signal: &EvaluationSignal) -> bool {
    // 1. Human signal must be present
    let Some(human) = &signal.human else {
        return false;
    };

    // 2. Structural must have passed
    if !signal.structural.passed() {
        return false;
    }

    // 3. Semantic alignment must be > 0.5
    if signal.semantic.alignment <= 0.5 {
        return false;
    }

    // 4. Human must have confirmed
    human.signal_type == HumanSignalType::Confirm
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_routing_proposal ────────────────────────────────────────────

    fn valid_routing_proposal() -> RoutingProposal {
        RoutingProposal {
            intent_class: "code_edit".to_string(),
            confidence: 0.85,
            tool_chain: vec!["bash".to_string(), "file_ops".to_string()],
            preconditions: vec![],
            fallback: "next_tier".to_string(),
        }
    }

    #[test]
    fn routing_proposal_valid() {
        let p = valid_routing_proposal();
        assert!(validate_routing_proposal(&p).is_ok());
    }

    #[test]
    fn routing_proposal_confidence_too_high() {
        let mut p = valid_routing_proposal();
        p.confidence = 1.1;
        let errs = validate_routing_proposal(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("confidence")));
    }

    #[test]
    fn routing_proposal_confidence_negative() {
        let mut p = valid_routing_proposal();
        p.confidence = -0.1;
        let errs = validate_routing_proposal(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("confidence")));
    }

    #[test]
    fn routing_proposal_empty_tool_chain() {
        let mut p = valid_routing_proposal();
        p.tool_chain = vec![];
        let errs = validate_routing_proposal(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("tool_chain")));
    }

    #[test]
    fn routing_proposal_invalid_fallback() {
        let mut p = valid_routing_proposal();
        p.fallback = "do_nothing".to_string();
        let errs = validate_routing_proposal(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("fallback")));
    }

    #[test]
    fn routing_proposal_valid_fallback_variants() {
        for fb in ["ask_user", "next_tier", "abort"] {
            let mut p = valid_routing_proposal();
            p.fallback = fb.to_string();
            assert!(
                validate_routing_proposal(&p).is_ok(),
                "fallback '{fb}' should be valid"
            );
        }
    }

    // ── validate_execution_plan ──────────────────────────────────────────────

    fn valid_plan_step(id: &str) -> PlanStepIR {
        PlanStepIR {
            id: id.to_string(),
            role: "executor".to_string(),
            description: "Run the build".to_string(),
            inputs: vec!["src".to_string()],
            expected_outputs: vec!["binary".to_string()],
            termination_condition: "exit_code == 0".to_string(),
            rollback: None,
        }
    }

    fn valid_execution_plan() -> ExecutionPlanIR {
        ExecutionPlanIR {
            goal: "Build the project".to_string(),
            steps: vec![valid_plan_step("step-1"), valid_plan_step("step-2")],
        }
    }

    #[test]
    fn execution_plan_valid() {
        let p = valid_execution_plan();
        assert!(validate_execution_plan(&p).is_ok());
    }

    #[test]
    fn execution_plan_empty_steps() {
        let mut p = valid_execution_plan();
        p.steps = vec![];
        let errs = validate_execution_plan(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("step")));
    }

    #[test]
    fn execution_plan_step_missing_id() {
        let mut p = valid_execution_plan();
        p.steps[0].id = String::new();
        let errs = validate_execution_plan(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("id")));
    }

    #[test]
    fn execution_plan_step_missing_role() {
        let mut p = valid_execution_plan();
        p.steps[0].role = String::new();
        let errs = validate_execution_plan(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("role")));
    }

    #[test]
    fn execution_plan_step_missing_description() {
        let mut p = valid_execution_plan();
        p.steps[0].description = String::new();
        let errs = validate_execution_plan(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("description")));
    }

    // ── validate_tool_ranking ────────────────────────────────────────────────

    fn valid_tool_ranking() -> ToolRanking {
        ToolRanking {
            ranked_tools: vec![
                RankedTool {
                    tool: "bash".to_string(),
                    score: 0.9,
                    rationale: "best for shell tasks".to_string(),
                },
                RankedTool {
                    tool: "file_ops".to_string(),
                    score: 0.7,
                    rationale: "good for file operations".to_string(),
                },
            ],
            query: "run a build command".to_string(),
        }
    }

    #[test]
    fn tool_ranking_valid() {
        let p = valid_tool_ranking();
        assert!(validate_tool_ranking(&p).is_ok());
    }

    #[test]
    fn tool_ranking_empty_tools() {
        let mut p = valid_tool_ranking();
        p.ranked_tools = vec![];
        let errs = validate_tool_ranking(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("ranked_tools")));
    }

    #[test]
    fn tool_ranking_score_out_of_range_high() {
        let mut p = valid_tool_ranking();
        p.ranked_tools[0].score = 1.5;
        let errs = validate_tool_ranking(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("score")));
    }

    #[test]
    fn tool_ranking_score_out_of_range_low() {
        let mut p = valid_tool_ranking();
        p.ranked_tools[1].score = -0.1;
        let errs = validate_tool_ranking(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("score")));
    }

    #[test]
    fn tool_ranking_boundary_scores_valid() {
        let mut p = valid_tool_ranking();
        p.ranked_tools[0].score = 0.0;
        p.ranked_tools[1].score = 1.0;
        assert!(validate_tool_ranking(&p).is_ok());
    }

    // ── validate_alignment_check ─────────────────────────────────────────────

    fn valid_alignment_check() -> AlignmentCheck {
        AlignmentCheck {
            action: "write_file".to_string(),
            check_results: AlignmentResults {
                external_gate_required: false,
                reversible: true,
                touches_l3: false,
                god_time_status: GodTimeStatus::Confirmed,
            },
            decision: AlignmentDecision::Proceed,
        }
    }

    #[test]
    fn alignment_check_valid_proceed() {
        let p = valid_alignment_check();
        assert!(validate_alignment_check(&p).is_ok());
    }

    #[test]
    fn alignment_check_empty_action() {
        let mut p = valid_alignment_check();
        p.action = String::new();
        let errs = validate_alignment_check(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("action")));
    }

    #[test]
    fn alignment_check_drift_detected_cannot_proceed() {
        let mut p = valid_alignment_check();
        p.check_results.god_time_status = GodTimeStatus::DriftDetected;
        p.decision = AlignmentDecision::Proceed;
        let errs = validate_alignment_check(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("DriftDetected")));
    }

    #[test]
    fn alignment_check_drift_detected_can_block() {
        let mut p = valid_alignment_check();
        p.check_results.god_time_status = GodTimeStatus::DriftDetected;
        p.decision = AlignmentDecision::Block {
            reason: "drift detected".to_string(),
        };
        assert!(validate_alignment_check(&p).is_ok());
    }

    #[test]
    fn alignment_check_external_gate_cannot_proceed() {
        let mut p = valid_alignment_check();
        p.check_results.external_gate_required = true;
        p.decision = AlignmentDecision::Proceed;
        let errs = validate_alignment_check(&p).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("external_gate_required")));
    }

    #[test]
    fn alignment_check_external_gate_queue_ok() {
        let mut p = valid_alignment_check();
        p.check_results.external_gate_required = true;
        p.decision = AlignmentDecision::QueueForReview {
            reason: "needs review".to_string(),
        };
        assert!(validate_alignment_check(&p).is_ok());
    }

    // ── can_update_weights ───────────────────────────────────────────────────

    fn passing_signal() -> EvaluationSignal {
        EvaluationSignal {
            structural: StructuralEval {
                schema_valid: true,
                preconditions_met: true,
                tool_calls_succeeded: true,
                details: "all good".to_string(),
            },
            semantic: SemanticEval {
                alignment: 0.8,
                misalignment_reason: None,
            },
            human: Some(HumanSignal {
                signal_type: HumanSignalType::Confirm,
                delta: None,
            }),
        }
    }

    #[test]
    fn can_update_weights_all_passing() {
        let signal = passing_signal();
        assert!(can_update_weights(&signal));
    }

    #[test]
    fn can_update_weights_no_human_signal() {
        let mut signal = passing_signal();
        signal.human = None;
        assert!(!can_update_weights(&signal));
    }

    #[test]
    fn can_update_weights_structural_failed_schema() {
        let mut signal = passing_signal();
        signal.structural.schema_valid = false;
        assert!(!can_update_weights(&signal));
    }

    #[test]
    fn can_update_weights_structural_failed_preconditions() {
        let mut signal = passing_signal();
        signal.structural.preconditions_met = false;
        assert!(!can_update_weights(&signal));
    }

    #[test]
    fn can_update_weights_structural_failed_tool_calls() {
        let mut signal = passing_signal();
        signal.structural.tool_calls_succeeded = false;
        assert!(!can_update_weights(&signal));
    }

    #[test]
    fn can_update_weights_semantic_below_threshold() {
        let mut signal = passing_signal();
        signal.semantic.alignment = 0.5; // must be > 0.5
        assert!(!can_update_weights(&signal));
    }

    #[test]
    fn can_update_weights_semantic_just_above_threshold() {
        let mut signal = passing_signal();
        signal.semantic.alignment = 0.51;
        assert!(can_update_weights(&signal));
    }

    #[test]
    fn can_update_weights_human_correct_not_confirm() {
        let mut signal = passing_signal();
        signal.human = Some(HumanSignal {
            signal_type: HumanSignalType::Correct,
            delta: Some("use a different tool".to_string()),
        });
        assert!(!can_update_weights(&signal));
    }

    #[test]
    fn can_update_weights_human_abandon() {
        let mut signal = passing_signal();
        signal.human = Some(HumanSignal {
            signal_type: HumanSignalType::Abandon,
            delta: None,
        });
        assert!(!can_update_weights(&signal));
    }

    // ── Serde round-trip ─────────────────────────────────────────────────────

    #[test]
    fn routing_proposal_serde_round_trip() {
        let p = valid_routing_proposal();
        let json = serde_json::to_string(&p).unwrap();
        let p2: RoutingProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(p.intent_class, p2.intent_class);
        assert!((p.confidence - p2.confidence).abs() < f64::EPSILON);
    }

    #[test]
    fn weight_delta_serde_round_trip() {
        let delta = WeightDelta {
            task_type: "code_edit".to_string(),
            route_stage: "tier2".to_string(),
            direction: WeightDirection::Increase,
            magnitude: WeightMagnitude::Medium,
            reason: "consistent success".to_string(),
            supporting_episodes: vec!["ep-1".to_string(), "ep-2".to_string()],
        };
        let json = serde_json::to_string(&delta).unwrap();
        let delta2: WeightDelta = serde_json::from_str(&json).unwrap();
        assert_eq!(delta.task_type, delta2.task_type);
        assert_eq!(delta.direction, delta2.direction);
        assert_eq!(delta.magnitude, delta2.magnitude);
    }
}
