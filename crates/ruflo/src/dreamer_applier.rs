//! DreamerApplier — deterministic code that applies Dreamer outputs.
//!
//! Reads a `DreamingReport`. Inserts lesson cards into L2, queues L3 patches,
//! records routing weight hints, and surfaces reports to Jona.
//!
//! NEVER calls an LLM. NEVER interprets prose. Only reads typed fields.

use crate::dreamer::{DreamingReport, SystemMode};
use crate::memory::{LessonCard, MemorySystem};

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

        result
    }
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
}
