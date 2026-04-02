//! Code utilities deterministic functions — semver, hashing, encoding, diff metrics.

use serde::Serialize;

// ═══════════════════════════════════════
// Semver
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: String,
    pub build: String,
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty()   { write!(f, "-{}", self.pre)?;   }
        if !self.build.is_empty() { write!(f, "+{}", self.build)?; }
        Ok(())
    }
}

/// Parse a semver string.
pub fn parse_semver(s: &str) -> Result<SemVer, String> {
    let s = s.trim_start_matches('v');
    let (core, build) = if let Some((c, b)) = s.split_once('+') { (c, b.to_string()) }
                        else { (s, String::new()) };
    let (nums, pre) = if let Some((n, p)) = core.split_once('-') { (n, p.to_string()) }
                      else { (core, String::new()) };
    let parts: Vec<&str> = nums.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err(format!("invalid semver: expected X.Y.Z, got '{s}'"));
    }
    let parse_part = |p: &str| p.parse::<u64>().map_err(|e| format!("invalid number '{p}': {e}"));
    Ok(SemVer {
        major: parse_part(parts[0])?,
        minor: parse_part(parts[1])?,
        patch: parse_part(parts[2])?,
        pre,
        build,
    })
}

/// Compare two semver strings. Returns -1, 0, or 1.
pub fn semver_compare(a: &str, b: &str) -> Result<i32, String> {
    let va = parse_semver(a)?;
    let vb = parse_semver(b)?;
    let cmp = va.major.cmp(&vb.major)
        .then(va.minor.cmp(&vb.minor))
        .then(va.patch.cmp(&vb.patch));
    Ok(match cmp {
        std::cmp::Ordering::Less    => -1,
        std::cmp::Ordering::Equal   => 0,
        std::cmp::Ordering::Greater => 1,
    })
}

/// Whether version satisfies a constraint (e.g. ">=1.2.0", "^2.0.0", "~1.2").
pub fn semver_satisfies(version: &str, constraint: &str) -> Result<bool, String> {
    let v = parse_semver(version)?;
    let constraint = constraint.trim();
    if constraint.starts_with(">=") {
        let c = parse_semver(constraint.trim_start_matches(">="))?;
        return Ok(semver_compare(version, &c.to_string())? >= 0);
    }
    if constraint.starts_with("<=") {
        let c = parse_semver(constraint.trim_start_matches("<="))?;
        return Ok(semver_compare(version, &c.to_string())? <= 0);
    }
    if constraint.starts_with('>') {
        let c = parse_semver(constraint.trim_start_matches('>'))?;
        return Ok(semver_compare(version, &c.to_string())? > 0);
    }
    if constraint.starts_with('<') {
        let c = parse_semver(constraint.trim_start_matches('<'))?;
        return Ok(semver_compare(version, &c.to_string())? < 0);
    }
    if constraint.starts_with('^') {
        // Caret: compatible with (same major, >= minor.patch)
        let c = parse_semver(constraint.trim_start_matches('^'))?;
        return Ok(v.major == c.major && (v.minor > c.minor || (v.minor == c.minor && v.patch >= c.patch)));
    }
    if constraint.starts_with('~') {
        // Tilde: same major.minor, >= patch
        let c = parse_semver(constraint.trim_start_matches('~'))?;
        return Ok(v.major == c.major && v.minor == c.minor && v.patch >= c.patch);
    }
    if constraint.starts_with('=') {
        let c = parse_semver(constraint.trim_start_matches('='))?;
        return Ok(semver_compare(version, &c.to_string())? == 0);
    }
    // Exact match
    Ok(semver_compare(version, constraint)? == 0)
}

/// Next version: bump major, minor, or patch.
pub fn semver_bump(version: &str, bump: &str) -> Result<String, String> {
    let mut v = parse_semver(version)?;
    match bump.to_lowercase().as_str() {
        "major" => { v.major += 1; v.minor = 0; v.patch = 0; }
        "minor" => { v.minor += 1; v.patch = 0; }
        "patch" => { v.patch += 1; }
        _ => return Err(format!("unknown bump: {bump}. Use major/minor/patch")),
    }
    v.pre = String::new();
    v.build = String::new();
    Ok(v.to_string())
}

// ═══════════════════════════════════════
// Encoding / Hashing
// ═══════════════════════════════════════

/// Base64 encode (standard alphabet).
#[must_use]
pub fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    let mut i = 0;
    while i + 2 < input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i+1] as u32) << 8) | (input[i+2] as u32);
        output.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        output.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        output.push(TABLE[((n >>  6) & 0x3F) as usize] as char);
        output.push(TABLE[( n        & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        output.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        output.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        output.push_str("==");
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i+1] as u32) << 8);
        output.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        output.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        output.push(TABLE[((n >>  6) & 0x3F) as usize] as char);
        output.push('=');
    }
    output
}

/// Hex encode bytes.
#[must_use]
pub fn hex_encode(input: &[u8]) -> String {
    input.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hex decode string to bytes.
pub fn hex_decode(input: &str) -> Result<Vec<u8>, String> {
    let s = input.trim();
    if s.len() % 2 != 0 { return Err("odd-length hex string".to_string()); }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i+2], 16)
            .map_err(|e| format!("invalid hex at position {i}: {e}")))
        .collect()
}

/// Simple DJB2 hash (non-cryptographic, for identifiers/routing keys).
#[must_use]
pub fn djb2_hash(input: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

/// FNV-1a 64-bit hash (non-cryptographic).
#[must_use]
pub fn fnv1a_hash(input: &str) -> u64 {
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;
    let mut hash = FNV_OFFSET;
    for byte in input.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ═══════════════════════════════════════
// Diff & Code Metrics
// ═══════════════════════════════════════

/// Count changed lines between two text versions (simple diff).
#[derive(Debug, Serialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub unchanged: usize,
    pub total_lines_a: usize,
    pub total_lines_b: usize,
}

#[must_use]
pub fn diff_stats(a: &str, b: &str) -> DiffStats {
    let lines_a: std::collections::HashSet<&str> = a.lines().collect();
    let lines_b: std::collections::HashSet<&str> = b.lines().collect();
    let total_a = a.lines().count();
    let total_b = b.lines().count();
    let common = lines_a.intersection(&lines_b).count();
    DiffStats {
        additions: total_b.saturating_sub(common),
        deletions: total_a.saturating_sub(common),
        unchanged: common,
        total_lines_a: total_a,
        total_lines_b: total_b,
    }
}

/// Count lines of code (non-empty, non-comment) in text.
#[must_use]
pub fn count_loc(source: &str, comment_prefix: &str) -> usize {
    source.lines()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty() && !trimmed.starts_with(comment_prefix)
        })
        .count()
}

/// Estimate cyclomatic complexity from keywords (rough heuristic).
#[must_use]
pub fn cyclomatic_complexity_hint(source: &str) -> usize {
    let keywords = ["if ", "else if ", "while ", "for ", "match ", "case ",
                    "catch ", "&&", "||", "?", "=>"];
    let lower = source.to_lowercase();
    1 + keywords.iter().map(|kw| lower.matches(kw).count()).sum::<usize>()
}

/// Validate JSON string.
#[must_use]
pub fn json_validate(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text).is_ok()
}

/// Validate URL.
#[must_use]
pub fn url_validate(text: &str) -> bool {
    text.starts_with("http://") || text.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn parse_semver_works() {
        let v = parse_semver("1.2.3-alpha+build123").unwrap();
        assert_eq!(v.major, 1); assert_eq!(v.minor, 2); assert_eq!(v.patch, 3);
        assert_eq!(v.pre, "alpha"); assert_eq!(v.build, "build123");
    }
    #[test] fn semver_compare_works() {
        assert_eq!(semver_compare("1.2.3", "1.2.4").unwrap(), -1);
        assert_eq!(semver_compare("2.0.0", "1.9.9").unwrap(),  1);
        assert_eq!(semver_compare("1.0.0", "1.0.0").unwrap(),  0);
    }
    #[test] fn semver_satisfies_caret() {
        assert!(semver_satisfies("1.3.0", "^1.2.0").unwrap());
        assert!(!semver_satisfies("2.0.0", "^1.2.0").unwrap());
    }
    #[test] fn semver_bump_works() {
        assert_eq!(semver_bump("1.2.3", "patch").unwrap(), "1.2.4");
        assert_eq!(semver_bump("1.2.3", "minor").unwrap(), "1.3.0");
        assert_eq!(semver_bump("1.2.3", "major").unwrap(), "2.0.0");
    }
    #[test] fn base64_encode_known() {
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b"M"), "TQ==");
    }
    #[test] fn hex_roundtrip() {
        let orig = b"hello world";
        let encoded = hex_encode(orig);
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(decoded, orig);
    }
    #[test] fn djb2_deterministic() {
        assert_eq!(djb2_hash("hello"), djb2_hash("hello"));
        assert_ne!(djb2_hash("hello"), djb2_hash("world"));
    }
    #[test] fn diff_stats_works() {
        let a = "line1\nline2\nline3";
        let b = "line1\nline3\nline4";
        let d = diff_stats(a, b);
        assert!(d.additions > 0);
    }
}
