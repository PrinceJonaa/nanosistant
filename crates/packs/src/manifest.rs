//! Pack manifest — describes a nstn-pack's identity, tier, and routing hints.
//!
//! Every pack directory or archive must contain a `pack.toml` file whose
//! contents deserialize into [`PackManifest`].

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════
// PackManifest
// ═══════════════════════════════════════

/// Top-level wrapper that matches the `[pack]` TOML table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    pub pack: PackMeta,
}

/// Full metadata for a pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMeta {
    /// Pack identifier, e.g. `"music-theory"`.
    pub name: String,
    /// SemVer string, e.g. `"1.0.0"`.
    pub version: String,
    /// Author name or org.
    pub author: String,
    /// Human-readable description.
    pub description: String,
    /// Semver constraint on the minimum Nanosistant engine version, e.g. `">=0.6.0"`.
    pub nstn_version: String,
    /// Domain bucket: `"music"`, `"finance"`, `"custom"`, etc.
    pub domain: String,
    /// The tier this pack belongs to.
    pub tier: PackTier,
    /// Searchable tags.
    pub tags: Vec<String>,
    /// SPDX license identifier or short name.
    pub license: String,
    /// Optional project homepage URL.
    pub homepage: Option<String>,
    /// Optional source repository URL.
    pub repository: Option<String>,
    /// Number of exported functions (informational).
    pub functions: Option<u32>,
    /// Test coverage percentage as a string, e.g. `"100%"`.
    pub test_coverage: Option<String>,
    /// Routing hints used by the pack loader.
    pub routing: Option<RoutingMeta>,
}

// ═══════════════════════════════════════
// PackTier
// ═══════════════════════════════════════

/// The tier a pack belongs to — controls trust, review requirements, and routing priority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PackTier {
    /// Core functions that apply to any domain (logic, graph, probability, …).
    Universal,
    /// Domain-specific functions (music, finance, time, …).
    Domain,
    /// Operator-defined TOML rules for a specific deployment context.
    Operator,
}

// ═══════════════════════════════════════
// RoutingMeta
// ═══════════════════════════════════════

/// Routing hints embedded in the pack manifest.
///
/// Supports two complementary routing strategies:
/// - **Option A** – explicit keyword triggers (fast, deterministic).
/// - **Option C** – semantic description for embedding-similarity routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingMeta {
    /// Option A: explicit keyword triggers — the pack fires when *any* keyword
    /// appears in the lowercased query.
    pub keywords: Vec<String>,
    /// Option C: free-text description used to compute an embedding for
    /// semantic similarity matching.
    pub semantic_description: Option<String>,
    /// Confidence threshold (0.0–1.0).  The pack only fires when its match
    /// score exceeds this value.
    pub confidence_threshold: f64,
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_toml() -> &'static str {
        r#"
[pack]
name = "music-theory"
version = "1.0.0"
author = "Nanosistant Team"
description = "Music theory helpers: BPM, scales, chords"
nstn_version = ">=0.6.0"
domain = "music"
tier = "Domain"
tags = ["music", "bpm", "theory"]
license = "MIT"
functions = 12
test_coverage = "100%"

[pack.routing]
keywords = ["bpm", "scale", "chord", "note", "hz"]
semantic_description = "Music theory calculations including BPM, scales, chords, and note frequencies"
confidence_threshold = 0.75
"#
    }

    #[test]
    fn deserializes_full_manifest() {
        let manifest: PackManifest = toml::from_str(sample_toml()).unwrap();
        assert_eq!(manifest.pack.name, "music-theory");
        assert_eq!(manifest.pack.tier, PackTier::Domain);
        assert_eq!(manifest.pack.functions, Some(12));
    }

    #[test]
    fn routing_meta_parses() {
        let manifest: PackManifest = toml::from_str(sample_toml()).unwrap();
        let routing = manifest.pack.routing.unwrap();
        assert_eq!(routing.keywords.len(), 5);
        assert!((routing.confidence_threshold - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn manifest_roundtrips_via_toml() {
        let manifest: PackManifest = toml::from_str(sample_toml()).unwrap();
        let serialized = toml::to_string(&manifest).unwrap();
        let reparsed: PackManifest = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed.pack.name, manifest.pack.name);
        assert_eq!(reparsed.pack.tier, manifest.pack.tier);
    }

    #[test]
    fn operator_tier_parses() {
        let toml_str = r#"
[pack]
name = "my-rules"
version = "0.1.0"
author = "Operator"
description = "Custom business rules"
nstn_version = ">=0.7.0"
domain = "custom"
tier = "Operator"
tags = []
license = "LicenseRef-Proprietary"
"#;
        let manifest: PackManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.pack.tier, PackTier::Operator);
    }
}
