//! Computational complexity analysis and algorithm properties — pure deterministic functions.

// ═══════════════════════════════════════
// Complexity Classification
// ═══════════════════════════════════════

/// Infer a Big-O class from the ratio of `ops` to `n`.
///
/// Uses fixed thresholds on the ratio ops/n (and ops alone for O(1)):
/// - O(1)        : ops ≤ 1 or n == 0
/// - O(log n)    : ops ≤ log2(n) * 4
/// - O(n)        : ops ≤ n * 4
/// - O(n log n)  : ops ≤ n * log2(n) * 4
/// - O(n²)       : ops ≤ n² * 4
/// - O(2ⁿ)       : otherwise
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn complexity_class(n: u64, ops: u64) -> &'static str {
    if n == 0 || ops <= 1 {
        return "O(1)";
    }
    let nf = n as f64;
    let of = ops as f64;
    let log_n = nf.log2().max(1.0);

    if of <= log_n * 4.0 {
        "O(log n)"
    } else if of <= nf * 4.0 {
        "O(n)"
    } else if of <= nf * log_n * 4.0 {
        "O(n log n)"
    } else if of <= nf * nf * 4.0 {
        "O(n²)"
    } else {
        "O(2ⁿ)"
    }
}

/// Returns `true` if `ops` grows no faster than `n^10`.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn is_polynomial(n: u64, ops: u64) -> bool {
    if n <= 1 { return true; }
    let threshold = (n as f64).powi(10);
    (ops as f64) <= threshold
}

/// Project operation count for a new input size given a power-law exponent.
///
/// `ops(target_n) ≈ base_ops * (target_n / base_n) ^ exponent`
///
/// Returns 0 if `base_n` is 0.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn ops_at_scale(base_n: u64, base_ops: u64, target_n: u64, exponent: f64) -> u64 {
    if base_n == 0 { return 0; }
    let ratio = target_n as f64 / base_n as f64;
    let scaled = base_ops as f64 * ratio.powf(exponent);
    scaled.round() as u64
}

/// Ceiling of log base 2.
///
/// Returns 0 for n == 0 or n == 1.
/// Equivalent to the number of bits needed to represent n distinct values.
#[must_use]
pub fn log2_ceil(n: u64) -> u32 {
    if n <= 1 {
        return 0;
    }
    // u64::BITS - leading_zeros gives floor(log2(n)) + 1 when n is not a power of two,
    // and exactly log2(n) when it is.
    let floor = u64::BITS - n.leading_zeros() - 1; // floor(log2(n))
    if n & (n - 1) == 0 {
        floor // exact power of two
    } else {
        floor + 1
    }
}

/// Expected number of comparisons for a successful binary search over `n` elements.
///
/// Formula: floor(log2(n)) + 1
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn expected_comparisons_binary_search(n: u64) -> f64 {
    if n == 0 { return 0.0; }
    (n as f64).log2().floor() + 1.0
}

// ═══════════════════════════════════════
// State Machine
// ═══════════════════════════════════════

/// Returns `true` if no two transitions share the same `(state, input)` pair.
///
/// A deterministic FSM requires that each (state, symbol) pair maps to at most
/// one next state.
#[must_use]
pub fn is_deterministic_fsm(transitions: &[(u32, char, u32)]) -> bool {
    // Check for duplicate (state, input) pairs
    let mut seen: Vec<(u32, char)> = Vec::with_capacity(transitions.len());
    for &(state, ch, _) in transitions {
        let key = (state, ch);
        if seen.contains(&key) {
            return false;
        }
        seen.push(key);
    }
    true
}

/// BFS from `start`; returns all reachable states (sorted, deduplicated).
#[must_use]
pub fn fsm_reachable_states(transitions: &[(u32, char, u32)], start: u32) -> Vec<u32> {
    let mut visited: Vec<u32> = Vec::new();
    let mut queue: Vec<u32> = vec![start];
    while let Some(current) = queue.first().copied() {
        queue.remove(0);
        if visited.contains(&current) {
            continue;
        }
        visited.push(current);
        for &(from, _, to) in transitions {
            if from == current && !visited.contains(&to) && !queue.contains(&to) {
                queue.push(to);
            }
        }
    }
    visited.sort_unstable();
    visited
}

/// Simulate the FSM on `input`; returns `true` if the machine ends in an accept state.
///
/// Starts at `start`. If a character has no transition from the current state,
/// the machine rejects immediately.
#[must_use]
pub fn fsm_accepts(
    transitions: &[(u32, char, u32)],
    accept: &[u32],
    start: u32,
    input: &str,
) -> bool {
    let mut state = start;
    for ch in input.chars() {
        match transitions.iter().find(|&&(s, c, _)| s == state && c == ch) {
            Some(&(_, _, next)) => state = next,
            None => return false,
        }
    }
    accept.contains(&state)
}

/// Returns the total number of transition rules.
#[must_use]
pub fn transition_count(transitions: &[(u32, char, u32)]) -> usize {
    transitions.len()
}

// ═══════════════════════════════════════
// Algorithm Properties
// ═══════════════════════════════════════

/// Returns `true` if `data` is non-decreasingly sorted.
#[must_use]
pub fn is_sorted(data: &[i64]) -> bool {
    data.windows(2).all(|w| w[0] <= w[1])
}

/// Counts the number of inversions in `data`: pairs `(i, j)` where `i < j`
/// but `data[i] > data[j]`. O(n²) brute-force.
#[must_use]
pub fn inversion_count(data: &[i64]) -> u64 {
    let n = data.len();
    let mut count = 0u64;
    for i in 0..n {
        for j in (i + 1)..n {
            if data[i] > data[j] {
                count += 1;
            }
        }
    }
    count
}

/// Lossless compression ratio: `original / compressed`.
///
/// Returns `0.0` if `compressed_bytes` is 0 to avoid division by zero.
/// Named `algo_compression_ratio` to avoid collision with `information::compression_ratio`.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn algo_compression_ratio(original_bytes: usize, compressed_bytes: usize) -> f64 {
    if compressed_bytes == 0 { return 0.0; }
    original_bytes as f64 / compressed_bytes as f64
}

/// Speedup ratio: `serial_time / parallel_time`.
///
/// Returns `0.0` if `parallel_time` is zero or negative.
#[must_use]
pub fn speedup_ratio(serial_time: f64, parallel_time: f64) -> f64 {
    if parallel_time <= 0.0 {
        return 0.0;
    }
    serial_time / parallel_time
}

/// Amdahl's Law speedup: `1 / ((1 - p) + p / n)`
///
/// - `parallel_fraction`: fraction of work that can be parallelised (0.0–1.0).
/// - `num_processors`: number of parallel processors.
///
/// Returns `1.0` if `num_processors` is 0.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn amdahl_speedup(parallel_fraction: f64, num_processors: u64) -> f64 {
    if num_processors == 0 { return 1.0; }
    let p = parallel_fraction.clamp(0.0, 1.0);
    let n = num_processors as f64;
    1.0 / ((1.0 - p) + p / n)
}

// ═══════════════════════════════════════
// Resource Estimation
// ═══════════════════════════════════════

/// Bytes required for an n×n adjacency matrix (1 byte per cell).
///
/// Returns `n²`.
#[must_use]
pub fn memory_for_adjacency_matrix(nodes: u64) -> u64 {
    nodes.saturating_mul(nodes)
}

/// Bytes required for an adjacency list representation.
///
/// Each node pointer and each edge endpoint is stored as an 8-byte word:
/// `(nodes + edges) * 8`.
#[must_use]
pub fn memory_for_adjacency_list(nodes: u64, edges: u64) -> u64 {
    nodes.saturating_add(edges).saturating_mul(8)
}

/// Number of items of `item_bytes` that fit in one cache line of `cache_line_bytes`.
///
/// Returns 0 if either argument is 0.
#[must_use]
pub fn cache_line_fits(item_bytes: u64, cache_line_bytes: u64) -> u64 {
    if item_bytes == 0 || cache_line_bytes == 0 {
        return 0;
    }
    cache_line_bytes / item_bytes
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Complexity Classification ──────────────────────────────────────────

    #[test]
    fn test_complexity_class_constant() {
        assert_eq!(complexity_class(0, 5), "O(1)");
        assert_eq!(complexity_class(1_000_000, 1), "O(1)");
        assert_eq!(complexity_class(1_000_000, 0), "O(1)");
    }

    #[test]
    fn test_complexity_class_linear_and_log() {
        // ops ≈ n → O(n)
        assert_eq!(complexity_class(1000, 1000), "O(n)");
        // ops ≈ log2(n) ≈ 10 for n=1024 → O(log n)
        assert_eq!(complexity_class(1024, 10), "O(log n)");
        // ops ≈ n² → O(n²)
        assert_eq!(complexity_class(100, 10_000), "O(n²)");
        // ops >> n² → O(2ⁿ)
        assert_eq!(complexity_class(10, 100_000), "O(2ⁿ)");
    }

    #[test]
    fn test_is_polynomial() {
        assert!(is_polynomial(0, 999));
        assert!(is_polynomial(1, u64::MAX));
        assert!(is_polynomial(2, 1024));       // 2^10 ≤ 2^10
        assert!(!is_polynomial(10, u64::MAX)); // u64::MAX >> 10^10
    }

    #[test]
    fn test_ops_at_scale() {
        // Doubling n with exponent=2 should quadruple ops
        let result = ops_at_scale(100, 10_000, 200, 2.0);
        assert_eq!(result, 40_000);
        // base_n == 0 → 0
        assert_eq!(ops_at_scale(0, 100, 200, 1.0), 0);
        // Same scale → same ops
        assert_eq!(ops_at_scale(50, 500, 50, 3.0), 500);
    }

    #[test]
    fn test_log2_ceil() {
        assert_eq!(log2_ceil(0), 0);
        assert_eq!(log2_ceil(1), 0);
        assert_eq!(log2_ceil(2), 1);
        assert_eq!(log2_ceil(3), 2);
        assert_eq!(log2_ceil(4), 2);
        assert_eq!(log2_ceil(8), 3);
        assert_eq!(log2_ceil(9), 4);
        assert_eq!(log2_ceil(1024), 10);
        assert_eq!(log2_ceil(1025), 11);
    }

    #[test]
    fn test_expected_comparisons_binary_search() {
        assert_eq!(expected_comparisons_binary_search(0), 0.0);
        assert_eq!(expected_comparisons_binary_search(1), 1.0);   // log2(1)+1 = 1
        assert_eq!(expected_comparisons_binary_search(8), 4.0);   // log2(8)+1 = 4
        assert_eq!(expected_comparisons_binary_search(16), 5.0);  // log2(16)+1 = 5
    }

    // ── State Machine ──────────────────────────────────────────────────────

    #[test]
    fn test_is_deterministic_fsm() {
        let det = &[(0, 'a', 1), (0, 'b', 2), (1, 'a', 1)];
        assert!(is_deterministic_fsm(det));
        let non_det = &[(0, 'a', 1), (0, 'a', 2)];
        assert!(!is_deterministic_fsm(non_det));
        assert!(is_deterministic_fsm(&[]));
    }

    #[test]
    fn test_fsm_reachable_states() {
        // 0 --a--> 1 --b--> 2; state 3 unreachable
        let t = &[(0, 'a', 1), (1, 'b', 2), (3, 'c', 0)];
        let reachable = fsm_reachable_states(t, 0);
        assert_eq!(reachable, vec![0, 1, 2]);
        // Isolated start
        assert_eq!(fsm_reachable_states(&[], 5), vec![5]);
    }

    #[test]
    fn test_fsm_accepts() {
        // Simple FSM: 0 --a--> 1 --b--> 2, accept = {2}
        let t = &[(0, 'a', 1), (1, 'b', 2)];
        let accept = &[2u32];
        assert!( fsm_accepts(t, accept, 0, "ab"));
        assert!(!fsm_accepts(t, accept, 0, "a"));   // stops at state 1 (not accept)
        assert!(!fsm_accepts(t, accept, 0, "ba"));  // no 'b' from state 0
        assert!(!fsm_accepts(t, accept, 0, ""));    // empty: stays at 0 (not accept)
        // start already in accept state, empty input
        let accept2 = &[0u32];
        assert!(fsm_accepts(t, accept2, 0, ""));
    }

    #[test]
    fn test_transition_count() {
        assert_eq!(transition_count(&[]), 0);
        assert_eq!(transition_count(&[(0, 'a', 1), (1, 'b', 2)]), 2);
    }

    // ── Algorithm Properties ───────────────────────────────────────────────

    #[test]
    fn test_is_sorted() {
        assert!( is_sorted(&[]));
        assert!( is_sorted(&[1]));
        assert!( is_sorted(&[1, 2, 3]));
        assert!( is_sorted(&[1, 1, 2]));
        assert!(!is_sorted(&[3, 2, 1]));
        assert!(!is_sorted(&[1, 3, 2]));
    }

    #[test]
    fn test_inversion_count() {
        assert_eq!(inversion_count(&[]), 0);
        assert_eq!(inversion_count(&[1, 2, 3]), 0);     // sorted
        assert_eq!(inversion_count(&[3, 2, 1]), 3);     // max inversions for 3 elements
        assert_eq!(inversion_count(&[2, 1, 3]), 1);     // one inversion: (2,1)
        assert_eq!(inversion_count(&[1, 3, 2, 4]), 1);  // one inversion: (3,2)
    }

    #[test]
    fn test_compression_ratio() {
        let r = algo_compression_ratio(1000, 400);
        assert!((r - 2.5).abs() < 1e-10);
        assert_eq!(algo_compression_ratio(100, 0), 0.0);
        let identity = algo_compression_ratio(500, 500);
        assert!((identity - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_speedup_ratio() {
        let s = speedup_ratio(8.0, 2.0);
        assert!((s - 4.0).abs() < 1e-10);
        assert_eq!(speedup_ratio(10.0, 0.0), 0.0);
        assert_eq!(speedup_ratio(10.0, -1.0), 0.0);
    }

    #[test]
    fn test_amdahl_speedup() {
        // 100% parallel, 4 processors → speedup = 4
        let s = amdahl_speedup(1.0, 4);
        assert!((s - 4.0).abs() < 1e-10, "got {s}");
        // 0% parallel → always 1.0
        let s0 = amdahl_speedup(0.0, 100);
        assert!((s0 - 1.0).abs() < 1e-10);
        // 50% parallel, 2 processors → 1 / (0.5 + 0.25) = 1.333...
        let s50 = amdahl_speedup(0.5, 2);
        assert!((s50 - (4.0 / 3.0)).abs() < 1e-10, "got {s50}");
        // 0 processors → 1.0
        assert_eq!(amdahl_speedup(0.9, 0), 1.0);
    }

    // ── Resource Estimation ────────────────────────────────────────────────

    #[test]
    fn test_memory_adjacency() {
        assert_eq!(memory_for_adjacency_matrix(4), 16);
        assert_eq!(memory_for_adjacency_matrix(0), 0);
        assert_eq!(memory_for_adjacency_list(4, 6), (4 + 6) * 8);
        assert_eq!(memory_for_adjacency_list(0, 0), 0);
    }

    #[test]
    fn test_cache_line_fits() {
        assert_eq!(cache_line_fits(8, 64), 8);   // 8 u64s per 64-byte cache line
        assert_eq!(cache_line_fits(4, 64), 16);  // 16 i32s
        assert_eq!(cache_line_fits(0, 64), 0);
        assert_eq!(cache_line_fits(8, 0), 0);
        assert_eq!(cache_line_fits(100, 64), 0); // item larger than cache line
    }
}
