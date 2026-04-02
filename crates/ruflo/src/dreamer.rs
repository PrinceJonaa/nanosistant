//! Dreamer — offline batch analysis agent.
//!
//! The Dreamer analyzes L1 episodic traces, classifies failures using
//! the MAST taxonomy, and produces a typed JSON report. It NEVER writes
//! to memory, dispatches agents, or modifies config directly.
//! DreamerApplier (deterministic Rust) reads the report and acts.

use serde::{Deserialize, Serialize};

use crate::memory::{L1Event, LessonInstruction, L3PatchProposal};

// Re-export LessonCard for use by DreamerApplier — we need its fields to
// construct a memory::LessonCard from a DreamerLessonCard.
pub use crate::memory::LessonCard;

// Re-export OperatorRuleProposal from nstn-common so callers don't need
// to depend on nstn-common directly.
pub use nstn_common::function_proposal::OperatorRuleProposal;

// ═══════════════════════════════════════
// Dreamer Input
// ═══════════════════════════════════════

/// The Dreamer's input batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamerInput {
    pub batch_window: String,
    pub total_episodes: usize,
    pub session_count: usize,
    pub system_mode: SystemMode,
    pub soul_md_hash: String,
    pub prior_lesson_ids: Vec<String>,
    pub episodes: Vec<L1Event>,
}

// ═══════════════════════════════════════
// System Mode
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SystemMode {
    /// ruflo unverified, static weights, or HashEmbedding still in use.
    /// Lesson cards only. No L3 proposals. No dispatch.
    ReadOnly,
    /// All preconditions met. Full dreaming.
    Active,
}

// ═══════════════════════════════════════
// Failure Classification
// ═══════════════════════════════════════

/// MAST failure classification for a single episode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureClassification {
    pub episode_id: String,
    pub primary_class: String,    // "FC1.1", "FC1.2", etc.
    pub secondary_class: Option<String>,
    pub galileo_pattern: Option<String>,  // "G1", "G2", etc.
    pub evidence: String,
    pub confidence: ClassificationConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClassificationConfidence { High, Medium, Low, Unclassified }

// ═══════════════════════════════════════
// Dreamer Lesson Card
// ═══════════════════════════════════════

/// A lesson card as output by the Dreamer (structured for DreamerApplier).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamerLessonCard {
    pub id: String,
    pub task_types: Vec<String>,
    pub primary_class: String,
    pub galileo_pattern: Option<String>,
    pub confidence: ClassificationConfidence,
    pub supporting_episodes: Vec<String>,
    pub contradicts_prior: Option<String>,
    pub supersedes_prior: Option<String>,
    pub situation: String,
    pub what_happened: String,
    pub instruction: LessonInstruction,
    pub verifiable_signal: String,
    pub orchestrator_task: Option<OrchestratorTask>,
}

// ═══════════════════════════════════════
// Orchestrator Task
// ═══════════════════════════════════════

/// A task the Dreamer wants the Orchestrator to dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorTask {
    pub task_id: String,
    pub agent_role: String,  // "coder", "config", "memory_writer"
    pub task_description: String,
    pub context: TaskContext,
    pub expected_output: String,
    pub verification_step: String,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub lesson_card_id: Option<String>,
    pub l3_patch_id: Option<String>,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Priority { High, Medium, Low }

// ═══════════════════════════════════════
// Tool Description Fix
// ═══════════════════════════════════════

/// Tool description fix suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptionFix {
    pub tool_name: String,
    pub current_description: String,
    pub proposed_description: String,
    pub reason: String,
    pub supporting_episodes: Vec<String>,
}

// ═══════════════════════════════════════
// Routing Weight Hint
// ═══════════════════════════════════════

/// Routing weight hint from dreaming analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingWeightHint {
    pub task_type: String,
    pub route_stage: String,
    pub direction: String,  // "increase" or "decrease"
    pub magnitude: String,  // "small", "medium", "large"
    pub reason: String,
    pub supporting_episodes: Vec<String>,
}

// ═══════════════════════════════════════
// Health Signal
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthSignal { Healthy, Degraded, Critical }

// ═══════════════════════════════════════
// Supporting Report Types
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub clean_success: usize,
    pub qualified_success: usize,
    pub failure_partial: usize,
    pub unsafe_blocked: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LensAnalysis {
    pub by_tool: Option<String>,
    pub by_agent_role: Option<String>,
    pub by_memory_usage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsufficientEvidence {
    pub episode_id: String,
    pub reason: String,
}

// ═══════════════════════════════════════
// Dreaming Report (primary output type)
// ═══════════════════════════════════════

/// The complete dreaming report — the Dreamer's only output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamingReport {
    pub dream_id: String,
    pub batch_window: String,
    pub system_mode: SystemMode,
    pub soul_md_hash_verified: bool,
    pub soul_md_changed: bool,
    pub partition: Partition,
    pub health_signal: HealthSignal,
    pub pattern_statement: String,
    pub failure_classifications: Vec<FailureClassification>,
    pub lens_analysis: LensAnalysis,
    pub lesson_cards: Vec<DreamerLessonCard>,
    pub l3_patch_proposals: Vec<L3PatchProposal>,
    pub tool_description_fixes: Vec<ToolDescriptionFix>,
    pub routing_weight_hints: Vec<RoutingWeightHint>,
    pub orchestrator_dispatch: Vec<OrchestratorTask>,
    pub insufficient_evidence_notes: Vec<InsufficientEvidence>,
    pub dreamer_confidence: ClassificationConfidence,
    pub suggested_next_batch_focus: String,
    pub read_only_suppressions: Vec<String>,
    /// Operator rule proposals generated when the Dreamer detects repeatable
    /// patterns that are better expressed as TOML rules than compiled Rust.
    ///
    /// These are generated even in `ReadOnly` mode because they require only
    /// operator review, not agent dispatch.  They are queued as `Pending` until
    /// an operator flips `approved = true` in the generated `rules.toml`.
    #[serde(default)]
    pub operator_rule_proposals: Vec<OperatorRuleProposal>,
}

// ═══════════════════════════════════════
// Validation
// ═══════════════════════════════════════

/// Validate a dreaming report for structural correctness.
pub fn validate_dreaming_report(report: &DreamingReport) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Max 5 lesson cards
    if report.lesson_cards.len() > 5 {
        errors.push(format!("Max 5 lesson cards, got {}", report.lesson_cards.len()));
    }

    // In read_only mode: no L3 proposals, no dispatch
    if report.system_mode == SystemMode::ReadOnly {
        if !report.l3_patch_proposals.is_empty() {
            errors.push("L3 proposals not allowed in read_only mode".into());
        }
        if !report.orchestrator_dispatch.is_empty() {
            errors.push("Orchestrator dispatch not allowed in read_only mode".into());
        }
    }

    // Every lesson card must cite episodes
    for lc in &report.lesson_cards {
        if lc.supporting_episodes.is_empty() {
            errors.push(format!("Lesson card {} has no supporting episodes", lc.id));
        }
    }

    // Health signal validation
    let total = report.partition.clean_success
        + report.partition.qualified_success
        + report.partition.failure_partial
        + report.partition.unsafe_blocked;
    if total > 0 {
        let failure_rate = (report.partition.failure_partial + report.partition.unsafe_blocked)
            as f64
            / total as f64;
        let has_g_patterns = report
            .failure_classifications
            .iter()
            .any(|f| f.galileo_pattern.is_some());
        match report.health_signal {
            HealthSignal::Healthy if failure_rate > 0.15 || has_g_patterns => {
                errors.push(
                    "Health signal is HEALTHY but failure rate > 15% or G-patterns present".into(),
                );
            }
            _ => {}
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

// ═══════════════════════════════════════
// LessonCard conversion helpers
// ═══════════════════════════════════════

impl DreamerLessonCard {
    /// Convert a confidence enum to the f64 used by `memory::LessonCard`.
    pub fn confidence_f64(&self) -> f64 {
        match self.confidence {
            ClassificationConfidence::High => 0.9,
            ClassificationConfidence::Medium => 0.6,
            ClassificationConfidence::Low => 0.35,
            ClassificationConfidence::Unclassified => 0.1,
        }
    }
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{LessonInstruction, ProposedChange, RiskLevel};

    // ── helpers ──────────────────────────────────────────────────────────────

    fn base_partition(failure_partial: usize, unsafe_blocked: usize) -> Partition {
        Partition {
            clean_success: 10,
            qualified_success: 2,
            failure_partial,
            unsafe_blocked,
        }
    }

    fn lesson_card_with_episodes(id: &str, episodes: Vec<String>) -> DreamerLessonCard {
        DreamerLessonCard {
            id: id.to_string(),
            task_types: vec!["FileOp".to_string()],
            primary_class: "FC1.1".to_string(),
            galileo_pattern: None,
            confidence: ClassificationConfidence::High,
            supporting_episodes: episodes,
            contradicts_prior: None,
            supersedes_prior: None,
            situation: "situation".to_string(),
            what_happened: "something happened".to_string(),
            instruction: LessonInstruction {
                trigger_condition: "trigger".to_string(),
                required_action: "act".to_string(),
                check_before: None,
                check_after: None,
            },
            verifiable_signal: "signal".to_string(),
            orchestrator_task: None,
        }
    }

    fn valid_report() -> DreamingReport {
        DreamingReport {
            dream_id: "dream-001".to_string(),
            batch_window: "2026-04-01".to_string(),
            system_mode: SystemMode::Active,
            soul_md_hash_verified: true,
            soul_md_changed: false,
            partition: base_partition(1, 0),
            health_signal: HealthSignal::Healthy,
            pattern_statement: "Normal operation".to_string(),
            failure_classifications: vec![],
            lens_analysis: LensAnalysis {
                by_tool: None,
                by_agent_role: None,
                by_memory_usage: None,
            },
            lesson_cards: vec![lesson_card_with_episodes("lc-1", vec!["ep-1".to_string()])],
            l3_patch_proposals: vec![],
            tool_description_fixes: vec![],
            routing_weight_hints: vec![],
            orchestrator_dispatch: vec![],
            insufficient_evidence_notes: vec![],
            dreamer_confidence: ClassificationConfidence::High,
            suggested_next_batch_focus: "code_edit failures".to_string(),
            read_only_suppressions: vec![],
            operator_rule_proposals: vec![],
        }
    }

    // ── validation passes ────────────────────────────────────────────────────

    #[test]
    fn valid_report_passes() {
        let report = valid_report();
        assert!(validate_dreaming_report(&report).is_ok());
    }

    #[test]
    fn empty_lesson_cards_passes() {
        let mut report = valid_report();
        report.lesson_cards.clear();
        assert!(validate_dreaming_report(&report).is_ok());
    }

    // ── lesson card limits ───────────────────────────────────────────────────

    #[test]
    fn too_many_lesson_cards_fails() {
        let mut report = valid_report();
        for i in 0..6 {
            report.lesson_cards.push(lesson_card_with_episodes(
                &format!("lc-{i}"),
                vec!["ep-x".to_string()],
            ));
        }
        let result = validate_dreaming_report(&report);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Max 5 lesson cards")));
    }

    #[test]
    fn exactly_five_lesson_cards_passes() {
        let mut report = valid_report();
        report.lesson_cards.clear();
        for i in 0..5 {
            report.lesson_cards.push(lesson_card_with_episodes(
                &format!("lc-{i}"),
                vec!["ep-x".to_string()],
            ));
        }
        assert!(validate_dreaming_report(&report).is_ok());
    }

    // ── lesson card must cite episodes ───────────────────────────────────────

    #[test]
    fn lesson_card_with_no_episodes_fails() {
        let mut report = valid_report();
        report.lesson_cards = vec![lesson_card_with_episodes("lc-empty", vec![])];
        let result = validate_dreaming_report(&report);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("no supporting episodes")));
    }

    // ── read_only mode constraints ───────────────────────────────────────────

    #[test]
    fn read_only_with_l3_patches_fails() {
        let mut report = valid_report();
        report.system_mode = SystemMode::ReadOnly;
        report.l3_patch_proposals = vec![L3PatchProposal::new(
            "some.config",
            RiskLevel::Low,
            "current",
            ProposedChange { before: "a".to_string(), after: "b".to_string() },
            "test",
            "rollback",
        )];
        let result = validate_dreaming_report(&report);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("L3 proposals not allowed")));
    }

    #[test]
    fn read_only_with_dispatch_fails() {
        let mut report = valid_report();
        report.system_mode = SystemMode::ReadOnly;
        report.orchestrator_dispatch = vec![OrchestratorTask {
            task_id: "t1".to_string(),
            agent_role: "coder".to_string(),
            task_description: "fix it".to_string(),
            context: TaskContext {
                lesson_card_id: None,
                l3_patch_id: None,
                target: "main.rs".to_string(),
            },
            expected_output: "compiled".to_string(),
            verification_step: "cargo build".to_string(),
            priority: Priority::Low,
        }];
        let result = validate_dreaming_report(&report);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Orchestrator dispatch not allowed")));
    }

    #[test]
    fn read_only_with_only_lesson_cards_passes() {
        let mut report = valid_report();
        report.system_mode = SystemMode::ReadOnly;
        // No L3 patches, no dispatch — should pass
        assert!(validate_dreaming_report(&report).is_ok());
    }

    // ── health signal validation ─────────────────────────────────────────────

    #[test]
    fn healthy_signal_with_high_failure_rate_fails() {
        let mut report = valid_report();
        // 5 failures out of 10 total = 50% failure rate — must not be HEALTHY
        report.partition = Partition {
            clean_success: 5,
            qualified_success: 0,
            failure_partial: 5,
            unsafe_blocked: 0,
        };
        report.health_signal = HealthSignal::Healthy;
        let result = validate_dreaming_report(&report);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("failure rate > 15%")));
    }

    #[test]
    fn healthy_signal_with_galileo_patterns_fails() {
        let mut report = valid_report();
        // Low failure rate but G-pattern present
        report.partition = base_partition(1, 0); // 1/13 ≈ 7.7%
        report.health_signal = HealthSignal::Healthy;
        report.failure_classifications = vec![FailureClassification {
            episode_id: "ep-1".to_string(),
            primary_class: "FC1.1".to_string(),
            secondary_class: None,
            galileo_pattern: Some("G1".to_string()),
            evidence: "repeated pattern".to_string(),
            confidence: ClassificationConfidence::High,
        }];
        let result = validate_dreaming_report(&report);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("G-patterns present")));
    }

    #[test]
    fn degraded_signal_with_high_failure_rate_passes() {
        let mut report = valid_report();
        report.partition = Partition {
            clean_success: 5,
            qualified_success: 0,
            failure_partial: 5,
            unsafe_blocked: 0,
        };
        report.health_signal = HealthSignal::Degraded;
        assert!(validate_dreaming_report(&report).is_ok());
    }

    #[test]
    fn healthy_signal_at_exactly_15_percent_fails() {
        let mut report = valid_report();
        // Exactly 15% = 0.15 — NOT > 0.15, so should not fail on rate alone.
        // Using 3 failures out of 20 total = 15% exactly → should pass.
        report.partition = Partition {
            clean_success: 17,
            qualified_success: 0,
            failure_partial: 3,
            unsafe_blocked: 0,
        };
        report.health_signal = HealthSignal::Healthy;
        assert!(validate_dreaming_report(&report).is_ok());
    }

    #[test]
    fn zero_total_episodes_skips_health_check() {
        let mut report = valid_report();
        report.partition = Partition {
            clean_success: 0,
            qualified_success: 0,
            failure_partial: 0,
            unsafe_blocked: 0,
        };
        report.health_signal = HealthSignal::Healthy;
        // No total episodes — health check is skipped, should pass
        assert!(validate_dreaming_report(&report).is_ok());
    }

    // ── multiple errors accumulated ──────────────────────────────────────────

    #[test]
    fn multiple_violations_all_reported() {
        let mut report = valid_report();
        // Too many lesson cards
        for i in 0..6 {
            report.lesson_cards.push(lesson_card_with_episodes(
                &format!("extra-{i}"),
                vec!["ep".to_string()],
            ));
        }
        // L3 patches in read_only mode
        report.system_mode = SystemMode::ReadOnly;
        report.l3_patch_proposals = vec![L3PatchProposal::new(
            "k",
            RiskLevel::Low,
            "curr",
            ProposedChange { before: "a".to_string(), after: "b".to_string() },
            "test",
            "rollback",
        )];
        let result = validate_dreaming_report(&report);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.len() >= 2);
    }

    // ── confidence conversion ─────────────────────────────────────────────────

    #[test]
    fn confidence_high_maps_to_0_9() {
        let lc = lesson_card_with_episodes("x", vec!["ep".to_string()]);
        assert_eq!(lc.confidence_f64(), 0.9);
    }

    #[test]
    fn confidence_unclassified_maps_to_0_1() {
        let mut lc = lesson_card_with_episodes("x", vec!["ep".to_string()]);
        lc.confidence = ClassificationConfidence::Unclassified;
        assert_eq!(lc.confidence_f64(), 0.1);
    }

    // ── operator_rule_proposals ───────────────────────────────────────────

    use chrono::Utc;
    use nstn_common::function_proposal::{OperatorRuleProposal, ProposalStatus, ProposedFormula};

    fn sample_operator_proposal(id: &str) -> OperatorRuleProposal {
        OperatorRuleProposal {
            id: id.to_string(),
            rule_id: "bpm-bar".to_string(),
            description: "BPM bar duration rule".to_string(),
            trigger_keywords: vec!["bpm".to_string(), "bar".to_string()],
            semantic_hint: None,
            formula: ProposedFormula::Arithmetic {
                expr: "60 / x * 4".to_string(),
                variables: vec!["x".to_string()],
            },
            confidence: 0.92,
            supporting_episodes: vec!["ep-1".to_string()],
            example_inputs: vec!["bar at 120 bpm".to_string()],
            example_outputs: vec!["2.000 seconds".to_string()],
            proposed_at: Utc::now(),
            status: ProposalStatus::Pending,
        }
    }

    #[test]
    fn operator_rule_proposals_default_empty() {
        let report = valid_report();
        assert!(report.operator_rule_proposals.is_empty());
    }

    #[test]
    fn operator_rule_proposals_allowed_in_read_only_mode() {
        let mut report = valid_report();
        report.system_mode = SystemMode::ReadOnly;
        report.operator_rule_proposals = vec![sample_operator_proposal("orp-1")];
        // validate_dreaming_report does NOT block operator_rule_proposals in ReadOnly
        assert!(validate_dreaming_report(&report).is_ok());
    }

    #[test]
    fn operator_rule_proposals_allowed_in_active_mode() {
        let mut report = valid_report();
        report.operator_rule_proposals = vec![
            sample_operator_proposal("orp-1"),
            sample_operator_proposal("orp-2"),
        ];
        assert!(validate_dreaming_report(&report).is_ok());
        assert_eq!(report.operator_rule_proposals.len(), 2);
    }

    #[test]
    fn operator_rule_proposal_roundtrips_json() {
        let mut report = valid_report();
        report.operator_rule_proposals = vec![sample_operator_proposal("orp-xyz")];
        let json = serde_json::to_string(&report).unwrap();
        let back: DreamingReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.operator_rule_proposals.len(), 1);
        assert_eq!(back.operator_rule_proposals[0].id, "orp-xyz");
    }

    #[test]
    fn old_report_without_operator_proposals_deserializes() {
        // Simulate a report serialized before the field was added
        // (serde(default) must handle this gracefully)
        let json = r#"{
            "dream_id": "d1",
            "batch_window": "2026-04-01",
            "system_mode": "Active",
            "soul_md_hash_verified": true,
            "soul_md_changed": false,
            "partition": {"clean_success":5,"qualified_success":0,"failure_partial":0,"unsafe_blocked":0},
            "health_signal": "Healthy",
            "pattern_statement": "ok",
            "failure_classifications": [],
            "lens_analysis": {"by_tool":null,"by_agent_role":null,"by_memory_usage":null},
            "lesson_cards": [],
            "l3_patch_proposals": [],
            "tool_description_fixes": [],
            "routing_weight_hints": [],
            "orchestrator_dispatch": [],
            "insufficient_evidence_notes": [],
            "dreamer_confidence": "High",
            "suggested_next_batch_focus": "nothing",
            "read_only_suppressions": []
        }"#;
        let report: DreamingReport = serde_json::from_str(json).unwrap();
        // operator_rule_proposals should default to empty vec
        assert!(report.operator_rule_proposals.is_empty());
    }
}
