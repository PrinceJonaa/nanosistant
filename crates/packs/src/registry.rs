//! Pack registry — in-memory catalogue of installed packs with usage tracking.
//!
//! The registry is a lightweight index over installed packs. It can be
//! serialised to JSON for persistence and reloaded on startup.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::manifest::{PackManifest, PackTier};
use crate::loader::PackError;

// ═══════════════════════════════════════
// RegistryEntry
// ═══════════════════════════════════════

/// A single entry in the pack registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// The pack's manifest.
    pub manifest: PackManifest,
    /// Filesystem path where the pack is installed.
    pub install_path: PathBuf,
    /// When the pack was registered.
    pub installed_at: DateTime<Utc>,
    /// How many times this pack has been used (query fired and answered).
    pub usage_count: u64,
}

// ═══════════════════════════════════════
// PackRegistry
// ═══════════════════════════════════════

/// In-memory registry of all available packs and their routing info.
pub struct PackRegistry {
    entries: Vec<RegistryEntry>,
}

impl Default for PackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PackRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Register a new pack entry.
    pub fn register(&mut self, entry: RegistryEntry) {
        // Deduplicate by pack name — replace if already present
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.manifest.pack.name == entry.manifest.pack.name)
        {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// Search by pack name, description, or tags (case-insensitive substring).
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&RegistryEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                let m = &e.manifest.pack;
                m.name.to_lowercase().contains(&q)
                    || m.description.to_lowercase().contains(&q)
                    || m.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || m.domain.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Get all packs in a given domain (case-insensitive).
    #[must_use]
    pub fn by_domain(&self, domain: &str) -> Vec<&RegistryEntry> {
        let d = domain.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.manifest.pack.domain.to_lowercase() == d)
            .collect()
    }

    /// Get all packs at a given tier.
    #[must_use]
    pub fn by_tier(&self, tier: &PackTier) -> Vec<&RegistryEntry> {
        self.entries
            .iter()
            .filter(|e| &e.manifest.pack.tier == tier)
            .collect()
    }

    /// Increment the usage counter for a pack.
    pub fn record_usage(&mut self, pack_name: &str) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.manifest.pack.name == pack_name) {
            e.usage_count += 1;
        }
    }

    /// Aggregate statistics across all registered packs.
    #[must_use]
    pub fn stats(&self) -> RegistryStats {
        let mut stats = RegistryStats {
            total_packs: self.entries.len(),
            universal_count: 0,
            domain_count: 0,
            operator_count: 0,
            total_usage: 0,
        };
        for e in &self.entries {
            match e.manifest.pack.tier {
                PackTier::Universal => stats.universal_count += 1,
                PackTier::Domain => stats.domain_count += 1,
                PackTier::Operator => stats.operator_count += 1,
            }
            stats.total_usage += e.usage_count;
        }
        stats
    }

    /// Persist the registry to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), PackError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(PackError::Io)?;
        }
        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| PackError::Parse(e.to_string()))?;
        std::fs::write(path, json).map_err(PackError::Io)?;
        Ok(())
    }

    /// Load the registry from a JSON file.
    pub fn load(path: &Path) -> Result<Self, PackError> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let json = std::fs::read_to_string(path).map_err(PackError::Io)?;
        let entries: Vec<RegistryEntry> = serde_json::from_str(&json)
            .map_err(|e| PackError::Parse(e.to_string()))?;
        Ok(Self { entries })
    }
}

// ═══════════════════════════════════════
// RegistryStats
// ═══════════════════════════════════════

/// Aggregate statistics for the pack registry.
#[derive(Debug, Clone, Serialize)]
pub struct RegistryStats {
    pub total_packs: usize,
    pub universal_count: usize,
    pub domain_count: usize,
    pub operator_count: usize,
    pub total_usage: u64,
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{PackMeta, PackTier};
    use tempfile::TempDir;

    fn make_manifest(name: &str, tier: PackTier, domain: &str) -> PackManifest {
        PackManifest {
            pack: PackMeta {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                author: "Test".to_string(),
                description: format!("{name} description"),
                nstn_version: ">=0.7.0".to_string(),
                domain: domain.to_string(),
                tier,
                tags: vec!["test".to_string(), name.to_string()],
                license: "MIT".to_string(),
                homepage: None,
                repository: None,
                functions: Some(5),
                test_coverage: Some("100%".to_string()),
                routing: None,
            },
        }
    }

    fn make_entry(name: &str, tier: PackTier, domain: &str) -> RegistryEntry {
        RegistryEntry {
            manifest: make_manifest(name, tier, domain),
            install_path: PathBuf::from(format!("/packs/{name}")),
            installed_at: Utc::now(),
            usage_count: 0,
        }
    }

    #[test]
    fn register_and_list() {
        let mut reg = PackRegistry::new();
        reg.register(make_entry("music", PackTier::Domain, "music"));
        reg.register(make_entry("logic", PackTier::Universal, "universal"));
        assert_eq!(reg.stats().total_packs, 2);
    }

    #[test]
    fn register_deduplicates_by_name() {
        let mut reg = PackRegistry::new();
        reg.register(make_entry("music", PackTier::Domain, "music"));
        reg.register(make_entry("music", PackTier::Domain, "music")); // same name
        assert_eq!(reg.stats().total_packs, 1);
    }

    #[test]
    fn search_by_name() {
        let mut reg = PackRegistry::new();
        reg.register(make_entry("music-theory", PackTier::Domain, "music"));
        reg.register(make_entry("finance-calc", PackTier::Domain, "finance"));

        let results = reg.search("music");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.pack.name, "music-theory");
    }

    #[test]
    fn search_by_domain() {
        let mut reg = PackRegistry::new();
        reg.register(make_entry("music-theory", PackTier::Domain, "music"));
        reg.register(make_entry("finance-calc", PackTier::Domain, "finance"));

        let results = reg.search("finance");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn by_domain_filter() {
        let mut reg = PackRegistry::new();
        reg.register(make_entry("a", PackTier::Domain, "music"));
        reg.register(make_entry("b", PackTier::Domain, "music"));
        reg.register(make_entry("c", PackTier::Domain, "finance"));

        assert_eq!(reg.by_domain("music").len(), 2);
        assert_eq!(reg.by_domain("finance").len(), 1);
        assert_eq!(reg.by_domain("geo").len(), 0);
    }

    #[test]
    fn by_tier_filter() {
        let mut reg = PackRegistry::new();
        reg.register(make_entry("logic", PackTier::Universal, "universal"));
        reg.register(make_entry("music", PackTier::Domain, "music"));
        reg.register(make_entry("my-rules", PackTier::Operator, "custom"));

        assert_eq!(reg.by_tier(&PackTier::Universal).len(), 1);
        assert_eq!(reg.by_tier(&PackTier::Domain).len(), 1);
        assert_eq!(reg.by_tier(&PackTier::Operator).len(), 1);
    }

    #[test]
    fn record_usage_increments_counter() {
        let mut reg = PackRegistry::new();
        reg.register(make_entry("music", PackTier::Domain, "music"));
        reg.record_usage("music");
        reg.record_usage("music");

        assert_eq!(reg.stats().total_usage, 2);
    }

    #[test]
    fn record_usage_unknown_pack_is_noop() {
        let mut reg = PackRegistry::new();
        reg.register(make_entry("music", PackTier::Domain, "music"));
        reg.record_usage("nonexistent"); // should not panic
        assert_eq!(reg.stats().total_usage, 0);
    }

    #[test]
    fn stats_counts_by_tier() {
        let mut reg = PackRegistry::new();
        reg.register(make_entry("l1", PackTier::Universal, "u"));
        reg.register(make_entry("l2", PackTier::Universal, "u"));
        reg.register(make_entry("d1", PackTier::Domain, "d"));
        reg.register(make_entry("o1", PackTier::Operator, "c"));

        let stats = reg.stats();
        assert_eq!(stats.universal_count, 2);
        assert_eq!(stats.domain_count, 1);
        assert_eq!(stats.operator_count, 1);
        assert_eq!(stats.total_packs, 4);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("registry.json");

        let mut reg = PackRegistry::new();
        reg.register(make_entry("music", PackTier::Domain, "music"));
        reg.record_usage("music");
        reg.save(&path).unwrap();

        let loaded = PackRegistry::load(&path).unwrap();
        let stats = loaded.stats();
        assert_eq!(stats.total_packs, 1);
        assert_eq!(stats.total_usage, 1);
    }

    #[test]
    fn load_nonexistent_returns_empty() {
        let reg = PackRegistry::load(Path::new("/nonexistent/registry.json")).unwrap();
        assert_eq!(reg.stats().total_packs, 0);
    }
}
