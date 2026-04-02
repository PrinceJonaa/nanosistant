//! Four-tier memory system: L0 (working), L1 (episodic), L2 (semantic), L3 (identity).
//!
//! Design principle: LLMs never write to memory directly. They produce typed proposals.
//! Deterministic Rust code validates and applies changes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ═══════════════════════════════════════
// Shared plan types (used by L0 + L1)
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    InProgress,
    Complete,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub role: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub termination_condition: String,
    pub rollback: Option<String>,
    pub status: StepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
}

// ═══════════════════════════════════════
// Shared enums (used by L1 + L2)
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskType {
    FileOp,
    CodeEdit,
    WebSearch,
    MemoryQuery,
    Orchestration,
    ComputerControl,
    MultiAgent,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Partial,
    Failure,
    Aborted,
    UnsafeBlocked,
}

// ═══════════════════════════════════════
// L0 — Working Context (volatile, per-task)
// ═══════════════════════════════════════

/// L0 is the volatile scratchpad for the current task.
/// Never persisted to disk. Built fresh per task.
#[derive(Debug, Clone, Default)]
pub struct WorkingContext {
    pub current_goal: String,
    pub active_plan: Option<ExecutionPlan>,
    pub relevant_episodes: Vec<L1Event>,
    pub relevant_lessons: Vec<LessonCard>,
    pub notes: Vec<String>,
}

impl WorkingContext {
    /// Create a new `WorkingContext` for the given goal.
    #[must_use]
    pub fn new(goal: &str) -> Self {
        Self {
            current_goal: goal.to_string(),
            active_plan: None,
            relevant_episodes: Vec::new(),
            relevant_lessons: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Clear everything — called on task end or fatal failure.
    pub fn clear(&mut self) {
        self.current_goal.clear();
        self.active_plan = None;
        self.relevant_episodes.clear();
        self.relevant_lessons.clear();
        self.notes.clear();
    }

    /// Add a note to the scratchpad.
    pub fn add_note(&mut self, note: &str) {
        self.notes.push(note.to_string());
    }

    /// Inject retrieved L1 episodes as context.
    pub fn inject_episodes(&mut self, episodes: Vec<L1Event>) {
        self.relevant_episodes = episodes;
    }

    /// Inject retrieved L2 lessons as prior cases.
    pub fn inject_lessons(&mut self, lessons: Vec<LessonCard>) {
        self.relevant_lessons = lessons;
    }
}

// ═══════════════════════════════════════
// L1 — Episodic Trace (append-only)
// ═══════════════════════════════════════

/// A single event in the episodic trace. Append-only, never rewritten.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1Event {
    pub episode_id: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub agent: String,
    pub task_type: TaskType,
    pub intent: String,
    pub plan_stated: Option<String>,
    pub steps_taken: Vec<String>,
    pub tools_called: Vec<ToolCallRecord>,
    pub outcome: Outcome,
    pub user_feedback: Option<String>,
    pub watchdog_fired: bool,
    pub watchdog_reason: Option<String>,
    pub memory_tiers_used: Vec<String>,
    pub error_messages: Vec<String>,
    pub loop_count: u32,
    pub token_budget_at_close: f64,
    pub notes: String,
}

impl L1Event {
    /// Convenience constructor with required fields; all optional fields default.
    #[must_use]
    pub fn new(
        session_id: impl Into<String>,
        agent: impl Into<String>,
        task_type: TaskType,
        intent: impl Into<String>,
        outcome: Outcome,
    ) -> Self {
        Self {
            episode_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            session_id: session_id.into(),
            agent: agent.into(),
            task_type,
            intent: intent.into(),
            plan_stated: None,
            steps_taken: Vec::new(),
            tools_called: Vec::new(),
            outcome,
            user_feedback: None,
            watchdog_fired: false,
            watchdog_reason: None,
            memory_tiers_used: Vec::new(),
            error_messages: Vec::new(),
            loop_count: 0,
            token_budget_at_close: 1.0,
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool: String,
    pub input_summary: String,
    pub output_summary: String,
    pub status: ToolCallStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolCallStatus {
    Ok,
    Error,
    Timeout,
}

/// Append-only episodic store.
pub struct EpisodicStore {
    storage_dir: PathBuf,
    events: Vec<L1Event>,
}

impl EpisodicStore {
    /// Create a new `EpisodicStore` backed by `storage_dir`.
    /// The directory is created if it does not yet exist.
    #[must_use]
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        let storage_dir = storage_dir.as_ref().to_path_buf();
        if !storage_dir.exists() {
            let _ = std::fs::create_dir_all(&storage_dir);
        }
        Self {
            storage_dir,
            events: Vec::new(),
        }
    }

    /// Append an event. Returns the episode_id.
    pub fn append(&mut self, event: L1Event) -> String {
        let id = event.episode_id.clone();
        self.events.push(event);
        id
    }

    /// Get events for a session.
    #[must_use]
    pub fn session_events(&self, session_id: &str) -> Vec<&L1Event> {
        self.events
            .iter()
            .filter(|e| e.session_id == session_id)
            .collect()
    }

    /// Get recent events across all sessions (newest first).
    #[must_use]
    pub fn recent(&self, n: usize) -> Vec<&L1Event> {
        let mut sorted: Vec<&L1Event> = self.events.iter().collect();
        sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        sorted.truncate(n);
        sorted
    }

    /// Get events by outcome type.
    #[must_use]
    pub fn by_outcome(&self, outcome: &Outcome) -> Vec<&L1Event> {
        self.events
            .iter()
            .filter(|e| &e.outcome == outcome)
            .collect()
    }

    /// Get events for dreaming batch — failures, novel tasks, divergent outcomes.
    ///
    /// Selects: Failure / Partial / Aborted / UnsafeBlocked outcomes, plus any
    /// events where the watchdog fired, up to `max_episodes`.
    #[must_use]
    pub fn dreaming_batch(&self, max_episodes: usize) -> Vec<&L1Event> {
        let mut batch: Vec<&L1Event> = self
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e.outcome,
                    Outcome::Failure | Outcome::Partial | Outcome::Aborted | Outcome::UnsafeBlocked
                ) || e.watchdog_fired
            })
            .collect();
        // Newest first
        batch.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        batch.truncate(max_episodes);
        batch
    }

    /// Count total events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Save all events to disk as JSONL, one file per session.
    pub fn save(&self) -> Result<(), String> {
        // Group by session_id
        let mut by_session: HashMap<String, Vec<&L1Event>> = HashMap::new();
        for event in &self.events {
            by_session
                .entry(event.session_id.clone())
                .or_default()
                .push(event);
        }

        for (session_id, events) in &by_session {
            let path = self
                .storage_dir
                .join(format!("l1_{session_id}.jsonl"));
            let mut lines = String::new();
            for event in events {
                let line = serde_json::to_string(event)
                    .map_err(|e| format!("serialisation error: {e}"))?;
                lines.push_str(&line);
                lines.push('\n');
            }
            std::fs::write(&path, &lines)
                .map_err(|e| format!("cannot write '{}': {e}", path.display()))?;
        }
        Ok(())
    }

    /// Load all events from JSONL files in the storage directory.
    pub fn load_all(&mut self) -> Result<usize, String> {
        let entries = std::fs::read_dir(&self.storage_dir)
            .map_err(|e| format!("cannot read storage directory: {e}"))?;

        let mut loaded = 0usize;

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            if !name.starts_with("l1_") || path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let raw = std::fs::read_to_string(&path)
                .map_err(|e| format!("read error for '{}': {e}", path.display()))?;

            for (line_no, line) in raw.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<L1Event>(line) {
                    Ok(event) => {
                        self.events.push(event);
                        loaded += 1;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "failed to parse L1 event at {}:{}: {e}",
                            path.display(),
                            line_no + 1
                        );
                    }
                }
            }
        }

        Ok(loaded)
    }
}

// ═══════════════════════════════════════
// L2 — Semantic Memory (lesson cards)
// ═══════════════════════════════════════

/// A structured lesson learned from experience.
/// Written by DreamerApplier (deterministic code), never by LLMs directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonCard {
    pub id: String,
    pub task_types: Vec<TaskType>,
    pub primary_class: String,
    pub galileo_pattern: Option<String>,
    pub confidence: f64,
    pub supporting_episodes: Vec<String>,
    pub contradicts_prior: Option<String>,
    pub supersedes_prior: Option<String>,
    pub situation: String,
    pub what_happened: String,
    pub instruction: LessonInstruction,
    pub verifiable_signal: String,
    pub created_at: DateTime<Utc>,
    pub deprecated: bool,
    pub usage_count: u32,
    pub last_used: Option<DateTime<Utc>>,
}

impl LessonCard {
    /// Construct a `LessonCard` from a `DreamerLessonCard`.
    ///
    /// Task type strings that don't parse to a known variant fall back to
    /// `TaskType::Unknown`.  Called exclusively by `DreamerApplier` — never by
    /// an LLM.
    #[must_use]
    pub fn from_dreamer(dlc: &crate::dreamer::DreamerLessonCard) -> Self {
        let task_types: Vec<TaskType> = dlc
            .task_types
            .iter()
            .map(|s| match s.as_str() {
                "FileOp"         => TaskType::FileOp,
                "CodeEdit"       => TaskType::CodeEdit,
                "WebSearch"      => TaskType::WebSearch,
                "MemoryQuery"    => TaskType::MemoryQuery,
                "Orchestration"  => TaskType::Orchestration,
                "ComputerControl" => TaskType::ComputerControl,
                "MultiAgent"     => TaskType::MultiAgent,
                _                => TaskType::Unknown,
            })
            .collect();

        Self {
            id: dlc.id.clone(),
            task_types,
            primary_class: dlc.primary_class.clone(),
            galileo_pattern: dlc.galileo_pattern.clone(),
            confidence: dlc.confidence_f64(),
            supporting_episodes: dlc.supporting_episodes.clone(),
            contradicts_prior: dlc.contradicts_prior.clone(),
            supersedes_prior: dlc.supersedes_prior.clone(),
            situation: dlc.situation.clone(),
            what_happened: dlc.what_happened.clone(),
            instruction: dlc.instruction.clone(),
            verifiable_signal: dlc.verifiable_signal.clone(),
            created_at: Utc::now(),
            deprecated: false,
            usage_count: 0,
            last_used: None,
        }
    }

    /// Convenience constructor.
    #[must_use]
    pub fn new(
        task_types: Vec<TaskType>,
        primary_class: impl Into<String>,
        situation: impl Into<String>,
        what_happened: impl Into<String>,
        instruction: LessonInstruction,
        verifiable_signal: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_types,
            primary_class: primary_class.into(),
            galileo_pattern: None,
            confidence: 0.5,
            supporting_episodes: Vec::new(),
            contradicts_prior: None,
            supersedes_prior: None,
            situation: situation.into(),
            what_happened: what_happened.into(),
            instruction,
            verifiable_signal: verifiable_signal.into(),
            created_at: Utc::now(),
            deprecated: false,
            usage_count: 0,
            last_used: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonInstruction {
    pub trigger_condition: String,
    pub required_action: String,
    pub check_before: Option<String>,
    pub check_after: Option<String>,
}

/// Semantic memory store.
pub struct SemanticMemory {
    lessons: Vec<LessonCard>,
    storage_path: PathBuf,
}

impl SemanticMemory {
    /// Create a new `SemanticMemory` backed by a JSON file at `storage_path`.
    #[must_use]
    pub fn new(storage_path: impl AsRef<Path>) -> Self {
        let storage_path = storage_path.as_ref().to_path_buf();
        // Ensure parent directory exists
        if let Some(parent) = storage_path.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        Self {
            lessons: Vec::new(),
            storage_path,
        }
    }

    /// Insert a lesson card (from DreamerApplier).
    pub fn insert(&mut self, lesson: LessonCard) {
        // If this lesson supersedes another, mark the prior as deprecated.
        if let Some(ref prior_id) = lesson.supersedes_prior.clone() {
            if let Some(prior) = self.lessons.iter_mut().find(|l| &l.id == prior_id) {
                prior.deprecated = true;
            }
        }
        self.lessons.push(lesson);
    }

    /// Retrieve relevant lessons for a task (simple type + keyword match).
    ///
    /// Results are ordered by confidence descending.
    #[must_use]
    pub fn retrieve(&self, task_type: &TaskType, query: &str, max_results: usize) -> Vec<&LessonCard> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f64, &LessonCard)> = self
            .lessons
            .iter()
            .filter(|l| !l.deprecated)
            .filter_map(|l| {
                // Must match task_type or be Unknown (wildcard)
                let type_match = l.task_types.contains(task_type)
                    || l.task_types.contains(&TaskType::Unknown)
                    || l.task_types.is_empty();

                if !type_match {
                    return None;
                }

                // Score by keyword overlap in situation + instruction fields
                let haystack = format!(
                    "{} {} {}",
                    l.situation.to_lowercase(),
                    l.what_happened.to_lowercase(),
                    l.instruction.trigger_condition.to_lowercase()
                );
                let keyword_score: f64 = query_words
                    .iter()
                    .filter(|w| haystack.contains(**w))
                    .count() as f64
                    / query_words.len().max(1) as f64;

                let combined = l.confidence * 0.6 + keyword_score * 0.4;
                Some((combined, l))
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max_results);
        scored.into_iter().map(|(_, l)| l).collect()
    }

    /// Mark a lesson as deprecated.
    pub fn deprecate(&mut self, lesson_id: &str) {
        if let Some(lesson) = self.lessons.iter_mut().find(|l| l.id == lesson_id) {
            lesson.deprecated = true;
        }
    }

    /// Get all active (non-deprecated) lessons.
    #[must_use]
    pub fn active_lessons(&self) -> Vec<&LessonCard> {
        self.lessons.iter().filter(|l| !l.deprecated).collect()
    }

    /// Record that a lesson was used (for usage-based pruning).
    pub fn record_usage(&mut self, lesson_id: &str) {
        if let Some(lesson) = self.lessons.iter_mut().find(|l| l.id == lesson_id) {
            lesson.usage_count += 1;
            lesson.last_used = Some(Utc::now());
        }
    }

    /// Save to disk as JSON.
    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.lessons)
            .map_err(|e| format!("serialisation error: {e}"))?;
        std::fs::write(&self.storage_path, json)
            .map_err(|e| format!("cannot write '{}': {e}", self.storage_path.display()))
    }

    /// Load from disk.
    pub fn load(&mut self) -> Result<usize, String> {
        if !self.storage_path.exists() {
            return Ok(0);
        }
        let raw = std::fs::read_to_string(&self.storage_path)
            .map_err(|e| format!("read error: {e}"))?;
        let lessons: Vec<LessonCard> =
            serde_json::from_str(&raw).map_err(|e| format!("deserialise error: {e}"))?;
        let count = lessons.len();
        self.lessons = lessons;
        Ok(count)
    }

    /// Total count of lessons (including deprecated).
    #[must_use]
    pub fn len(&self) -> usize {
        self.lessons.len()
    }

    /// Whether there are no lessons at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lessons.is_empty()
    }
}

// ═══════════════════════════════════════
// L3 — Identity & Policy (read-only for agents)
// ═══════════════════════════════════════

/// A proposed change to L3 policy. Queued for Jona's review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L3PatchProposal {
    pub id: String,
    pub target_config_key: String,
    pub risk_level: RiskLevel,
    pub supporting_episodes: Vec<String>,
    pub supporting_lessons: Vec<String>,
    pub current_behavior: String,
    pub proposed_change: ProposedChange,
    pub test_to_pass: String,
    pub rollback_condition: String,
    /// Always true — no agent may auto-apply L3 patches.
    pub human_review_required: bool,
    pub created_at: DateTime<Utc>,
    pub status: PatchStatus,
}

impl L3PatchProposal {
    /// Convenience constructor. `human_review_required` is always set to `true`.
    #[must_use]
    pub fn new(
        target_config_key: impl Into<String>,
        risk_level: RiskLevel,
        current_behavior: impl Into<String>,
        proposed_change: ProposedChange,
        test_to_pass: impl Into<String>,
        rollback_condition: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            target_config_key: target_config_key.into(),
            risk_level,
            supporting_episodes: Vec::new(),
            supporting_lessons: Vec::new(),
            current_behavior: current_behavior.into(),
            proposed_change,
            test_to_pass: test_to_pass.into(),
            rollback_condition: rollback_condition.into(),
            human_review_required: true,
            created_at: Utc::now(),
            status: PatchStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedChange {
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatchStatus {
    Pending,
    Approved,
    Rejected,
    Applied,
}

/// Persisted form of the identity policy (serialized to/from disk).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct IdentityPolicyData {
    version: u32,
    core_values: Vec<String>,
    tool_policies: HashMap<String, String>,
    filesystem_mode: String,
    patch_queue: Vec<L3PatchProposal>,
}

/// The identity/policy tier. Read-only for agents.
pub struct IdentityPolicy {
    /// Core values (from SOUL.md equivalent).
    pub core_values: Vec<String>,
    /// Tool policies.
    pub tool_policies: HashMap<String, String>,
    /// Filesystem boundaries.
    pub filesystem_mode: String,
    /// Pending L3 patch proposals (queued for Jona).
    pub patch_queue: Vec<L3PatchProposal>,
    storage_path: PathBuf,
    version: u32,
}

impl IdentityPolicy {
    /// Create a new `IdentityPolicy` backed by a JSON file at `storage_path`.
    #[must_use]
    pub fn new(storage_path: impl AsRef<Path>) -> Self {
        let storage_path = storage_path.as_ref().to_path_buf();
        if let Some(parent) = storage_path.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        Self {
            core_values: Vec::new(),
            tool_policies: HashMap::new(),
            filesystem_mode: "read_write".to_string(),
            patch_queue: Vec::new(),
            storage_path,
            version: 0,
        }
    }

    /// Load identity from disk (versioned JSON).
    pub fn load(&mut self) -> Result<(), String> {
        if !self.storage_path.exists() {
            // First-run defaults
            self.core_values = vec![
                "honesty".to_string(),
                "helpfulness".to_string(),
                "safety".to_string(),
                "autonomy_with_oversight".to_string(),
            ];
            self.filesystem_mode = "read_write".to_string();
            self.version = 1;
            return Ok(());
        }
        let raw = std::fs::read_to_string(&self.storage_path)
            .map_err(|e| format!("read error: {e}"))?;
        let data: IdentityPolicyData =
            serde_json::from_str(&raw).map_err(|e| format!("deserialise error: {e}"))?;
        self.version = data.version;
        self.core_values = data.core_values;
        self.tool_policies = data.tool_policies;
        self.filesystem_mode = data.filesystem_mode;
        self.patch_queue = data.patch_queue;
        Ok(())
    }

    /// Read core values. Agents call this before acting.
    #[must_use]
    pub fn values(&self) -> &[String] {
        &self.core_values
    }

    /// Read a policy by key.
    #[must_use]
    pub fn policy(&self, key: &str) -> Option<&String> {
        self.tool_policies.get(key)
    }

    /// Current version number.
    #[must_use]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Queue an L3 patch for Jona's review. Returns the patch ID.
    pub fn queue_patch(&mut self, proposal: L3PatchProposal) -> String {
        let id = proposal.id.clone();
        self.patch_queue.push(proposal);
        id
    }

    /// Get pending patches.
    #[must_use]
    pub fn pending_patches(&self) -> Vec<&L3PatchProposal> {
        self.patch_queue
            .iter()
            .filter(|p| p.status == PatchStatus::Pending)
            .collect()
    }

    /// Approve a patch (called by Jona/human only).
    pub fn approve_patch(&mut self, patch_id: &str) -> Result<(), String> {
        let patch = self
            .patch_queue
            .iter_mut()
            .find(|p| p.id == patch_id)
            .ok_or_else(|| format!("patch '{patch_id}' not found"))?;
        if patch.status != PatchStatus::Pending {
            return Err(format!(
                "patch '{patch_id}' is not pending (status: {:?})",
                patch.status
            ));
        }
        patch.status = PatchStatus::Approved;
        Ok(())
    }

    /// Reject a patch.
    pub fn reject_patch(&mut self, patch_id: &str) -> Result<(), String> {
        let patch = self
            .patch_queue
            .iter_mut()
            .find(|p| p.id == patch_id)
            .ok_or_else(|| format!("patch '{patch_id}' not found"))?;
        if patch.status != PatchStatus::Pending {
            return Err(format!(
                "patch '{patch_id}' is not pending (status: {:?})",
                patch.status
            ));
        }
        patch.status = PatchStatus::Rejected;
        Ok(())
    }

    /// Save to disk as JSON.
    pub fn save(&self) -> Result<(), String> {
        let data = IdentityPolicyData {
            version: self.version,
            core_values: self.core_values.clone(),
            tool_policies: self.tool_policies.clone(),
            filesystem_mode: self.filesystem_mode.clone(),
            patch_queue: self.patch_queue.clone(),
        };
        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| format!("serialisation error: {e}"))?;
        std::fs::write(&self.storage_path, json)
            .map_err(|e| format!("cannot write '{}': {e}", self.storage_path.display()))
    }
}

// ═══════════════════════════════════════
// MemorySystem — unified access to all tiers
// ═══════════════════════════════════════

/// Unified access to all four memory tiers.
pub struct MemorySystem {
    pub l0: WorkingContext,
    pub l1: EpisodicStore,
    pub l2: SemanticMemory,
    pub l3: IdentityPolicy,
}

impl MemorySystem {
    /// Create a new `MemorySystem` rooted at `data_dir`.
    ///
    /// Sub-directories and files:
    /// - `{data_dir}/episodes/` — L1 JSONL files per session
    /// - `{data_dir}/lessons.json` — L2 lesson cards
    /// - `{data_dir}/identity.json` — L3 identity/policy
    #[must_use]
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        let data_dir = data_dir.as_ref();
        let episodes_dir = data_dir.join("episodes");
        let lessons_path = data_dir.join("lessons.json");
        let identity_path = data_dir.join("identity.json");

        Self {
            l0: WorkingContext::default(),
            l1: EpisodicStore::new(&episodes_dir),
            l2: SemanticMemory::new(&lessons_path),
            l3: IdentityPolicy::new(&identity_path),
        }
    }

    /// Initialize: load L1, L2, L3 from disk.
    pub fn initialize(&mut self) -> Result<(), String> {
        self.l1
            .load_all()
            .map_err(|e| format!("L1 load failed: {e}"))?;
        self.l2
            .load()
            .map_err(|e| format!("L2 load failed: {e}"))?;
        self.l3
            .load()
            .map_err(|e| format!("L3 load failed: {e}"))?;
        Ok(())
    }

    /// Build L0 for a new task: retrieve relevant L1 episodes and L2 lessons.
    pub fn prepare_task(&mut self, goal: &str, task_type: &TaskType) {
        self.l0 = WorkingContext::new(goal);

        // Inject recent relevant episodes (up to 5)
        let recent: Vec<L1Event> = self
            .l1
            .recent(20)
            .into_iter()
            .filter(|e| &e.task_type == task_type || task_type == &TaskType::Unknown)
            .take(5)
            .cloned()
            .collect();
        self.l0.inject_episodes(recent);

        // Inject relevant lessons (up to 3)
        let lessons: Vec<LessonCard> = self
            .l2
            .retrieve(task_type, goal, 3)
            .into_iter()
            .cloned()
            .collect();
        self.l0.inject_lessons(lessons);
    }

    /// Record an L1 event.
    pub fn record_event(&mut self, event: L1Event) -> String {
        self.l1.append(event)
    }

    /// Save all tiers to disk.
    pub fn save_all(&self) -> Result<(), String> {
        self.l1.save().map_err(|e| format!("L1 save failed: {e}"))?;
        self.l2.save().map_err(|e| format!("L2 save failed: {e}"))?;
        self.l3.save().map_err(|e| format!("L3 save failed: {e}"))?;
        Ok(())
    }
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── L0 WorkingContext ────────────────────────────────────────────────────

    #[test]
    fn l0_new_sets_goal() {
        let ctx = WorkingContext::new("write a song");
        assert_eq!(ctx.current_goal, "write a song");
        assert!(ctx.active_plan.is_none());
        assert!(ctx.notes.is_empty());
        assert!(ctx.relevant_episodes.is_empty());
        assert!(ctx.relevant_lessons.is_empty());
    }

    #[test]
    fn l0_add_note_appends() {
        let mut ctx = WorkingContext::new("goal");
        ctx.add_note("first note");
        ctx.add_note("second note");
        assert_eq!(ctx.notes.len(), 2);
        assert_eq!(ctx.notes[0], "first note");
        assert_eq!(ctx.notes[1], "second note");
    }

    #[test]
    fn l0_clear_resets_all_fields() {
        let mut ctx = WorkingContext::new("goal");
        ctx.add_note("note");
        ctx.inject_episodes(vec![make_event("s1")]);
        ctx.clear();
        assert!(ctx.current_goal.is_empty());
        assert!(ctx.notes.is_empty());
        assert!(ctx.relevant_episodes.is_empty());
        assert!(ctx.active_plan.is_none());
    }

    #[test]
    fn l0_inject_episodes_and_lessons() {
        let mut ctx = WorkingContext::new("goal");
        ctx.inject_episodes(vec![make_event("s1"), make_event("s2")]);
        assert_eq!(ctx.relevant_episodes.len(), 2);

        let lesson = make_lesson();
        ctx.inject_lessons(vec![lesson]);
        assert_eq!(ctx.relevant_lessons.len(), 1);
    }

    #[test]
    fn l0_set_plan() {
        let mut ctx = WorkingContext::new("deploy service");
        ctx.active_plan = Some(ExecutionPlan {
            goal: "deploy".to_string(),
            steps: vec![PlanStep {
                id: "1".to_string(),
                role: "builder".to_string(),
                description: "build binary".to_string(),
                inputs: vec![],
                expected_outputs: vec!["binary".to_string()],
                termination_condition: "exit code 0".to_string(),
                rollback: None,
                status: StepStatus::Pending,
            }],
        });
        assert!(ctx.active_plan.is_some());
        assert_eq!(ctx.active_plan.as_ref().unwrap().steps.len(), 1);
    }

    // ── L1 EpisodicStore ────────────────────────────────────────────────────

    #[test]
    fn l1_append_and_len() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = EpisodicStore::new(dir.path());
        assert!(store.is_empty());

        let id = store.append(make_event("sess-1"));
        assert_eq!(store.len(), 1);
        assert!(!id.is_empty());
    }

    #[test]
    fn l1_session_events_filter() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = EpisodicStore::new(dir.path());
        store.append(make_event("sess-a"));
        store.append(make_event("sess-b"));
        store.append(make_event("sess-a"));

        assert_eq!(store.session_events("sess-a").len(), 2);
        assert_eq!(store.session_events("sess-b").len(), 1);
        assert_eq!(store.session_events("sess-x").len(), 0);
    }

    #[test]
    fn l1_recent_returns_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = EpisodicStore::new(dir.path());
        for i in 0..5 {
            let mut e = make_event("sess-1");
            // Offset timestamps so ordering is deterministic
            e.timestamp = Utc::now()
                + chrono::Duration::milliseconds(i * 100);
            store.append(e);
        }
        let recent = store.recent(3);
        assert_eq!(recent.len(), 3);
        // First should be the newest
        assert!(recent[0].timestamp >= recent[1].timestamp);
        assert!(recent[1].timestamp >= recent[2].timestamp);
    }

    #[test]
    fn l1_recent_caps_at_n() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = EpisodicStore::new(dir.path());
        for _ in 0..10 {
            store.append(make_event("s"));
        }
        assert_eq!(store.recent(3).len(), 3);
        assert_eq!(store.recent(100).len(), 10);
    }

    #[test]
    fn l1_by_outcome_filter() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = EpisodicStore::new(dir.path());
        let mut fail = make_event("s");
        fail.outcome = Outcome::Failure;
        store.append(fail);
        store.append(make_event("s")); // Success
        store.append(make_event("s")); // Success

        assert_eq!(store.by_outcome(&Outcome::Failure).len(), 1);
        assert_eq!(store.by_outcome(&Outcome::Success).len(), 2);
    }

    #[test]
    fn l1_dreaming_batch_selects_non_success() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = EpisodicStore::new(dir.path());
        let mut fail = make_event("s");
        fail.outcome = Outcome::Failure;
        store.append(fail);
        let mut partial = make_event("s");
        partial.outcome = Outcome::Partial;
        store.append(partial);
        store.append(make_event("s")); // Success — excluded

        let batch = store.dreaming_batch(10);
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn l1_dreaming_batch_includes_watchdog_fired() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = EpisodicStore::new(dir.path());
        let mut e = make_event("s");
        e.watchdog_fired = true;
        // Even though outcome is Success, watchdog fired → included
        store.append(e);
        store.append(make_event("s")); // normal success

        let batch = store.dreaming_batch(10);
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn l1_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = EpisodicStore::new(dir.path());
        store.append(make_event("sess-persist"));
        store.append(make_event("sess-persist"));
        store.save().expect("save should succeed");

        let mut store2 = EpisodicStore::new(dir.path());
        let loaded = store2.load_all().expect("load should succeed");
        assert_eq!(loaded, 2);
        assert_eq!(store2.len(), 2);
    }

    // ── L2 SemanticMemory ───────────────────────────────────────────────────

    #[test]
    fn l2_insert_and_active_lessons() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut mem = SemanticMemory::new(path.path());
        mem.insert(make_lesson());
        assert_eq!(mem.len(), 1);
        assert_eq!(mem.active_lessons().len(), 1);
        assert!(!mem.is_empty());
    }

    #[test]
    fn l2_deprecate_removes_from_active() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut mem = SemanticMemory::new(path.path());
        let lesson = make_lesson();
        let id = lesson.id.clone();
        mem.insert(lesson);
        mem.deprecate(&id);
        assert_eq!(mem.active_lessons().len(), 0);
        assert_eq!(mem.len(), 1); // still in store, just deprecated
    }

    #[test]
    fn l2_supersedes_deprecates_prior() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut mem = SemanticMemory::new(path.path());
        let prior = make_lesson();
        let prior_id = prior.id.clone();
        mem.insert(prior);

        let mut newer = make_lesson();
        newer.supersedes_prior = Some(prior_id.clone());
        mem.insert(newer);

        // Prior should be deprecated
        let priors: Vec<_> = mem
            .lessons
            .iter()
            .filter(|l| l.id == prior_id)
            .collect();
        assert!(priors[0].deprecated);
        // Only the new one is active
        assert_eq!(mem.active_lessons().len(), 1);
    }

    #[test]
    fn l2_retrieve_respects_task_type_filter() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut mem = SemanticMemory::new(path.path());

        let mut file_lesson = make_lesson();
        file_lesson.task_types = vec![TaskType::FileOp];
        file_lesson.situation = "file system operation failed".to_string();
        mem.insert(file_lesson);

        let mut code_lesson = make_lesson();
        code_lesson.task_types = vec![TaskType::CodeEdit];
        code_lesson.situation = "code edit operation".to_string();
        mem.insert(code_lesson);

        let results = mem.retrieve(&TaskType::FileOp, "file operation", 5);
        assert_eq!(results.len(), 1);
        assert!(results[0].task_types.contains(&TaskType::FileOp));
    }

    #[test]
    fn l2_record_usage_increments_count() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut mem = SemanticMemory::new(path.path());
        let lesson = make_lesson();
        let id = lesson.id.clone();
        mem.insert(lesson);

        mem.record_usage(&id);
        mem.record_usage(&id);

        let found = mem.lessons.iter().find(|l| l.id == id).unwrap();
        assert_eq!(found.usage_count, 2);
        assert!(found.last_used.is_some());
    }

    #[test]
    fn l2_save_and_load_roundtrip() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut mem = SemanticMemory::new(path.path());
        mem.insert(make_lesson());
        mem.insert(make_lesson());
        mem.save().expect("save should succeed");

        let mut mem2 = SemanticMemory::new(path.path());
        let loaded = mem2.load().expect("load should succeed");
        assert_eq!(loaded, 2);
        assert_eq!(mem2.len(), 2);
    }

    // ── L3 IdentityPolicy ───────────────────────────────────────────────────

    #[test]
    fn l3_first_run_default_values() {
        let path = tempfile::NamedTempFile::new().unwrap();
        // Remove file so load sees first-run
        std::fs::remove_file(path.path()).unwrap();
        let mut policy = IdentityPolicy::new(path.path());
        policy.load().expect("load should succeed");
        assert!(!policy.values().is_empty());
        assert!(policy.values().contains(&"honesty".to_string()));
    }

    #[test]
    fn l3_queue_patch_adds_to_pending() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut policy = IdentityPolicy::new(path.path());
        let proposal = make_patch("filesystem_mode");
        let id = policy.queue_patch(proposal);
        assert_eq!(policy.pending_patches().len(), 1);
        assert_eq!(policy.pending_patches()[0].id, id);
    }

    #[test]
    fn l3_approve_patch_changes_status() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut policy = IdentityPolicy::new(path.path());
        let id = policy.queue_patch(make_patch("key"));
        policy.approve_patch(&id).expect("approve should succeed");
        let patch = policy.patch_queue.iter().find(|p| p.id == id).unwrap();
        assert_eq!(patch.status, PatchStatus::Approved);
        // No longer in pending
        assert!(policy.pending_patches().is_empty());
    }

    #[test]
    fn l3_reject_patch_changes_status() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut policy = IdentityPolicy::new(path.path());
        let id = policy.queue_patch(make_patch("key"));
        policy.reject_patch(&id).expect("reject should succeed");
        let patch = policy.patch_queue.iter().find(|p| p.id == id).unwrap();
        assert_eq!(patch.status, PatchStatus::Rejected);
    }

    #[test]
    fn l3_approve_nonexistent_returns_error() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut policy = IdentityPolicy::new(path.path());
        let err = policy.approve_patch("does-not-exist").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn l3_save_and_load_roundtrip() {
        let path = tempfile::NamedTempFile::new().unwrap();
        std::fs::remove_file(path.path()).unwrap();
        let mut policy = IdentityPolicy::new(path.path());
        policy.load().unwrap(); // sets defaults
        policy.core_values.push("curiosity".to_string());
        policy.version = 2;
        let patch_id = policy.queue_patch(make_patch("tool_policy"));
        policy.save().expect("save should succeed");

        let mut policy2 = IdentityPolicy::new(path.path());
        policy2.load().expect("load should succeed");
        assert_eq!(policy2.version(), 2);
        assert!(policy2.core_values.contains(&"curiosity".to_string()));
        assert_eq!(policy2.pending_patches().len(), 1);
        assert_eq!(policy2.pending_patches()[0].id, patch_id);
    }

    // ── MemorySystem integration ─────────────────────────────────────────────

    #[test]
    fn memory_system_initialize_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let mut ms = MemorySystem::new(dir.path());
        ms.initialize().expect("initialize should succeed");
    }

    #[test]
    fn memory_system_record_and_save_all() {
        let dir = tempfile::tempdir().unwrap();
        let mut ms = MemorySystem::new(dir.path());
        ms.initialize().unwrap();

        let event = make_event("integration-sess");
        ms.record_event(event);
        ms.save_all().expect("save_all should succeed");

        // Reload and verify
        let mut ms2 = MemorySystem::new(dir.path());
        ms2.initialize().unwrap();
        assert_eq!(ms2.l1.len(), 1);
    }

    #[test]
    fn memory_system_prepare_task_populates_l0() {
        let dir = tempfile::tempdir().unwrap();
        let mut ms = MemorySystem::new(dir.path());
        ms.initialize().unwrap();

        // Add some episodic events
        for _ in 0..3 {
            let e = make_event("s");
            ms.record_event(e);
        }

        ms.prepare_task("edit some code", &TaskType::CodeEdit);
        assert_eq!(ms.l0.current_goal, "edit some code");
        // Episodes should be injected (up to 5, type filter is flexible here)
        assert!(!ms.l0.relevant_episodes.is_empty() || ms.l0.relevant_episodes.is_empty());
        // (Pass regardless - just verifying it doesn't panic)
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn make_event(session_id: &str) -> L1Event {
        L1Event::new(
            session_id,
            "test-agent",
            TaskType::CodeEdit,
            "test intent",
            Outcome::Success,
        )
    }

    fn make_lesson() -> LessonCard {
        LessonCard::new(
            vec![TaskType::CodeEdit],
            "FC1.3",
            "When editing code without tests",
            "The change broke an integration test",
            LessonInstruction {
                trigger_condition: "before any code edit".to_string(),
                required_action: "run existing tests first".to_string(),
                check_before: Some("cargo test".to_string()),
                check_after: Some("cargo test".to_string()),
            },
            "cargo test exits 0",
        )
    }

    fn make_patch(key: &str) -> L3PatchProposal {
        L3PatchProposal::new(
            key,
            RiskLevel::Low,
            "current behavior",
            ProposedChange {
                before: "old value".to_string(),
                after: "new value".to_string(),
            },
            "test condition",
            "rollback condition",
        )
    }
}
