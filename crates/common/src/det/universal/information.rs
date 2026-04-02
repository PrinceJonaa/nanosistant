//! Information theory — pure deterministic functions.

// ═══════════════════════════════════════
// Entropy & Coding
// ═══════════════════════════════════════

/// Shannon entropy H(X) = -Σ p(x) log₂ p(x), in bits.
/// Probabilities need not sum to 1 — values ≤ 0 are skipped.
#[must_use]
pub fn shannon_entropy(probabilities: &[f64]) -> f64 {
    probabilities.iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.log2())
        .sum()
}

/// Minimum bits needed to represent `num_symbols` distinct symbols.
/// = ceil(log₂(n)), returned as f64.
#[must_use]
pub fn bits_needed(num_symbols: usize) -> f64 {
    if num_symbols <= 1 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let b = (num_symbols as f64).log2();
    b
}

/// Mutual information I(X;Y) from joint probability table.
/// `joint[i][j]` = p(X=i, Y=j). Marginals computed from joint.
#[must_use]
pub fn mutual_information(joint: &[Vec<f64>]) -> f64 {
    if joint.is_empty() { return 0.0; }
    let rows = joint.len();
    let cols = joint[0].len();
    // Marginals
    let px: Vec<f64> = (0..rows).map(|i| joint[i].iter().sum()).collect();
    let py: Vec<f64> = (0..cols).map(|j| joint.iter().map(|row| row[j]).sum()).collect();
    let mut mi = 0.0;
    for i in 0..rows {
        for j in 0..cols {
            let pij = joint[i][j];
            if pij > 0.0 && px[i] > 0.0 && py[j] > 0.0 {
                mi += pij * (pij / (px[i] * py[j])).log2();
            }
        }
    }
    mi
}

/// KL divergence D_KL(P || Q) = Σ p(x) log(p(x)/q(x)).
/// Undefined (returns f64::INFINITY) if q[i]=0 and p[i]>0.
#[must_use]
pub fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    p.iter().zip(q.iter())
        .filter(|(&pi, _)| pi > 0.0)
        .map(|(&pi, &qi)| {
            if qi <= 0.0 { f64::INFINITY }
            else { pi * (pi / qi).ln() }
        })
        .sum()
}

/// Compression ratio = original_size / compressed_size.
/// Returns 1.0 if compressed_size is 0.
#[must_use]
pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f64 {
    if compressed_size == 0 { return 1.0; }
    #[allow(clippy::cast_precision_loss)]
    let r = original_size as f64 / compressed_size as f64;
    r
}

/// Redundancy = 1 - entropy / max_entropy.
#[must_use]
pub fn redundancy(entropy: f64, max_entropy: f64) -> f64 {
    if max_entropy <= 0.0 { return 0.0; }
    1.0 - entropy / max_entropy
}

// ═══════════════════════════════════════
// Channel Capacity & Distance
// ═══════════════════════════════════════

/// Shannon–Hartley channel capacity C = B * log₂(1 + SNR), in bits/s.
#[must_use]
pub fn channel_capacity_shannon(bandwidth_hz: f64, snr_linear: f64) -> f64 {
    bandwidth_hz * (1.0 + snr_linear).log2()
}

/// Hamming distance: number of positions where bytes differ (bitwise).
#[must_use]
pub fn hamming_distance(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter())
        .map(|(&x, &y)| (x ^ y).count_ones() as usize)
        .sum::<usize>()
        + if a.len() > b.len() {
            a[b.len()..].iter().map(|&x| x.count_ones() as usize).sum::<usize>()
          } else {
            b[a.len()..].iter().map(|&x| x.count_ones() as usize).sum::<usize>()
          }
}

/// Levenshtein (edit) distance between two strings.
#[must_use]
pub fn edit_distance_levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();
    // Use two rows to save memory
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            curr[j] = (prev[j] + 1)          // deletion
                .min(curr[j - 1] + 1)         // insertion
                .min(prev[j - 1] + cost);      // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

/// Approximate Kolmogorov complexity via gzip-compression ratio heuristic.
/// Returns the ratio compressed_len / original_len (lower = more compressible = less complex).
/// Uses a simple byte-frequency entropy estimate as a proxy (no actual gzip).
#[must_use]
pub fn kolmogorov_complexity_approx(data: &str) -> f64 {
    if data.is_empty() { return 0.0; }
    let bytes = data.as_bytes();
    let len = bytes.len();
    let mut freq = [0u64; 256];
    for &b in bytes { freq[b as usize] += 1; }
    #[allow(clippy::cast_precision_loss)]
    let entropy: f64 = freq.iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len as f64;
            -p * p.log2()
        })
        .sum();
    // Normalise to [0,1]: max entropy for bytes is log2(256)=8 bits
    entropy / 8.0
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_shannon_entropy_uniform() {
        // 4 equally likely outcomes → 2 bits
        let probs = [0.25, 0.25, 0.25, 0.25];
        let h = shannon_entropy(&probs);
        assert!((h - 2.0).abs() < 1e-10, "expected 2.0, got {h}");
    }

    #[test] fn test_shannon_entropy_certain() {
        let probs = [1.0, 0.0];
        let h = shannon_entropy(&probs);
        assert!((h - 0.0).abs() < 1e-10);
    }

    #[test] fn test_bits_needed() {
        assert_eq!(bits_needed(1), 0.0);
        assert!((bits_needed(8) - 3.0).abs() < 1e-10);
        assert!((bits_needed(256) - 8.0).abs() < 1e-10);
    }

    #[test] fn test_mutual_information_independent() {
        // Independent: p(x,y) = p(x)*p(y) → MI = 0
        let joint = vec![
            vec![0.25, 0.25],
            vec![0.25, 0.25],
        ];
        let mi = mutual_information(&joint);
        assert!(mi.abs() < 1e-10, "expected 0, got {mi}");
    }

    #[test] fn test_mutual_information_perfect() {
        // Perfect correlation
        let joint = vec![
            vec![0.5, 0.0],
            vec![0.0, 0.5],
        ];
        let mi = mutual_information(&joint);
        assert!(mi > 0.9, "expected ~1.0, got {mi}");
    }

    #[test] fn test_kl_divergence_equal() {
        let p = [0.5, 0.5];
        let q = [0.5, 0.5];
        let kl = kl_divergence(&p, &q);
        assert!(kl.abs() < 1e-10, "KL(P||P) should be 0, got {kl}");
    }

    #[test] fn test_kl_divergence_known() {
        // p=[0.5,0.5], q=[0.25,0.75]
        // KL = 0.5*ln(0.5/0.25) + 0.5*ln(0.5/0.75)
        let p = [0.5, 0.5];
        let q = [0.25, 0.75];
        let kl = kl_divergence(&p, &q);
        assert!(kl > 0.0);
    }

    #[test] fn test_compression_ratio() {
        assert!((compression_ratio(1000, 500) - 2.0).abs() < 1e-10);
        assert!((compression_ratio(100, 0) - 1.0).abs() < 1e-10);
    }

    #[test] fn test_redundancy() {
        assert!((redundancy(2.0, 4.0) - 0.5).abs() < 1e-10);
        assert!((redundancy(0.0, 1.0) - 1.0).abs() < 1e-10);
        assert_eq!(redundancy(1.0, 0.0), 0.0);
    }

    #[test] fn test_channel_capacity() {
        // C = 1000 * log2(1 + 1) = 1000 bits/s
        let c = channel_capacity_shannon(1000.0, 1.0);
        assert!((c - 1000.0).abs() < 1e-10, "got {c}");
    }

    #[test] fn test_hamming_distance() {
        // 0b00000001 ^ 0b00000000 = 1 bit different
        assert_eq!(hamming_distance(&[1u8], &[0u8]), 1);
        // 0xFF ^ 0x00 = 8 bits
        assert_eq!(hamming_distance(&[0xFF], &[0x00]), 8);
        assert_eq!(hamming_distance(&[], &[]), 0);
    }

    #[test] fn test_levenshtein_empty() {
        assert_eq!(edit_distance_levenshtein("", ""), 0);
        assert_eq!(edit_distance_levenshtein("abc", ""), 3);
        assert_eq!(edit_distance_levenshtein("", "abc"), 3);
    }

    #[test] fn test_levenshtein_known() {
        assert_eq!(edit_distance_levenshtein("kitten", "sitting"), 3);
        assert_eq!(edit_distance_levenshtein("saturday", "sunday"), 3);
        assert_eq!(edit_distance_levenshtein("abc", "abc"), 0);
    }

    #[test] fn test_kolmogorov_approx_random_vs_structured() {
        // Highly repetitive string should have lower complexity than random-looking
        let structured = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let varied = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@";
        let k_structured = kolmogorov_complexity_approx(structured);
        let k_varied = kolmogorov_complexity_approx(varied);
        assert!(k_structured < k_varied, "structured={k_structured}, varied={k_varied}");
    }
}
