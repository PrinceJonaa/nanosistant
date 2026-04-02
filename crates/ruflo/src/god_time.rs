//! God-Time vs Drift-Time detection.
//!
//! Before any non-trivial action, check whether the system is acting
//! from presence (God-Time) or pattern-fill (Drift-Time).
//!
//! ## Drift signals
//!
//! 1. No clear goal in L0 (empty `current_goal`)
//! 2. Last 3+ events have the same intent (tight loop)
//! 3. Recent events are all failures with no plan change
//! 4. Working context notes mention "confused" or "unsure"

use nstn_common::typed_ir::GodTimeStatus;

use crate::memory::{L1Event, Outcome, WorkingContext};

// ═══════════════════════════════════════
// Public Types
// ═══════════════════════════════════════

/// Result of a God-Time check.
#[derive(Debug, Clone)]
pub struct GodTimeCheckResult {
    pub status: GodTimeStatus,
    pub reason: String,
    pub action_allowed: bool,
}

// ═══════════════════════════════════════
// God-Time Check
// ═══════════════════════════════════════

/// Run the God-Time check against the current working context and recent events.
///
/// Returns [`GodTimeCheckResult`] with status `GodTime` (action allowed) or
/// `DriftTime` (action blocked pending re-grounding).
///
/// Drift is detected when **any** of the following hold:
///
/// 1. `context.current_goal` is empty — no clear goal.
/// 2. The last 3 or more `recent_events` share the same `intent` — tight loop.
/// 3. The last 3 or more `recent_events` are all failure-class outcomes with no
///    change in `plan_stated` — failing without adapting.
/// 4. Any of `context.notes` contain the words `"confused"` or `"unsure"`.
#[must_use]
pub fn check_god_time(context: &WorkingContext, recent_events: &[L1Event]) -> GodTimeCheckResult {
    // ── Signal 1: no clear goal ──────────────────────────────────────────────
    if context.current_goal.trim().is_empty() {
        return GodTimeCheckResult {
            status: GodTimeStatus::DriftDetected,
            reason: "L0 has no current_goal — acting without grounding".to_string(),
            action_allowed: false,
        };
    }

    // ── Signal 4: notes contain confusion markers ────────────────────────────
    // (checked early to short-circuit expensive event scanning)
    let confusion_markers = ["confused", "unsure"];
    for note in &context.notes {
        let lower = note.to_lowercase();
        for marker in &confusion_markers {
            if lower.contains(marker) {
                return GodTimeCheckResult {
                    status: GodTimeStatus::DriftDetected,
                    reason: format!(
                        "working context note contains drift marker '{}': \"{}\"",
                        marker, note
                    ),
                    action_allowed: false,
                };
            }
        }
    }

    // ── Signal 2: last 3+ events same intent (loop) ──────────────────────────
    if recent_events.len() >= 3 {
        let tail = &recent_events[recent_events.len().saturating_sub(3)..];
        if tail.len() == 3 {
            let all_same_intent = tail.windows(2).all(|w| w[0].intent == w[1].intent);
            if all_same_intent {
                return GodTimeCheckResult {
                    status: GodTimeStatus::DriftDetected,
                    reason: format!(
                        "last {} events all share intent '{}' — possible tight loop",
                        tail.len(),
                        tail[0].intent
                    ),
                    action_allowed: false,
                };
            }
        }
    }

    // ── Signal 3: last 3+ events all failures, no plan change ───────────────
    if recent_events.len() >= 3 {
        let tail = &recent_events[recent_events.len().saturating_sub(3)..];
        let all_failures = tail.iter().all(|e| {
            matches!(
                e.outcome,
                Outcome::Failure | Outcome::Partial | Outcome::Aborted | Outcome::UnsafeBlocked
            )
        });

        if all_failures {
            // Check whether plan_stated changed across these events.
            let plans: Vec<Option<&String>> =
                tail.iter().map(|e| e.plan_stated.as_ref()).collect();
            let plan_changed = plans.windows(2).any(|w| w[0] != w[1]);

            if !plan_changed {
                return GodTimeCheckResult {
                    status: GodTimeStatus::DriftDetected,
                    reason: format!(
                        "last {} events are all failure-class outcomes with no plan change — \
                         failing without adapting",
                        tail.len()
                    ),
                    action_allowed: false,
                };
            }
        }
    }

    // ── All clear: God-Time ──────────────────────────────────────────────────
    GodTimeCheckResult {
        status: GodTimeStatus::Confirmed,
        reason: "goal is clear, no drift signals detected".to_string(),
        action_allowed: true,
    }
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use nstn_common::typed_ir::GodTimeStatus;
    use crate::memory::{L1Event, Outcome, TaskType, WorkingContext};

    fn make_event(intent: &str, outcome: Outcome, plan: Option<&str>) -> L1Event {
        let mut e = L1Event::new("session-1", "agent", TaskType::CodeEdit, intent, outcome);
        e.plan_stated = plan.map(|s| s.to_string());
        e
    }

    fn grounded_ctx() -> WorkingContext {
        WorkingContext::new("implement feature X")
    }

    // ── Signal 1: no goal ───────────────────────────────────────────────────

    #[test]
    fn drift_when_no_goal() {
        let ctx = WorkingContext::new("");
        let result = check_god_time(&ctx, &[]);
        assert_eq!(result.status, GodTimeStatus::DriftDetected);
        assert!(!result.action_allowed);
        assert!(result.reason.contains("current_goal"));
    }

    #[test]
    fn drift_when_goal_is_whitespace() {
        let ctx = WorkingContext::new("   ");
        let result = check_god_time(&ctx, &[]);
        assert_eq!(result.status, GodTimeStatus::DriftDetected);
        assert!(!result.action_allowed);
    }

    // ── Signal 4: confusion markers in notes ─────────────────────────────────

    #[test]
    fn drift_when_note_contains_confused() {
        let mut ctx = grounded_ctx();
        ctx.add_note("I am confused about which tool to call next");
        let result = check_god_time(&ctx, &[]);
        assert_eq!(result.status, GodTimeStatus::DriftDetected);
        assert!(result.reason.contains("confused"));
        assert!(!result.action_allowed);
    }

    #[test]
    fn drift_when_note_contains_unsure() {
        let mut ctx = grounded_ctx();
        ctx.add_note("Unsure whether to proceed");
        let result = check_god_time(&ctx, &[]);
        assert_eq!(result.status, GodTimeStatus::DriftDetected);
        assert!(result.reason.contains("unsure"));
        assert!(!result.action_allowed);
    }

    #[test]
    fn god_time_when_notes_are_neutral() {
        let mut ctx = grounded_ctx();
        ctx.add_note("Proceeding with compilation step");
        ctx.add_note("All checks passed");
        let result = check_god_time(&ctx, &[]);
        assert_eq!(result.status, GodTimeStatus::Confirmed);
        assert!(result.action_allowed);
    }

    // ── Signal 2: repeated intent loop ───────────────────────────────────────

    #[test]
    fn drift_when_last_three_events_same_intent() {
        let ctx = grounded_ctx();
        let events = vec![
            make_event("compile project", Outcome::Failure, None),
            make_event("compile project", Outcome::Failure, None),
            make_event("compile project", Outcome::Failure, None),
        ];
        let result = check_god_time(&ctx, &events);
        assert_eq!(result.status, GodTimeStatus::DriftDetected);
        assert!(result.reason.contains("loop"));
        assert!(!result.action_allowed);
    }

    #[test]
    fn god_time_when_intents_vary() {
        let ctx = grounded_ctx();
        let events = vec![
            make_event("compile project", Outcome::Failure, None),
            make_event("check logs", Outcome::Success, None),
            make_event("fix imports", Outcome::Success, None),
        ];
        let result = check_god_time(&ctx, &events);
        assert_eq!(result.status, GodTimeStatus::Confirmed);
        assert!(result.action_allowed);
    }

    #[test]
    fn god_time_with_fewer_than_three_events() {
        let ctx = grounded_ctx();
        let events = vec![
            make_event("compile project", Outcome::Failure, None),
            make_event("compile project", Outcome::Failure, None),
        ];
        let result = check_god_time(&ctx, &events);
        // With only 2 events, the loop detection doesn't trigger
        assert_eq!(result.status, GodTimeStatus::Confirmed);
        assert!(result.action_allowed);
    }

    // ── Signal 3: repeated failures, no plan change ───────────────────────────

    #[test]
    fn drift_when_three_failures_same_plan() {
        let ctx = grounded_ctx();
        let events = vec![
            make_event("run tests", Outcome::Failure, Some("plan-A")),
            make_event("build code", Outcome::Failure, Some("plan-A")),
            make_event("deploy", Outcome::Aborted, Some("plan-A")),
        ];
        let result = check_god_time(&ctx, &events);
        assert_eq!(result.status, GodTimeStatus::DriftDetected);
        assert!(result.reason.contains("failing without adapting"));
        assert!(!result.action_allowed);
    }

    #[test]
    fn god_time_when_failures_but_plan_changed() {
        let ctx = grounded_ctx();
        let events = vec![
            make_event("run tests", Outcome::Failure, Some("plan-A")),
            make_event("build code", Outcome::Failure, Some("plan-A")),
            make_event("deploy", Outcome::Failure, Some("plan-B")), // plan changed
        ];
        let result = check_god_time(&ctx, &events);
        assert_eq!(result.status, GodTimeStatus::Confirmed);
        assert!(result.action_allowed);
    }

    #[test]
    fn god_time_when_failures_with_no_plan_but_partial_success() {
        let ctx = grounded_ctx();
        let events = vec![
            make_event("run tests", Outcome::Failure, None),
            make_event("build code", Outcome::Success, None), // one success breaks it
            make_event("deploy", Outcome::Failure, None),
        ];
        let result = check_god_time(&ctx, &events);
        assert_eq!(result.status, GodTimeStatus::Confirmed);
        assert!(result.action_allowed);
    }

    // ── Clean context, no events ──────────────────────────────────────────────

    #[test]
    fn god_time_clean_context_no_events() {
        let ctx = grounded_ctx();
        let result = check_god_time(&ctx, &[]);
        assert_eq!(result.status, GodTimeStatus::Confirmed);
        assert!(result.action_allowed);
        assert!(result.reason.contains("clear"));
    }
}
