//! DreamerApplier — deterministic code that applies Dreamer outputs.
//!
//! Reads a `DreamingReport`. Inserts lesson cards into L2, queues L3 patches,
//! records routing weight hints, and surfaces reports to Jona.
//!
//! ## Operator rule proposal handling
//!
//! When the report contains `operator_rule_proposals`, the Applier:
//! - Writes **approved** proposals to `.nanosistant/operator/functions.toml`
//!   (appending TOML rule blocks ready for the operator to activate).
//! - Queues **unapproved/pending** proposals in
//!   `.nanosistant/operator/pending_proposals.json` for operator review.
//!
//! NEVER calls an LLM. NEVER interprets prose. Only reads typed fields.

use std::path::Path;

use crate::dreamer::{DreamingReport, SystemMode};
use crate::memory::{LessonCard, MemorySystem};
use nstn_common::function_proposal::{OperatorRuleProposal, ProposalStatus};

// ═══════════════════════════════════════
// Apply Result
// ═══════════════════════════════════════

/// Result of applying a dreaming report.
#[derive(Debug, Clone, Default)]
pub struct ApplyResult {
    pub lessons_inserted: usize,
    pub lessons_skipped: usize,
    pub l3_patches_queued: usize,
    pub weight_hints_recorded: usize,
    pub dispatch_tasks: usize,
    /// Number of approved operator rule proposals written to `functions.toml`.
    pub operator_rules_written: usize,
    /// Number of pending operator rule proposals queued in `pending_proposals.json`.
    pub operator_rules_pending: usize,
    pub errors: Vec<String>,
}

// ═══════════════════════════════════════
// DreamerApplier
// ═══════════════════════════════════════

pub struct DreamerApplier;

impl DreamerApplier {
    /// Apply a validated dreaming report to the memory system.
    ///
    /// This function is deterministic — it never calls an LLM. It maps typed
    /// fields from the report directly onto the memory tiers.
    pub fn apply(report: &DreamingReport, memory: &mut MemorySystem) -> ApplyResult {
        Self::apply_with_operator_dir(report, memory, None)
    }

    /// Apply with an explicit operator directory for rule proposal persistence.
    ///
    /// `operator_dir` is the path to `.nanosistant/operator/` (or equivalent).
    /// When `None`, operator rule proposals are counted but not persisted.
    pub fn apply_with_operator_dir(
        report: &DreamingReport,
        memory: &mut MemorySystem,
        operator_dir: Option<&Path>,
    ) -> ApplyResult {
        let mut result = ApplyResult::default();

        // 1. Insert lesson cards into L2
        for dlc in &report.lesson_cards {
            let lesson = LessonCard::from_dreamer(dlc);
            memory.l2.insert(lesson);
            result.lessons_inserted += 1;
        }

        // 2. Queue L3 patches (only in active mode)
        if report.system_mode == SystemMode::Active {
            for patch in &report.l3_patch_proposals {
                memory.l3.queue_patch(patch.clone());
                result.l3_patches_queued += 1;
            }
        }

        // 3. Record routing weight hints
        //    (stored as count; the hint structs are surfaced to Orchestrator
        //     separately via ExternalMirror)
        result.weight_hints_recorded = report.routing_weight_hints.len();

        // 4. Count dispatch tasks
        //    (Orchestrator handles these, not DreamerApplier)
        result.dispatch_tasks = report.orchestrator_dispatch.len();

        // 5. Handle operator rule proposals
        if !report.operator_rule_proposals.is_empty() {
            let (written, pending, errors) =
                apply_operator_rule_proposals(&report.operator_rule_proposals, operator_dir);
            result.operator_rules_written = written;
            result.operator_rules_pending = pending;
            result.errors.extend(errors);
        }

        result
    }
}

// ═══════════════════════════════════════
// Operator rule proposal persistence
// ═══════════════════════════════════════

/// Process operator rule proposals: write approved ones to `functions.toml`,
/// queue pending ones in `pending_proposals.json`.
///
/// Returns `(written, pending, errors)`.
fn apply_operator_rule_proposals(
    proposals: &[OperatorRuleProposal],
    operator_dir: Option<&Path>,
) -> (usize, usize, Vec<String>) {
    let mut written = 0usize;
    let mut pending_list: Vec<&OperatorRuleProposal> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let approved: Vec<&OperatorRuleProposal> = proposals
        .iter()
        .filter(|p| p.status == ProposalStatus::Approved)
        .collect();

    let pending: Vec<&OperatorRuleProposal> = proposals
        .iter()
        .filter(|p| p.status == ProposalStatus::Pending)
        .collect();

    pending_list.extend(&pending);

    if let Some(dir) = operator_dir {
        // Write approved proposals to functions.toml (append TOML rule blocks)
        if !approved.is_empty() {
            let functions_path = dir.join("functions.toml");
            match write_approved_rules(&functions_path, &approved) {
                Ok(n) => written = n,
                Err(e) => errors.push(format!("functions.toml write error: {e}")),
            }
        }

        // Queue pending proposals to pending_proposals.json
        if !pending_list.is_empty() {
            let pending_path = dir.join("pending_proposals.json");
            match write_pending_proposals(&pending_path, &pending_list) {
                Ok(_) => {}
                Err(e) => errors.push(format!("pending_proposals.json write error: {e}")),
            }
        }
    } else {
        // No directory provided — just count
        written = approved.len();
    }

    (written, pending_list.len(), errors)
}

/// Render approved proposals as TOML rule blocks and append to `functions.toml`.
fn write_approved_rules(
    path: &Path,
    proposals: &[&OperatorRuleProposal],
) -> Result<usize, String> {
    use std::fmt::Write as FmtWrite;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut toml_block = String::new();
    for p in proposals {
        // Render a minimal [[rules]] TOML block
        let keywords_toml = p
            .trigger_keywords
            .iter()
            .map(|k| format!("\"{k}\""))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(toml_block, "\n[[rules]]").map_err(|e| e.to_string())?;
        writeln!(toml_block, "id = \"{}\"", p.rule_id).map_err(|e| e.to_string())?;
        writeln!(toml_block, "description = \"{}\"", p.description).map_err(|e| e.to_string())?;
        writeln!(toml_block, "trigger_keywords = [{keywords_toml}]").map_err(|e| e.to_string())?;
        writeln!(toml_block, "confidence = {:.2}", p.confidence).map_err(|e| e.to_string())?;
        writeln!(toml_block, "proposed_by = \"dreamer\"").map_err(|e| e.to_string())?;
        writeln!(toml_block, "approved = false  # operator must flip to true").map_err(|e| e.to_string())?;
        writeln!(toml_block, "examples = []").map_err(|e| e.to_string())?;
        // Render formula
        match &p.formula {
            nstn_common::function_proposal::ProposedFormula::Static { response } => {
                writeln!(toml_block, "\n[rules.formula]").map_err(|e| e.to_string())?;
                writeln!(toml_block, "Static = {{ response = \"{response}\" }}").map_err(|e| e.to_string())?;
            }
            nstn_common::function_proposal::ProposedFormula::Arithmetic { expr, variables } => {
                let vars_toml = variables
                    .iter()
                    .map(|v| format!("\"{v}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                writeln!(toml_block, "\n[rules.formula]").map_err(|e| e.to_string())?;
                writeln!(toml_block, "Arithmetic = {{ expr = \"{expr}\", variables = [{vars_toml}] }}").map_err(|e| e.to_string())?;
            }
            nstn_common::function_proposal::ProposedFormula::Template { template, slots } => {
                let slots_toml = slots
                    .iter()
                    .map(|s| format!("\"{s}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                writeln!(toml_block, "\n[rules.formula]").map_err(|e| e.to_string())?;
                writeln!(toml_block, "Template = {{ template = \"{template}\", slots = [{slots_toml}] }}").map_err(|e| e.to_string())?;
            }
            _ => {
                writeln!(toml_block, "# formula: see pending_proposals.json for full spec").map_err(|e| e.to_string())?;
            }
        }
    }

    // Append to existing file or create new
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(toml_block.as_bytes()).map_err(|e| e.to_string())?;

    Ok(proposals.len())
}

/// Serialise pending proposals to JSON for operator review.
fn write_pending_proposals(
    path: &Path,
    proposals: &[&OperatorRuleProposal],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Load existing pending proposals if any, then merge
    let mut existing: Vec<OperatorRuleProposal> = if path.exists() {
        let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&json).unwrap_or_default()
    } else {
        Vec::new()
    };

    for p in proposals {
        // Deduplicate by id
        if !existing.iter().any(|e| e.id == p.id) {
            existing.push((*p).clone());
        }
    }

    let json = serde_json::to_string_pretty(&existing).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::dreamer::{
        ClassificationConfidence, DreamerLessonCard, DreamingReport, HealthSignal, InsufficientEvidence,
        LensAnalysis, OrchestratorTask, Partition, Priority, RoutingWeightHint, SystemMode,
        TaskContext, ToolDescriptionFix,
    };
    use crate::memory::{
        LessonInstruction, L3PatchProposal, MemorySystem, ProposedChange, RiskLevel,
    };

    // ── helpers ──────────────────────────────────────────────────────────────

    fn temp_memory() -> (TempDir, MemorySystem) {
        let dir = TempDir::new().expect("tempdir");
        let ms = MemorySystem::new(dir.path());
        (dir, ms)
    }

    fn make_lesson_card(id: &str) -> DreamerLessonCard {
        DreamerLessonCard {
            id: id.to_string(),
            task_types: vec!["CodeEdit".to_string()],
            primary_class: "FC1.1".to_string(),
            galileo_pattern: None,
            confidence: ClassificationConfidence::High,
            supporting_episodes: vec!["ep-001".to_string()],
            contradicts_prior: None,
            supersedes_prior: None,
            situation: "agent looped on code edit".to_string(),
            what_happened: "loop count exceeded 3".to_string(),
            instruction: LessonInstruction {
                trigger_condition: "loop_count > 3".to_string(),
                required_action: "abort and surface to Jona".to_string(),
                check_before: None,
                check_after: Some("watchdog did not fire again".to_string()),
            },
            verifiable_signal: "watchdog silent on next run".to_string(),
            orchestrator_task: None,
        }
    }

    fn make_patch() -> L3PatchProposal {
        L3PatchProposal::new(
            "agent.loop_limit",
            RiskLevel::Low,
            "loop_limit = 5",
            ProposedChange {
                before: "loop_limit = 5".to_string(),
                after: "loop_limit = 3".to_string(),
            },
            "watchdog does not fire within 3 loops",
            "revert to loop_limit = 5",
        )
    }

    fn minimal_report(system_mode: SystemMode) -> DreamingReport {
        DreamingReport {
            dream_id: "dream-test-001".to_string(),
            batch_window: "2026-04-01".to_string(),
            system_mode,
            soul_md_hash_verified: true,
            soul_md_changed: false,
            partition: Partition {
                clean_success: 8,
                qualified_success: 2,
                failure_partial: 0,
                unsafe_blocked: 0,
            },
            health_signal: HealthSignal::Healthy,
            pattern_statement: "clean batch".to_string(),
            failure_classifications: vec![],
            lens_analysis: LensAnalysis {
                by_tool: None,
                by_agent_role: None,
                by_memory_usage: None,
            },
            lesson_cards: vec![],
            l3_patch_proposals: vec![],
            tool_description_fixes: vec![],
            routing_weight_hints: vec![],
            orchestrator_dispatch: vec![],
            insufficient_evidence_notes: vec![],
            dreamer_confidence: ClassificationConfidence::High,
            suggested_next_batch_focus: "web search failures".to_string(),
            read_only_suppressions: vec![],
            operator_rule_proposals: vec![],
        }
    }

    // ── lesson card insertion ─────────────────────────────────────────────────

    #[test]
    fn inserts_lesson_cards_into_l2() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.lesson_cards = vec![make_lesson_card("lc-a"), make_lesson_card("lc-b")];

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.lessons_inserted, 2);
        assert_eq!(result.lessons_skipped, 0);
        assert_eq!(ms.l2.active_lessons().len(), 2);
    }

    #[test]
    fn no_lesson_cards_yields_zero_inserted() {
        let (_dir, mut ms) = temp_memory();
        let report = minimal_report(SystemMode::Active);

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.lessons_inserted, 0);
        assert_eq!(ms.l2.active_lessons().len(), 0);
    }

    #[test]
    fn lesson_card_id_preserved_in_l2() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.lesson_cards = vec![make_lesson_card("fixed-id-123")];

        DreamerApplier::apply(&report, &mut ms);

        let lessons = ms.l2.active_lessons();
        assert_eq!(lessons[0].id, "fixed-id-123");
    }

    #[test]
    fn lesson_card_confidence_converted_correctly() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.lesson_cards = vec![make_lesson_card("lc-conf")];

        DreamerApplier::apply(&report, &mut ms);

        let lessons = ms.l2.active_lessons();
        // High confidence → 0.9
        assert!((lessons[0].confidence - 0.9).abs() < f64::EPSILON);
    }

    // ── L3 patch queuing ──────────────────────────────────────────────────────

    #[test]
    fn active_mode_queues_l3_patches() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.l3_patch_proposals = vec![make_patch()];

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.l3_patches_queued, 1);
        assert_eq!(ms.l3.pending_patches().len(), 1);
    }

    #[test]
    fn read_only_mode_does_not_queue_l3_patches() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::ReadOnly);
        // In a real system the Dreamer would not emit L3 patches in ReadOnly,
        // but we test that the Applier also enforces this.
        // We bypass validate_dreaming_report here intentionally.
        report.l3_patch_proposals = vec![make_patch()];

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.l3_patches_queued, 0);
        assert_eq!(ms.l3.pending_patches().len(), 0);
    }

    #[test]
    fn multiple_l3_patches_all_queued_in_active_mode() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.l3_patch_proposals = vec![make_patch(), make_patch()];

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.l3_patches_queued, 2);
        assert_eq!(ms.l3.pending_patches().len(), 2);
    }

    // ── weight hints ──────────────────────────────────────────────────────────

    #[test]
    fn weight_hints_counted() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.routing_weight_hints = vec![
            RoutingWeightHint {
                task_type: "CodeEdit".to_string(),
                route_stage: "tier1".to_string(),
                direction: "increase".to_string(),
                magnitude: "small".to_string(),
                reason: "better results with tier1".to_string(),
                supporting_episodes: vec!["ep-1".to_string()],
            },
            RoutingWeightHint {
                task_type: "WebSearch".to_string(),
                route_stage: "tier2".to_string(),
                direction: "decrease".to_string(),
                magnitude: "medium".to_string(),
                reason: "tier2 slower for searches".to_string(),
                supporting_episodes: vec!["ep-2".to_string()],
            },
        ];

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.weight_hints_recorded, 2);
    }

    #[test]
    fn no_weight_hints_recorded_as_zero() {
        let (_dir, mut ms) = temp_memory();
        let report = minimal_report(SystemMode::Active);

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.weight_hints_recorded, 0);
    }

    // ── dispatch tasks ────────────────────────────────────────────────────────

    #[test]
    fn dispatch_tasks_counted_not_executed() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.orchestrator_dispatch = vec![OrchestratorTask {
            task_id: "t-001".to_string(),
            agent_role: "coder".to_string(),
            task_description: "fix the loop issue".to_string(),
            context: TaskContext {
                lesson_card_id: Some("lc-a".to_string()),
                l3_patch_id: None,
                target: "src/agent.rs".to_string(),
            },
            expected_output: "no more loop failures".to_string(),
            verification_step: "cargo test".to_string(),
            priority: Priority::High,
        }];

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.dispatch_tasks, 1);
        // DreamerApplier does not execute dispatch — only counts
        assert!(result.errors.is_empty());
    }

    // ── no errors on clean report ─────────────────────────────────────────────

    #[test]
    fn apply_clean_report_produces_no_errors() {
        let (_dir, mut ms) = temp_memory();
        let report = minimal_report(SystemMode::Active);

        let result = DreamerApplier::apply(&report, &mut ms);

        assert!(result.errors.is_empty());
    }

    // ── combined: lesson + patch + hints ─────────────────────────────────────

    #[test]
    fn full_active_report_applies_all_components() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.lesson_cards = vec![make_lesson_card("lc-full")];
        report.l3_patch_proposals = vec![make_patch()];
        report.routing_weight_hints = vec![RoutingWeightHint {
            task_type: "FileOp".to_string(),
            route_stage: "tier1".to_string(),
            direction: "increase".to_string(),
            magnitude: "large".to_string(),
            reason: "consistently best".to_string(),
            supporting_episodes: vec!["ep-99".to_string()],
        }];

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.lessons_inserted, 1);
        assert_eq!(result.l3_patches_queued, 1);
        assert_eq!(result.weight_hints_recorded, 1);
        assert!(result.errors.is_empty());
    }

    // ── operator rule proposals ───────────────────────────────────────────

    use chrono::Utc;
    use nstn_common::function_proposal::{OperatorRuleProposal, ProposalStatus, ProposedFormula};

    fn make_operator_proposal(id: &str, status: ProposalStatus) -> OperatorRuleProposal {
        OperatorRuleProposal {
            id: id.to_string(),
            rule_id: format!("rule-{id}"),
            description: format!("Test rule {id}"),
            trigger_keywords: vec!["bpm".to_string()],
            semantic_hint: None,
            formula: ProposedFormula::Static {
                response: format!("response-{id}"),
            },
            confidence: 0.85,
            supporting_episodes: vec!["ep-1".to_string()],
            example_inputs: vec!["test query".to_string()],
            example_outputs: vec![format!("response-{id}")],
            proposed_at: Utc::now(),
            status,
        }
    }

    #[test]
    fn pending_proposals_are_counted() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.operator_rule_proposals = vec![
            make_operator_proposal("p1", ProposalStatus::Pending),
            make_operator_proposal("p2", ProposalStatus::Pending),
        ];

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.operator_rules_pending, 2);
        assert_eq!(result.operator_rules_written, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn approved_proposals_are_counted_without_dir() {
        let (_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.operator_rule_proposals = vec![
            make_operator_proposal("a1", ProposalStatus::Approved),
        ];

        // No operator_dir provided — approved rules are counted, not persisted
        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.operator_rules_written, 1);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn approved_proposals_written_to_functions_toml() {
        let tmp = TempDir::new().unwrap();
        let operator_dir = tmp.path().join("operator");
        let (_ms_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.operator_rule_proposals = vec![
            make_operator_proposal("a2", ProposalStatus::Approved),
        ];

        let result = DreamerApplier::apply_with_operator_dir(
            &report, &mut ms, Some(&operator_dir),
        );

        assert_eq!(result.operator_rules_written, 1);
        assert!(result.errors.is_empty());

        let functions_path = operator_dir.join("functions.toml");
        assert!(functions_path.exists());
        let content = std::fs::read_to_string(&functions_path).unwrap();
        assert!(content.contains("[[rules]]"));
        assert!(content.contains("rule-a2"));
    }

    #[test]
    fn pending_proposals_written_to_json() {
        let tmp = TempDir::new().unwrap();
        let operator_dir = tmp.path().join("operator");
        let (_ms_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.operator_rule_proposals = vec![
            make_operator_proposal("p3", ProposalStatus::Pending),
        ];

        let result = DreamerApplier::apply_with_operator_dir(
            &report, &mut ms, Some(&operator_dir),
        );

        assert_eq!(result.operator_rules_pending, 1);
        assert!(result.errors.is_empty());

        let pending_path = operator_dir.join("pending_proposals.json");
        assert!(pending_path.exists());
        let json = std::fs::read_to_string(&pending_path).unwrap();
        assert!(json.contains("p3"));
    }

    #[test]
    fn mixed_proposals_split_correctly() {
        let tmp = TempDir::new().unwrap();
        let operator_dir = tmp.path().join("operator");
        let (_ms_dir, mut ms) = temp_memory();
        let mut report = minimal_report(SystemMode::Active);
        report.operator_rule_proposals = vec![
            make_operator_proposal("ap1", ProposalStatus::Approved),
            make_operator_proposal("ap2", ProposalStatus::Approved),
            make_operator_proposal("pn1", ProposalStatus::Pending),
        ];

        let result = DreamerApplier::apply_with_operator_dir(
            &report, &mut ms, Some(&operator_dir),
        );

        assert_eq!(result.operator_rules_written, 2);
        assert_eq!(result.operator_rules_pending, 1);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn no_operator_proposals_produces_zero_counts() {
        let (_dir, mut ms) = temp_memory();
        let report = minimal_report(SystemMode::Active);

        let result = DreamerApplier::apply(&report, &mut ms);

        assert_eq!(result.operator_rules_written, 0);
        assert_eq!(result.operator_rules_pending, 0);
    }
}
