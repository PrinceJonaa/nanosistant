//! Session persistence — saves and restores orchestrator sessions to disk.
//!
//! Each session is stored as `{storage_dir}/{session_id}.json`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── SessionMessage ────────────────────────────────────────────────────────────

/// A single message within a persisted session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// "user", "assistant", or "tool".
    pub role: String,
    pub content: String,
    pub domain: String,
    pub timestamp: DateTime<Utc>,
    pub tokens: u32,
}

// ── PersistedSession ──────────────────────────────────────────────────────────

/// A complete persisted session record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub current_domain: String,
    pub turn_count: u32,
    pub tokens_used: u32,
    pub messages: Vec<SessionMessage>,
}

impl PersistedSession {
    /// Create a new session with the given ID.
    #[must_use]
    pub fn new(session_id: impl Into<String>, domain: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            session_id: session_id.into(),
            created_at: now,
            last_active: now,
            current_domain: domain.into(),
            turn_count: 0,
            tokens_used: 0,
            messages: Vec::new(),
        }
    }
}

// ── SessionStore ──────────────────────────────────────────────────────────────

/// Manages session persistence to the filesystem.
///
/// Sessions are serialised as JSON and written to individual files named
/// `{storage_dir}/{session_id}.json`.
pub struct SessionStore {
    storage_dir: PathBuf,
    sessions: HashMap<String, PersistedSession>,
}

impl SessionStore {
    /// Create a new `SessionStore` backed by `storage_dir`.
    ///
    /// The directory is created if it does not yet exist.
    #[must_use]
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        let storage_dir = storage_dir.as_ref().to_path_buf();
        if !storage_dir.exists() {
            // Best-effort creation; load_all will surface errors if it fails.
            let _ = std::fs::create_dir_all(&storage_dir);
        }
        Self {
            storage_dir,
            sessions: HashMap::new(),
        }
    }

    /// Load all sessions from disk into memory.
    ///
    /// Returns the number of sessions successfully loaded.
    pub fn load_all(&mut self) -> Result<usize, String> {
        let entries = std::fs::read_dir(&self.storage_dir)
            .map_err(|e| format!("cannot read storage directory: {e}"))?;

        let mut loaded = 0usize;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            match self.load_session_file(&path) {
                Ok(session) => {
                    self.sessions.insert(session.session_id.clone(), session);
                    loaded += 1;
                }
                Err(e) => {
                    tracing::warn!("failed to load session from '{}': {e}", path.display());
                }
            }
        }

        Ok(loaded)
    }

    /// Save a session to `{storage_dir}/{session_id}.json`.
    pub fn save(&self, session: &PersistedSession) -> Result<(), String> {
        let path = self.session_path(&session.session_id);
        let json =
            serde_json::to_string_pretty(session).map_err(|e| format!("serialisation error: {e}"))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("cannot write '{}': {e}", path.display()))
    }

    /// Get a session by ID (from the in-memory cache).
    #[must_use]
    pub fn get(&self, session_id: &str) -> Option<&PersistedSession> {
        self.sessions.get(session_id)
    }

    /// List all session IDs currently in memory.
    #[must_use]
    pub fn list(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.sessions.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Insert or update a session in the in-memory map.
    ///
    /// Does **not** automatically persist to disk; call [`save`][Self::save]
    /// to write through.
    pub fn upsert(&mut self, session: PersistedSession) {
        self.sessions.insert(session.session_id.clone(), session);
    }

    /// Delete a session from memory and remove its file from disk.
    pub fn delete(&mut self, session_id: &str) -> Result<(), String> {
        self.sessions.remove(session_id);
        let path = self.session_path(session_id);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("cannot delete '{}': {e}", path.display()))?;
        }
        Ok(())
    }

    /// Append a message to the named session, updating token and turn counts.
    ///
    /// Creates the session if it does not exist (with domain `"general"`).
    pub fn record_message(
        &mut self,
        session_id: &str,
        role: &str,
        content: &str,
        domain: &str,
        tokens: u32,
    ) {
        let session = self.sessions.entry(session_id.to_owned()).or_insert_with(|| {
            PersistedSession::new(session_id, domain)
        });

        let msg = SessionMessage {
            role: role.to_owned(),
            content: content.to_owned(),
            domain: domain.to_owned(),
            timestamp: Utc::now(),
            tokens,
        };

        session.messages.push(msg);
        session.tokens_used += tokens;
        session.last_active = Utc::now();

        if role == "user" {
            session.turn_count += 1;
        }
    }

    /// Return the `n` most recently-active sessions, newest first.
    #[must_use]
    pub fn recent(&self, n: usize) -> Vec<&PersistedSession> {
        let mut sessions: Vec<&PersistedSession> = self.sessions.values().collect();
        sessions.sort_by(|a, b| b.last_active.cmp(&a.last_active));
        sessions.truncate(n);
        sessions
    }

    /// Count of sessions currently in memory.
    #[must_use]
    pub fn count(&self) -> usize {
        self.sessions.len()
    }

    // ── private helpers ───────────────────────────────────────────────────────

    fn session_path(&self, session_id: &str) -> PathBuf {
        self.storage_dir.join(format!("{session_id}.json"))
    }

    fn load_session_file(&self, path: &Path) -> Result<PersistedSession, String> {
        let raw =
            std::fs::read_to_string(path).map_err(|e| format!("read error: {e}"))?;
        serde_json::from_str(&raw).map_err(|e| format!("deserialise error: {e}"))
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(id: &str) -> PersistedSession {
        PersistedSession::new(id, "general")
    }

    // ── create / save / reload ────────────────────────────────────────────────

    #[test]
    fn create_save_and_reload_session() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SessionStore::new(dir.path());

        let session = make_session("sess-001");
        store.upsert(session.clone());
        store.save(&session).expect("save should succeed");

        // Fresh store from same directory.
        let mut store2 = SessionStore::new(dir.path());
        let loaded = store2.load_all().expect("load_all should succeed");
        assert_eq!(loaded, 1);

        let reloaded = store2.get("sess-001").expect("session should exist");
        assert_eq!(reloaded.session_id, "sess-001");
        assert_eq!(reloaded.current_domain, "general");
    }

    // ── record messages ───────────────────────────────────────────────────────

    #[test]
    fn record_messages_and_verify() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SessionStore::new(dir.path());

        store.upsert(make_session("s1"));
        store.record_message("s1", "user", "hello", "general", 10);
        store.record_message("s1", "assistant", "hi there", "general", 20);

        let session = store.get("s1").unwrap();
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[0].content, "hello");
        assert_eq!(session.messages[1].role, "assistant");
        assert_eq!(session.tokens_used, 30);
        assert_eq!(session.turn_count, 1, "only user messages increment turn_count");
    }

    // ── delete ────────────────────────────────────────────────────────────────

    #[test]
    fn delete_removes_from_disk_and_memory() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SessionStore::new(dir.path());

        let session = make_session("to-delete");
        store.upsert(session.clone());
        store.save(&session).unwrap();

        let file = dir.path().join("to-delete.json");
        assert!(file.exists(), "file should exist before delete");

        store.delete("to-delete").expect("delete should succeed");

        assert!(store.get("to-delete").is_none(), "session should be gone from memory");
        assert!(!file.exists(), "file should be removed from disk");
    }

    // ── recent ────────────────────────────────────────────────────────────────

    #[test]
    fn recent_returns_sorted_by_last_active() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SessionStore::new(dir.path());

        // Insert sessions and trigger last_active updates at slightly different times.
        store.upsert(make_session("alpha"));
        // Small sleep to ensure different timestamps on platforms with
        // coarse-grained clocks — use record_message to update last_active.
        store.record_message("alpha", "user", "first", "general", 5);

        store.upsert(make_session("beta"));
        store.record_message("beta", "user", "second", "general", 5);

        store.upsert(make_session("gamma"));
        store.record_message("gamma", "user", "third", "general", 5);

        let recent = store.recent(2);
        assert_eq!(recent.len(), 2, "should return at most 2");
        // The most recently active should be first.
        assert_eq!(recent[0].session_id, "gamma");
    }

    // ── multiple sessions ─────────────────────────────────────────────────────

    #[test]
    fn multiple_sessions_work_independently() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SessionStore::new(dir.path());

        for i in 0..5 {
            let session = make_session(&format!("sess-{i}"));
            store.upsert(session.clone());
            store.save(&session).unwrap();
        }

        assert_eq!(store.count(), 5);

        // Reload and verify all are present.
        let mut store2 = SessionStore::new(dir.path());
        let loaded = store2.load_all().unwrap();
        assert_eq!(loaded, 5);
        assert_eq!(store2.count(), 5);

        let ids = store2.list();
        assert_eq!(ids.len(), 5);
        // list() should be sorted
        assert!(ids.windows(2).all(|w| w[0] <= w[1]));
    }

    // ── count ─────────────────────────────────────────────────────────────────

    #[test]
    fn count_reflects_in_memory_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SessionStore::new(dir.path());

        assert_eq!(store.count(), 0);
        store.upsert(make_session("a"));
        assert_eq!(store.count(), 1);
        store.upsert(make_session("b"));
        assert_eq!(store.count(), 2);
        store.delete("a").unwrap();
        assert_eq!(store.count(), 1);
    }

    // ── recent n=0 ────────────────────────────────────────────────────────────

    #[test]
    fn recent_zero_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SessionStore::new(dir.path());
        store.upsert(make_session("x"));
        let recent = store.recent(0);
        assert!(recent.is_empty());
    }
}
