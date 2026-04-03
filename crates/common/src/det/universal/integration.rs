//! M-lens (Integration) — pure deterministic functions for combining outputs
//! from multiple deterministic modules: signal fusion, weighted aggregation,
//! cross-lens scoring, and multi-domain similarity.

// ═══════════════════════════════════════
// Weighted Aggregation
// ═══════════════════════════════════════

/// Weighted mean: Σ(w_i * v_i) / Σ(w_i).
/// Returns None if lengths differ or weights sum to 0.
#[must_use]
pub fn weighted_mean(values: &[f64], weights: &[f64]) -> Option<f64> {
    if values.len() != weights.len() {
        return None;
    }
    let weight_sum: f64 = weights.iter().sum();
    if weight_sum == 0.0 {
        return None;
    }
    let numerator: f64 = values.iter().zip(weights.iter()).map(|(v, w)| v * w).sum();
    Some(numerator / weight_sum)
}

/// Weighted sum: Σ(w_i * v_i).
/// Returns None if lengths differ.
#[must_use]
pub fn weighted_sum(values: &[f64], weights: &[f64]) -> Option<f64> {
    if values.len() != weights.len() {
        return None;
    }
    Some(values.iter().zip(weights.iter()).map(|(v, w)| v * w).sum())
}

/// Normalize weights so they sum to 1.0.
/// If all weights are 0 or the slice is empty, returns a uniform distribution.
#[must_use]
pub fn normalize_weights(weights: &[f64]) -> Vec<f64> {
    if weights.is_empty() {
        return Vec::new();
    }
    let sum: f64 = weights.iter().sum();
    if sum == 0.0 {
        #[allow(clippy::cast_precision_loss)]
        let uniform = 1.0 / weights.len() as f64;
        return vec![uniform; weights.len()];
    }
    weights.iter().map(|&w| w / sum).collect()
}

/// Softmax: exp(x_i) / Σ exp(x_j).
/// Uses the numerically stable max-subtraction trick.
/// Returns an empty Vec if input is empty.
#[must_use]
pub fn softmax(logits: &[f64]) -> Vec<f64> {
    if logits.is_empty() {
        return Vec::new();
    }
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = logits.iter().map(|&x| (x - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

/// Geometric mean: (Π v_i)^(1/n).
/// Returns None if the slice is empty or any value is <= 0.
#[must_use]
pub fn geometric_mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    if values.iter().any(|&v| v <= 0.0) {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let log_mean: f64 = values.iter().map(|&v| v.ln()).sum::<f64>() / values.len() as f64;
    Some(log_mean.exp())
}

// ═══════════════════════════════════════
// Signal Fusion
// ═══════════════════════════════════════

/// Majority vote over a list of class predictions.
/// Returns the winning class, or None on a tie or if `num_classes` is 0.
#[must_use]
pub fn ensemble_vote(predictions: &[usize], num_classes: usize) -> Option<usize> {
    if predictions.is_empty() || num_classes == 0 {
        return None;
    }
    let mut counts = vec![0usize; num_classes];
    for &p in predictions {
        if p < num_classes {
            counts[p] += 1;
        }
    }
    let max_count = *counts.iter().max()?;
    if max_count == 0 {
        return None;
    }
    // Check for a tie at the maximum
    let winners: Vec<usize> = counts
        .iter()
        .enumerate()
        .filter(|(_, &c)| c == max_count)
        .map(|(i, _)| i)
        .collect();
    if winners.len() == 1 {
        Some(winners[0])
    } else {
        None
    }
}

/// Confidence-weighted vote: sums confidences per class, returns argmax.
/// `predictions` is a list of `(class_index, confidence)` pairs.
/// Returns None if empty or `num_classes` is 0.
#[must_use]
pub fn confidence_weighted_vote(
    predictions: &[(usize, f64)],
    num_classes: usize,
) -> Option<usize> {
    if predictions.is_empty() || num_classes == 0 {
        return None;
    }
    let mut scores = vec![0.0f64; num_classes];
    for &(class, conf) in predictions {
        if class < num_classes {
            scores[class] += conf;
        }
    }
    let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    scores
        .iter()
        .position(|&s| s == max_score)
}

/// Borda count aggregation.
/// Each ranking assigns n-1 points to the top item, n-2 to the second, etc.
/// Returns items sorted best-first (highest total Borda score first).
/// Panics if any item index in a ranking is out of range.
#[must_use]
pub fn borda_count(rankings: &[Vec<usize>]) -> Vec<usize> {
    if rankings.is_empty() {
        return Vec::new();
    }
    // Determine the number of candidates from the union of all rankings
    let num_items = rankings
        .iter()
        .flat_map(|r| r.iter())
        .copied()
        .max()
        .map_or(0, |m| m + 1);
    if num_items == 0 {
        return Vec::new();
    }
    let mut scores = vec![0usize; num_items];
    for ranking in rankings {
        let n = ranking.len();
        for (rank, &item) in ranking.iter().enumerate() {
            if item < num_items {
                scores[item] += n.saturating_sub(rank + 1);
            }
        }
    }
    let mut items: Vec<usize> = (0..num_items).collect();
    items.sort_unstable_by(|&a, &b| scores[b].cmp(&scores[a]));
    items
}

/// Reciprocal Rank Fusion (RRF).
/// score(item) = Σ_r 1 / (k + rank_in_r), where rank is 1-based.
/// Returns items sorted best-first (highest RRF score first).
#[must_use]
pub fn rank_fusion_rrf(rankings: &[Vec<usize>], k: usize) -> Vec<usize> {
    if rankings.is_empty() {
        return Vec::new();
    }
    use std::collections::HashMap;
    let mut scores: HashMap<usize, f64> = HashMap::new();
    for ranking in rankings {
        for (rank_0, &item) in ranking.iter().enumerate() {
            let rank_1 = rank_0 + 1; // 1-based
            let entry = scores.entry(item).or_insert(0.0);
            *entry += 1.0 / (k + rank_1) as f64;
        }
    }
    let mut items: Vec<usize> = scores.keys().copied().collect();
    items.sort_unstable_by(|a, b| {
        scores[b].partial_cmp(&scores[a]).unwrap_or(std::cmp::Ordering::Equal)
    });
    items
}

// ═══════════════════════════════════════
// Score Combination
// ═══════════════════════════════════════

/// Harmonic mean: n / Σ(1/x_i).
/// Returns None if any value is 0 or the slice is empty.
#[must_use]
pub fn harmonic_mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    if values.iter().any(|&v| v == 0.0) {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let n = values.len() as f64;
    let recip_sum: f64 = values.iter().map(|&v| 1.0 / v).sum();
    Some(n / recip_sum)
}

/// Min-max normalization: (x - min) / (max - min).
/// If all values are equal (flat distribution), returns all 0.5.
#[must_use]
pub fn min_max_normalize(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range == 0.0 {
        return vec![0.5; values.len()];
    }
    values.iter().map(|&v| (v - min) / range).collect()
}

/// Clamp a score to [min, max].
#[must_use]
pub fn clamp_score(score: f64, min: f64, max: f64) -> f64 {
    score.clamp(min, max)
}

/// Linear interpolation: a + t*(b-a), with t clamped to [0.0, 1.0].
#[must_use]
pub fn interpolate(a: f64, b: f64, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    a + t * (b - a)
}

/// Sigmoid function: 1 / (1 + e^(-x)).
#[must_use]
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Rectified linear unit: max(0, x).
#[must_use]
pub fn relu(x: f64) -> f64 {
    x.max(0.0)
}

// ═══════════════════════════════════════
// Multi-Domain Similarity
// ═══════════════════════════════════════

/// Cosine similarity: dot(a, b) / (|a| * |b|).
/// Returns None if lengths differ, either vector is empty, or either norm is 0.
#[must_use]
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return None;
    }
    Some(dot / (norm_a * norm_b))
}

/// Jaccard similarity: |A ∩ B| / |A ∪ B|.
/// Returns 1.0 if both sets are empty.
#[must_use]
pub fn jaccard_similarity(a: &[usize], b: &[usize]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    use std::collections::HashSet;
    let set_a: HashSet<usize> = a.iter().copied().collect();
    let set_b: HashSet<usize> = b.iter().copied().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 1.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let r = intersection as f64 / union as f64;
    r
}

/// Overlap coefficient: |A ∩ B| / min(|A|, |B|).
/// Returns 1.0 if either set is empty.
#[must_use]
pub fn overlap_coefficient(a: &[usize], b: &[usize]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 1.0;
    }
    use std::collections::HashSet;
    let set_a: HashSet<usize> = a.iter().copied().collect();
    let set_b: HashSet<usize> = b.iter().copied().collect();
    let intersection = set_a.intersection(&set_b).count();
    let min_size = set_a.len().min(set_b.len());
    #[allow(clippy::cast_precision_loss)]
    let r = intersection as f64 / min_size as f64;
    r
}

/// Euclidean distance: sqrt(Σ (a_i - b_i)^2).
/// Returns None if lengths differ.
#[must_use]
pub fn euclidean_distance(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() {
        return None;
    }
    let sum_sq: f64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();
    Some(sum_sq.sqrt())
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Weighted Aggregation ───

    #[test]
    fn test_weighted_mean_basic() {
        // weights = [1, 2, 3], values = [1, 2, 3]
        // wm = (1 + 4 + 9) / 6 = 14/6 ≈ 2.333
        let wm = weighted_mean(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).unwrap();
        assert!((wm - 14.0 / 6.0).abs() < 1e-10, "got {wm}");
        assert!(weighted_mean(&[1.0], &[1.0, 2.0]).is_none());
        assert!(weighted_mean(&[1.0, 2.0], &[0.0, 0.0]).is_none());
    }

    #[test]
    fn test_weighted_sum_basic() {
        let ws = weighted_sum(&[2.0, 3.0], &[4.0, 5.0]).unwrap();
        assert!((ws - 23.0).abs() < 1e-10, "got {ws}");
        assert!(weighted_sum(&[1.0], &[]).is_none());
    }

    #[test]
    fn test_normalize_weights() {
        let nw = normalize_weights(&[1.0, 2.0, 3.0, 4.0]);
        let sum: f64 = nw.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10, "sum={sum}");
        assert!((nw[0] - 0.1).abs() < 1e-10);
        // All-zero → uniform
        let uniform = normalize_weights(&[0.0, 0.0]);
        assert!((uniform[0] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_softmax_properties() {
        let sm = softmax(&[1.0, 2.0, 3.0]);
        let sum: f64 = sm.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10, "softmax sum={sum}");
        // Highest logit → highest probability
        assert!(sm[2] > sm[1] && sm[1] > sm[0]);
        // Empty input
        assert!(softmax(&[]).is_empty());
    }

    #[test]
    fn test_geometric_mean() {
        // geo_mean(1, 4, 16) = (64)^(1/3) = 4
        let gm = geometric_mean(&[1.0, 4.0, 16.0]).unwrap();
        assert!((gm - 4.0).abs() < 1e-9, "got {gm}");
        // geo_mean(2, 8) = sqrt(16) = 4
        let gm2 = geometric_mean(&[2.0, 8.0]).unwrap();
        assert!((gm2 - 4.0).abs() < 1e-9, "got {gm2}");
        // Negative value → None
        assert!(geometric_mean(&[1.0, -1.0]).is_none());
        assert!(geometric_mean(&[]).is_none());
    }

    // ─── Signal Fusion ───

    #[test]
    fn test_ensemble_vote_clear_winner() {
        // Classes: 0, 1, 1, 1, 2 → class 1 wins
        let winner = ensemble_vote(&[0, 1, 1, 1, 2], 3).unwrap();
        assert_eq!(winner, 1);
    }

    #[test]
    fn test_ensemble_vote_tie_returns_none() {
        // 0, 0, 1, 1 → tie between 0 and 1
        assert!(ensemble_vote(&[0, 0, 1, 1], 2).is_none());
    }

    #[test]
    fn test_confidence_weighted_vote() {
        // class 0: conf 0.3+0.4=0.7, class 1: conf 0.6 → class 0 wins
        let preds = [(0, 0.3), (1, 0.6), (0, 0.4)];
        let winner = confidence_weighted_vote(&preds, 2).unwrap();
        assert_eq!(winner, 0);
    }

    #[test]
    fn test_borda_count() {
        // r1: [A=0, B=1, C=2], r2: [B=0, A=1, C=2]
        // r1 scores: A=2, B=1, C=0
        // r2 scores: B=2, A=1, C=0
        // totals:    A=3, B=3, C=0  → tie at top, but C always last
        let rankings = vec![vec![0usize, 1, 2], vec![1usize, 0, 2]];
        let sorted = borda_count(&rankings);
        // Item 2 (C) must be last
        assert_eq!(sorted.last(), Some(&2));
    }

    #[test]
    fn test_rank_fusion_rrf() {
        // Three lists all ranking item 0 first, k=1
        // item 0: 3 * 1/(1+1) = 1.5
        // item 1: 3 * 1/(1+2) = 1.0
        // item 2: 3 * 1/(1+3) = 0.75
        let rankings = vec![
            vec![0usize, 1, 2],
            vec![0usize, 1, 2],
            vec![0usize, 1, 2],
        ];
        let fused = rank_fusion_rrf(&rankings, 1);
        assert_eq!(fused[0], 0, "item 0 should rank first, got {fused:?}");
        assert_eq!(fused[1], 1, "item 1 should rank second, got {fused:?}");
        assert_eq!(fused[2], 2, "item 2 should rank third, got {fused:?}");
    }

    // ─── Score Combination ───

    #[test]
    fn test_harmonic_mean() {
        // H(1, 1) = 1
        assert!((harmonic_mean(&[1.0, 1.0]).unwrap() - 1.0).abs() < 1e-10);
        // H(1, 4) = 2 / (1 + 0.25) = 1.6
        let h = harmonic_mean(&[1.0, 4.0]).unwrap();
        assert!((h - 1.6).abs() < 1e-10, "got {h}");
        // Zero in list → None
        assert!(harmonic_mean(&[1.0, 0.0]).is_none());
        assert!(harmonic_mean(&[]).is_none());
    }

    #[test]
    fn test_min_max_normalize() {
        let normed = min_max_normalize(&[0.0, 5.0, 10.0]);
        assert!((normed[0] - 0.0).abs() < 1e-10);
        assert!((normed[1] - 0.5).abs() < 1e-10);
        assert!((normed[2] - 1.0).abs() < 1e-10);
        // Flat → all 0.5
        let flat = min_max_normalize(&[3.0, 3.0, 3.0]);
        assert!(flat.iter().all(|&v| (v - 0.5).abs() < 1e-10));
    }

    #[test]
    fn test_interpolate_sigmoid_relu() {
        // Interpolation
        assert!((interpolate(0.0, 10.0, 0.3) - 3.0).abs() < 1e-10);
        // t clamping
        assert!((interpolate(0.0, 10.0, 1.5) - 10.0).abs() < 1e-10);
        assert!((interpolate(0.0, 10.0, -1.0) - 0.0).abs() < 1e-10);
        // Sigmoid at 0 = 0.5
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
        // ReLU
        assert!((relu(-3.0) - 0.0).abs() < 1e-10);
        assert!((relu(5.0) - 5.0).abs() < 1e-10);
    }

    // ─── Multi-Domain Similarity ───

    #[test]
    fn test_cosine_similarity() {
        // Identical vectors → 1.0
        let cs = cosine_similarity(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).unwrap();
        assert!((cs - 1.0).abs() < 1e-10, "got {cs}");
        // Orthogonal → 0.0
        let cs_ortho = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).unwrap();
        assert!(cs_ortho.abs() < 1e-10, "got {cs_ortho}");
        // Zero vector → None
        assert!(cosine_similarity(&[0.0, 0.0], &[1.0, 2.0]).is_none());
        // Length mismatch → None
        assert!(cosine_similarity(&[1.0], &[1.0, 2.0]).is_none());
    }

    #[test]
    fn test_jaccard_and_overlap() {
        // |{0,1,2} ∩ {1,2,3}| = 2, |union| = 4 → J = 0.5
        let j = jaccard_similarity(&[0, 1, 2], &[1, 2, 3]);
        assert!((j - 0.5).abs() < 1e-10, "got {j}");
        // Both empty → 1.0
        assert!((jaccard_similarity(&[], &[]) - 1.0).abs() < 1e-10);
        // Overlap coefficient: min set size = 2, intersection = 2 → 1.0
        let ov = overlap_coefficient(&[1, 2], &[1, 2, 3]);
        assert!((ov - 1.0).abs() < 1e-10, "got {ov}");
    }

    #[test]
    fn test_euclidean_distance() {
        // 3-4-5 right triangle
        let d = euclidean_distance(&[0.0, 0.0], &[3.0, 4.0]).unwrap();
        assert!((d - 5.0).abs() < 1e-10, "got {d}");
        // Length mismatch → None
        assert!(euclidean_distance(&[1.0], &[1.0, 2.0]).is_none());
        // Same point → 0
        let d0 = euclidean_distance(&[5.0, 5.0], &[5.0, 5.0]).unwrap();
        assert!(d0.abs() < 1e-10);
    }
}
