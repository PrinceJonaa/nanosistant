//! External Mirror — Jona as the human gate.
//!
//! All L3 changes, Watchdog escalations, and dreaming reports are surfaced to
//! Jona before application. The `ExternalMirror` maintains a persistent
//! notification queue, serialized to disk as JSON.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ═══════════════════════════════════════
// Notification Types
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationType {
    WatchdogTrigger,
    DreamingReport,
    L3PatchProposal,
    SafetyIncident,
    SystemHealth,
}

// ═══════════════════════════════════════
// Mirror Notification
// ═══════════════════════════════════════

/// A notification queued for Jona's attention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorNotification {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub notification_type: NotificationType,
    pub summary: String,
    pub details: serde_json::Value,
    pub requires_action: bool,
    pub acknowledged: bool,
}

impl MirrorNotification {
    /// Create a new, unacknowledged notification with a generated UUID.
    pub fn new(
        notification_type: NotificationType,
        summary: impl Into<String>,
        details: serde_json::Value,
        requires_action: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            notification_type,
            summary: summary.into(),
            details,
            requires_action,
            acknowledged: false,
        }
    }
}

// ═══════════════════════════════════════
// ExternalMirror
// ═══════════════════════════════════════

/// The notification queue for Jona.
///
/// Persisted as a JSON array at `storage_path`. New notifications are pushed to
/// the in-memory list and flushed to disk on `save()`. Acknowledgements are
/// persisted the same way.
pub struct ExternalMirror {
    notifications: Vec<MirrorNotification>,
    storage_path: PathBuf,
}

impl ExternalMirror {
    /// Create a new `ExternalMirror` backed by a JSON file at `storage_path`.
    ///
    /// The file is created on first `save()`. The parent directory is created
    /// eagerly if it does not yet exist.
    pub fn new(storage_path: impl AsRef<Path>) -> Self {
        let storage_path = storage_path.as_ref().to_path_buf();
        if let Some(parent) = storage_path.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        Self {
            notifications: Vec::new(),
            storage_path,
        }
    }

    /// Push a notification onto the queue.
    pub fn notify(&mut self, notification: MirrorNotification) {
        self.notifications.push(notification);
    }

    /// Return all pending (unacknowledged) notifications.
    pub fn pending(&self) -> Vec<&MirrorNotification> {
        self.notifications
            .iter()
            .filter(|n| !n.acknowledged)
            .collect()
    }

    /// Acknowledge a notification by its ID.
    ///
    /// Does nothing if the ID is not found.
    pub fn acknowledge(&mut self, notification_id: &str) {
        if let Some(n) = self.notifications.iter_mut().find(|n| n.id == notification_id) {
            n.acknowledged = true;
        }
    }

    /// Persist the full notification list to disk as pretty-printed JSON.
    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.notifications)
            .map_err(|e| format!("serialisation error: {e}"))?;
        std::fs::write(&self.storage_path, json)
            .map_err(|e| format!("cannot write '{}': {e}", self.storage_path.display()))
    }

    /// Load notifications from disk, replacing the in-memory list.
    ///
    /// Returns the number of notifications loaded. If the file does not exist,
    /// returns `Ok(0)` without error (first-run case).
    pub fn load(&mut self) -> Result<usize, String> {
        if !self.storage_path.exists() {
            return Ok(0);
        }
        let raw = std::fs::read_to_string(&self.storage_path)
            .map_err(|e| format!("read error: {e}"))?;
        let notifications: Vec<MirrorNotification> =
            serde_json::from_str(&raw).map_err(|e| format!("deserialise error: {e}"))?;
        let count = notifications.len();
        self.notifications = notifications;
        Ok(count)
    }

    /// Total number of notifications in the queue (acknowledged + pending).
    pub fn len(&self) -> usize {
        self.notifications.len()
    }

    /// Whether the queue has no notifications at all.
    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn temp_mirror() -> (TempDir, ExternalMirror) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("mirror.json");
        let mirror = ExternalMirror::new(&path);
        (dir, mirror)
    }

    fn watchdog_notification() -> MirrorNotification {
        MirrorNotification::new(
            NotificationType::WatchdogTrigger,
            "Watchdog triggered: loop count exceeded",
            json!({"episode_id": "ep-001", "loop_count": 5}),
            true,
        )
    }

    fn dreaming_notification() -> MirrorNotification {
        MirrorNotification::new(
            NotificationType::DreamingReport,
            "Dreaming batch complete: 2 lesson cards",
            json!({"dream_id": "dream-001", "lessons": 2}),
            false,
        )
    }

    // ── basic operations ──────────────────────────────────────────────────────

    #[test]
    fn new_mirror_is_empty() {
        let (_dir, mirror) = temp_mirror();
        assert!(mirror.is_empty());
        assert_eq!(mirror.len(), 0);
    }

    #[test]
    fn notify_increases_len() {
        let (_dir, mut mirror) = temp_mirror();
        mirror.notify(watchdog_notification());
        assert_eq!(mirror.len(), 1);
        mirror.notify(dreaming_notification());
        assert_eq!(mirror.len(), 2);
    }

    #[test]
    fn pending_returns_unacknowledged_only() {
        let (_dir, mut mirror) = temp_mirror();
        let n1 = watchdog_notification();
        let n1_id = n1.id.clone();
        mirror.notify(n1);
        mirror.notify(dreaming_notification());

        assert_eq!(mirror.pending().len(), 2);

        mirror.acknowledge(&n1_id);
        let pending = mirror.pending();
        assert_eq!(pending.len(), 1);
        assert_ne!(pending[0].id, n1_id);
    }

    #[test]
    fn acknowledge_unknown_id_is_noop() {
        let (_dir, mut mirror) = temp_mirror();
        mirror.notify(watchdog_notification());
        mirror.acknowledge("non-existent-id");
        assert_eq!(mirror.pending().len(), 1); // still pending
    }

    #[test]
    fn acknowledge_sets_acknowledged_flag() {
        let (_dir, mut mirror) = temp_mirror();
        let n = watchdog_notification();
        let id = n.id.clone();
        mirror.notify(n);

        assert!(!mirror.notifications[0].acknowledged);
        mirror.acknowledge(&id);
        assert!(mirror.notifications[0].acknowledged);
    }

    // ── persistence ───────────────────────────────────────────────────────────

    #[test]
    fn save_and_load_round_trips() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("mirror.json");

        {
            let mut mirror = ExternalMirror::new(&path);
            mirror.notify(watchdog_notification());
            mirror.notify(dreaming_notification());
            mirror.save().expect("save");
        }

        {
            let mut mirror = ExternalMirror::new(&path);
            let loaded = mirror.load().expect("load");
            assert_eq!(loaded, 2);
            assert_eq!(mirror.len(), 2);
        }
    }

    #[test]
    fn load_nonexistent_file_returns_zero() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("does_not_exist.json");
        let mut mirror = ExternalMirror::new(&path);
        let loaded = mirror.load().expect("load");
        assert_eq!(loaded, 0);
        assert!(mirror.is_empty());
    }

    #[test]
    fn save_then_load_preserves_acknowledged_state() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("mirror.json");

        let id;
        {
            let mut mirror = ExternalMirror::new(&path);
            let n = watchdog_notification();
            id = n.id.clone();
            mirror.notify(n);
            mirror.acknowledge(&id);
            mirror.save().expect("save");
        }

        {
            let mut mirror = ExternalMirror::new(&path);
            mirror.load().expect("load");
            assert!(mirror.pending().is_empty());
            assert!(mirror.notifications[0].acknowledged);
        }
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = TempDir::new().expect("tempdir");
        // Deep nested path that doesn't exist yet
        let path = dir.path().join("deep").join("nested").join("mirror.json");
        let mut mirror = ExternalMirror::new(&path);
        mirror.notify(watchdog_notification());
        mirror.save().expect("save should create parent dirs");
        assert!(path.exists());
    }

    // ── notification fields ───────────────────────────────────────────────────

    #[test]
    fn notification_has_unique_ids() {
        let n1 = watchdog_notification();
        let n2 = watchdog_notification();
        assert_ne!(n1.id, n2.id);
    }

    #[test]
    fn notification_type_preserved() {
        let (_dir, mut mirror) = temp_mirror();
        mirror.notify(watchdog_notification());
        assert_eq!(
            mirror.notifications[0].notification_type,
            NotificationType::WatchdogTrigger
        );
    }

    #[test]
    fn requires_action_preserved() {
        let n = watchdog_notification(); // requires_action: true
        assert!(n.requires_action);
        let n2 = dreaming_notification(); // requires_action: false
        assert!(!n2.requires_action);
    }

    #[test]
    fn new_notification_is_not_acknowledged() {
        let n = watchdog_notification();
        assert!(!n.acknowledged);
    }

    // ── all notification types roundtrip ──────────────────────────────────────

    #[test]
    fn all_notification_types_serialize() {
        let types = [
            NotificationType::WatchdogTrigger,
            NotificationType::DreamingReport,
            NotificationType::L3PatchProposal,
            NotificationType::SafetyIncident,
            NotificationType::SystemHealth,
        ];
        for nt in types {
            let n = MirrorNotification::new(nt, "test", json!({}), false);
            let json = serde_json::to_string(&n).expect("serialize");
            let back: MirrorNotification = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.id, n.id);
        }
    }

    // ── pending after multiple acknowledge ────────────────────────────────────

    #[test]
    fn pending_empty_when_all_acknowledged() {
        let (_dir, mut mirror) = temp_mirror();
        let n1 = watchdog_notification();
        let n2 = dreaming_notification();
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        mirror.notify(n1);
        mirror.notify(n2);

        mirror.acknowledge(&id1);
        mirror.acknowledge(&id2);

        assert!(mirror.pending().is_empty());
        assert_eq!(mirror.len(), 2); // still in list, just acknowledged
    }
}
