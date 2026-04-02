//! Pack loader — discovers and loads packs from directories.
//!
//! A pack directory must contain `pack.toml` (required).  If it also contains
//! `rules.toml` the file is parsed and a [`RuleEvaluator`] is constructed for
//! that pack.  The loader validates approval status before activating a pack.

use std::path::{Path, PathBuf};

use crate::evaluator::RuleEvaluator;
use crate::manifest::{PackManifest, PackTier};
use crate::rules::RuleSet;

// ═══════════════════════════════════════
// PackLoader
// ═══════════════════════════════════════

/// Loads and queries packs from one or more directories.
pub struct PackLoader {
    pack_dirs: Vec<PathBuf>,
    loaded_packs: Vec<LoadedPack>,
}

impl Default for PackLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl PackLoader {
    /// Create an empty loader with no registered directories.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pack_dirs: Vec::new(),
            loaded_packs: Vec::new(),
        }
    }

    /// Register an additional directory to scan for packs.
    pub fn add_pack_dir(&mut self, path: impl AsRef<Path>) {
        self.pack_dirs.push(path.as_ref().to_path_buf());
    }

    /// Load all packs from registered directories.
    ///
    /// Each immediate subdirectory that contains a valid, approved `pack.toml`
    /// is loaded.  Returns the number of successfully loaded packs.
    ///
    /// Errors from individual packs are logged but do not abort loading.
    pub fn load_all(&mut self) -> Result<usize, PackError> {
        self.loaded_packs.clear();
        let dirs = self.pack_dirs.clone();

        for dir in &dirs {
            if !dir.exists() {
                tracing::warn!("pack dir does not exist: {}", dir.display());
                continue;
            }
            let entries = std::fs::read_dir(dir)
                .map_err(PackError::Io)?;

            for entry in entries.flatten() {
                let pack_dir = entry.path();
                if !pack_dir.is_dir() {
                    continue;
                }
                match self.load_one(&pack_dir) {
                    Ok(lp) => self.loaded_packs.push(lp),
                    Err(e) => {
                        tracing::warn!(
                            "skipping pack at {}: {e}",
                            pack_dir.display()
                        );
                    }
                }
            }
        }

        Ok(self.loaded_packs.len())
    }

    /// Try to evaluate a query against all loaded packs.
    ///
    /// Returns the best match (highest confidence) across all packs, or `None`
    /// if no rule fires.
    #[must_use]
    pub fn evaluate(&self, query: &str) -> Option<PackEvalResult> {
        let mut best: Option<PackEvalResult> = None;

        for pack in &self.loaded_packs {
            let Some(ref ev) = pack.evaluator else { continue };
            let Some((response, conf)) = ev.evaluate(query) else { continue };

            let is_better = match &best {
                None => true,
                Some(b) => conf > b.confidence,
            };

            if is_better {
                // Determine which routing strategy fired
                let routing_method = RoutingMethod::KeywordMatch;
                best = Some(PackEvalResult {
                    pack_name: pack.manifest.pack.name.clone(),
                    rule_id: String::new(), // evaluator does not expose which rule fired
                    response,
                    confidence: conf,
                    routing_method,
                });
            }
        }

        best
    }

    /// List all loaded pack manifests.
    #[must_use]
    pub fn list(&self) -> Vec<&PackManifest> {
        self.loaded_packs.iter().map(|lp| &lp.manifest).collect()
    }

    /// Filter loaded packs by tier.
    #[must_use]
    pub fn by_tier(&self, tier: &PackTier) -> Vec<&PackManifest> {
        self.loaded_packs
            .iter()
            .filter(|lp| &lp.manifest.pack.tier == tier)
            .map(|lp| &lp.manifest)
            .collect()
    }

    // ── internals ────────────────────────────────────────────────────────────

    fn load_one(&self, pack_dir: &Path) -> Result<LoadedPack, PackError> {
        // 1. Read and parse pack.toml
        let manifest_path = pack_dir.join("pack.toml");
        if !manifest_path.exists() {
            return Err(PackError::InvalidManifest(format!(
                "pack.toml missing in {}",
                pack_dir.display()
            )));
        }
        let manifest_str = std::fs::read_to_string(&manifest_path).map_err(PackError::Io)?;
        let manifest: PackManifest = toml::from_str(&manifest_str)
            .map_err(|e| PackError::InvalidManifest(e.to_string()))?;

        // 2. Parse rules.toml if present
        let rules_path = pack_dir.join("rules.toml");
        let evaluator = if rules_path.exists() {
            let rules_str = std::fs::read_to_string(&rules_path).map_err(PackError::Io)?;
            let ruleset: RuleSet = toml::from_str(&rules_str)
                .map_err(|e| PackError::Parse(e.to_string()))?;

            if !ruleset.meta.approved {
                return Err(PackError::NotApproved(manifest.pack.name.clone()));
            }

            // Only load approved rules
            let approved_rules: Vec<_> = ruleset.rules.into_iter().filter(|r| r.approved).collect();
            Some(RuleEvaluator::new(approved_rules))
        } else {
            None
        };

        Ok(LoadedPack { manifest, evaluator })
    }
}

// ═══════════════════════════════════════
// LoadedPack
// ═══════════════════════════════════════

/// A successfully loaded pack.
pub struct LoadedPack {
    /// Parsed pack manifest.
    pub manifest: PackManifest,
    /// Optional rule evaluator (present if `rules.toml` was found and approved).
    pub evaluator: Option<RuleEvaluator>,
}

// ═══════════════════════════════════════
// PackEvalResult
// ═══════════════════════════════════════

/// The result of evaluating a query against all loaded packs.
pub struct PackEvalResult {
    /// Name of the pack whose rule fired.
    pub pack_name: String,
    /// ID of the specific rule that fired (empty if not tracked).
    pub rule_id: String,
    /// The deterministic response.
    pub response: String,
    /// Effective confidence (rule.confidence × keyword_score).
    pub confidence: f64,
    /// Which routing strategy matched this pack.
    pub routing_method: RoutingMethod,
}

// ═══════════════════════════════════════
// RoutingMethod
// ═══════════════════════════════════════

/// Which routing strategy produced the match.
pub enum RoutingMethod {
    /// Option A: keyword-based matching.
    KeywordMatch,
    /// Option C: embedding-similarity matching.
    SemanticMatch,
    /// Both strategies agreed.
    Both,
}

// ═══════════════════════════════════════
// PackError
// ═══════════════════════════════════════

/// Errors from the pack loader.
#[derive(thiserror::Error, Debug)]
pub enum PackError {
    #[error("pack.toml missing or invalid: {0}")]
    InvalidManifest(String),
    #[error("pack not approved: {0}")]
    NotApproved(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_pack(dir: &Path, name: &str, tier: &str, with_rules: bool, approved: bool) {
        let pack_dir = dir.join(name);
        fs::create_dir_all(&pack_dir).unwrap();

        let approved_str = if approved { "true" } else { "false" };
        let manifest = format!(
            r#"[pack]
name = "{name}"
version = "1.0.0"
author = "Test"
description = "Test pack"
nstn_version = ">=0.7.0"
domain = "test"
tier = "{tier}"
tags = []
license = "MIT"
"#
        );
        fs::write(pack_dir.join("pack.toml"), manifest).unwrap();

        if with_rules {
            let rules = format!(
                r#"[meta]
version = "1.0"
pack_name = "{name}"
approved = {approved_str}

[[rules]]
id = "test-rule"
description = "A test rule"
trigger_keywords = ["hello", "world"]
confidence = 0.9
proposed_by = "operator"
approved = {approved_str}
examples = []

[rules.formula]
Static = {{ response = "hi from {name}" }}
"#
            );
            fs::write(pack_dir.join("rules.toml"), rules).unwrap();
        }
    }

    #[test]
    fn loads_approved_pack_with_rules() {
        let tmp = TempDir::new().unwrap();
        write_pack(tmp.path(), "my-pack", "Operator", true, true);

        let mut loader = PackLoader::new();
        loader.add_pack_dir(tmp.path());
        let count = loader.load_all().unwrap();

        assert_eq!(count, 1);
        assert_eq!(loader.list().len(), 1);
        assert_eq!(loader.list()[0].pack.name, "my-pack");
    }

    #[test]
    fn skips_unapproved_pack() {
        let tmp = TempDir::new().unwrap();
        write_pack(tmp.path(), "bad-pack", "Operator", true, false);

        let mut loader = PackLoader::new();
        loader.add_pack_dir(tmp.path());
        let count = loader.load_all().unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn loads_pack_without_rules() {
        let tmp = TempDir::new().unwrap();
        write_pack(tmp.path(), "no-rules", "Universal", false, true);

        let mut loader = PackLoader::new();
        loader.add_pack_dir(tmp.path());
        let count = loader.load_all().unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn evaluate_returns_response_from_matching_pack() {
        let tmp = TempDir::new().unwrap();
        write_pack(tmp.path(), "greeting", "Operator", true, true);

        let mut loader = PackLoader::new();
        loader.add_pack_dir(tmp.path());
        loader.load_all().unwrap();

        let result = loader.evaluate("hello world from the operator").unwrap();
        assert_eq!(result.pack_name, "greeting");
        assert_eq!(result.response, "hi from greeting");
    }

    #[test]
    fn evaluate_returns_none_when_no_match() {
        let tmp = TempDir::new().unwrap();
        write_pack(tmp.path(), "greeting", "Operator", true, true);

        let mut loader = PackLoader::new();
        loader.add_pack_dir(tmp.path());
        loader.load_all().unwrap();

        assert!(loader.evaluate("unrelated query about finance").is_none());
    }

    #[test]
    fn by_tier_filters_correctly() {
        let tmp = TempDir::new().unwrap();
        write_pack(tmp.path(), "univ-pack", "Universal", false, true);
        write_pack(tmp.path(), "op-pack", "Operator", false, true);

        let mut loader = PackLoader::new();
        loader.add_pack_dir(tmp.path());
        loader.load_all().unwrap();

        assert_eq!(loader.by_tier(&PackTier::Universal).len(), 1);
        assert_eq!(loader.by_tier(&PackTier::Operator).len(), 1);
        assert_eq!(loader.by_tier(&PackTier::Domain).len(), 0);
    }

    #[test]
    fn load_all_idempotent_clears_previous() {
        let tmp = TempDir::new().unwrap();
        write_pack(tmp.path(), "p1", "Universal", false, true);

        let mut loader = PackLoader::new();
        loader.add_pack_dir(tmp.path());
        loader.load_all().unwrap();
        let count2 = loader.load_all().unwrap();

        assert_eq!(count2, 1); // not doubled
    }

    #[test]
    fn missing_dir_does_not_error() {
        let mut loader = PackLoader::new();
        loader.add_pack_dir("/nonexistent/path/to/packs");
        let result = loader.load_all();
        // Should succeed (warning logged, no error)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
