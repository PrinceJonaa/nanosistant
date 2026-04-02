//! Probability and statistics — pure deterministic functions.

use std::f64::consts::PI;

// ═══════════════════════════════════════
// Bayesian
// ═══════════════════════════════════════

/// Bayes' theorem: P(H|E) = P(E|H) * P(H) / P(E).
#[must_use]
pub fn bayes_update(prior: f64, likelihood: f64, marginal: f64) -> f64 {
    if marginal == 0.0 { return 0.0; }
    (likelihood * prior) / marginal
}

// ═══════════════════════════════════════
// Expected Value & Distributions
// ═══════════════════════════════════════

/// E[X] = Σ x_i * p_i. Returns None if lengths differ or probs don't sum to ~1.
#[must_use]
pub fn expected_value(values: &[f64], probabilities: &[f64]) -> Option<f64> {
    if values.len() != probabilities.len() { return None; }
    Some(values.iter().zip(probabilities.iter()).map(|(v, p)| v * p).sum())
}

/// Binomial probability P(X=k) = C(n,k) * p^k * (1-p)^(n-k).
#[must_use]
pub fn binomial_probability(n: u32, k: u32, p: f64) -> f64 {
    if k > n { return 0.0; }
    #[allow(clippy::cast_lossless)]
    let binom = comb_f64(n as u64, k as u64);
    #[allow(clippy::cast_lossless)]
    let prob = binom * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32);
    prob
}

/// Poisson probability P(X=k) = (λ^k * e^(-λ)) / k!
#[must_use]
pub fn poisson_probability(k: u32, lambda: f64) -> f64 {
    if lambda < 0.0 { return 0.0; }
    #[allow(clippy::cast_lossless)]
    let log_prob = k as f64 * lambda.ln() - lambda - log_factorial(k as u64);
    log_prob.exp()
}

/// Standard normal PDF φ(x; μ, σ).
#[must_use]
pub fn normal_pdf(x: f64, mean: f64, std: f64) -> f64 {
    if std <= 0.0 { return 0.0; }
    let z = (x - mean) / std;
    (-0.5 * z * z).exp() / (std * (2.0 * PI).sqrt())
}

/// Approximation of Φ(x) (standard normal CDF) using Hart's rational approximation.
/// Accurate to ~5 significant digits.
#[must_use]
pub fn normal_cdf_approx(x: f64) -> f64 {
    // Abramowitz & Stegun 26.2.17
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let poly = t * (0.319_381_530
        + t * (-0.356_563_782
        + t * (1.781_477_937
        + t * (-1.821_255_978
        + t * 1.330_274_429))));
    let pdf = (-0.5 * x * x).exp() / (2.0 * PI).sqrt();
    let approx = 1.0 - pdf * poly;
    if x >= 0.0 { approx } else { 1.0 - approx }
}

// ═══════════════════════════════════════
// Confidence & Hypothesis Testing
// ═══════════════════════════════════════

/// 95% CI: (mean - 1.96*std/√n, mean + 1.96*std/√n).
#[must_use]
pub fn confidence_interval_95(mean: f64, std: f64, n: usize) -> (f64, f64) {
    if n == 0 { return (mean, mean); }
    #[allow(clippy::cast_precision_loss)]
    let margin = 1.96 * std / (n as f64).sqrt();
    (mean - margin, mean + margin)
}

/// Z-score: (value - mean) / std.
#[must_use]
pub fn z_score(value: f64, mean: f64, std: f64) -> f64 {
    if std == 0.0 { return 0.0; }
    (value - mean) / std
}

/// Two-tailed p-value from a z-score: 2 * (1 - Φ(|z|)).
#[must_use]
pub fn p_value_z(z_score: f64) -> f64 {
    2.0 * (1.0 - normal_cdf_approx(z_score.abs()))
}

/// Minimum sample size for a given margin of error, confidence z, and proportion p.
/// n = (z/margin)^2 * p * (1-p)
#[must_use]
pub fn sample_size_needed(margin: f64, confidence: f64, p: f64) -> usize {
    if margin <= 0.0 { return usize::MAX; }
    // confidence is the z-score (e.g. 1.96 for 95%)
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = ((confidence / margin).powi(2) * p * (1.0 - p)).ceil() as usize;
    n
}

// ═══════════════════════════════════════
// Classification Metrics
// ═══════════════════════════════════════

/// FPR = FP / (FP + TN).
#[must_use]
pub fn false_positive_rate(fp: usize, tn: usize) -> f64 {
    let denom = fp + tn;
    if denom == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let r = fp as f64 / denom as f64;
    r
}

/// FNR = FN / (FN + TP).
#[must_use]
pub fn false_negative_rate(fn_count: usize, tp: usize) -> f64 {
    let denom = fn_count + tp;
    if denom == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let r = fn_count as f64 / denom as f64;
    r
}

/// Precision = TP / (TP + FP).
#[must_use]
pub fn precision(tp: usize, fp: usize) -> f64 {
    let denom = tp + fp;
    if denom == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let r = tp as f64 / denom as f64;
    r
}

/// Recall = TP / (TP + FN).
#[must_use]
pub fn recall(tp: usize, fn_count: usize) -> f64 {
    let denom = tp + fn_count;
    if denom == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let r = tp as f64 / denom as f64;
    r
}

/// F1 = 2 * precision * recall / (precision + recall).
#[must_use]
pub fn f1_score(precision: f64, recall: f64) -> f64 {
    let denom = precision + recall;
    if denom == 0.0 { return 0.0; }
    2.0 * precision * recall / denom
}

// ═══════════════════════════════════════
// Internal helpers
// ═══════════════════════════════════════

fn comb_f64(n: u64, k: u64) -> f64 {
    if k > n { return 0.0; }
    let k = k.min(n - k);
    let mut result = 1.0f64;
    for i in 0..k {
        result *= (n - i) as f64 / (i + 1) as f64;
    }
    result
}

fn log_factorial(n: u64) -> f64 {
    // Stirling approximation for large n, exact for small n
    if n <= 20 {
        return (1..=n).map(|i| (i as f64).ln()).sum();
    }
    let n = n as f64;
    n * n.ln() - n + 0.5 * (2.0 * PI * n).ln()
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_bayes_update() {
        // P(H) = 0.01, P(E|H) = 0.95, P(E) = 0.059
        let post = bayes_update(0.01, 0.95, 0.059);
        assert!((post - 0.161).abs() < 0.001, "got {post}");
    }

    #[test] fn test_expected_value() {
        let ev = expected_value(&[1.0, 2.0, 3.0], &[0.2, 0.5, 0.3]).unwrap();
        assert!((ev - 2.1).abs() < 1e-10, "got {ev}");
        assert!(expected_value(&[1.0], &[1.0, 2.0]).is_none());
    }

    #[test] fn test_binomial_prob() {
        // P(X=3 | n=5, p=0.5) = C(5,3) * 0.5^3 * 0.5^2 = 10/32 = 0.3125
        let p = binomial_probability(5, 3, 0.5);
        assert!((p - 0.3125).abs() < 1e-10, "got {p}");
    }

    #[test] fn test_poisson_prob() {
        // P(X=2 | λ=3) = 9/2 * e^-3 ≈ 0.2240
        let p = poisson_probability(2, 3.0);
        assert!((p - 0.2240).abs() < 0.001, "got {p}");
    }

    #[test] fn test_normal_pdf() {
        // PDF of standard normal at x=0 = 1/sqrt(2π) ≈ 0.3989
        let pdf = normal_pdf(0.0, 0.0, 1.0);
        assert!((pdf - 0.3989).abs() < 0.001, "got {pdf}");
    }

    #[test] fn test_normal_cdf_approx() {
        // Φ(0) = 0.5
        let cdf0 = normal_cdf_approx(0.0);
        assert!((cdf0 - 0.5).abs() < 0.01, "Φ(0)={cdf0}");
        // Φ(1.96) ≈ 0.975
        let cdf196 = normal_cdf_approx(1.96);
        assert!((cdf196 - 0.975).abs() < 0.01, "Φ(1.96)={cdf196}");
    }

    #[test] fn test_confidence_interval_95() {
        let (lo, hi) = confidence_interval_95(100.0, 10.0, 100);
        assert!((lo - 98.04).abs() < 0.1, "lo={lo}");
        assert!((hi - 101.96).abs() < 0.1, "hi={hi}");
    }

    #[test] fn test_z_score() {
        assert!((z_score(110.0, 100.0, 10.0) - 1.0).abs() < 1e-10);
        assert_eq!(z_score(5.0, 5.0, 0.0), 0.0);
    }

    #[test] fn test_p_value_z() {
        // z=1.96 → p ≈ 0.05
        let p = p_value_z(1.96);
        assert!(p < 0.06 && p > 0.04, "got {p}");
    }

    #[test] fn test_sample_size_needed() {
        // margin=0.05, confidence=1.96, p=0.5 → ~385
        let n = sample_size_needed(0.05, 1.96, 0.5);
        assert!(n >= 380 && n <= 400, "got {n}");
    }

    #[test] fn test_false_positive_rate() {
        assert!((false_positive_rate(10, 90) - 0.1).abs() < 1e-10);
        assert_eq!(false_positive_rate(0, 0), 0.0);
    }

    #[test] fn test_false_negative_rate() {
        assert!((false_negative_rate(5, 95) - 0.05).abs() < 1e-10);
    }

    #[test] fn test_precision() {
        assert!((precision(90, 10) - 0.9).abs() < 1e-10);
        assert_eq!(precision(0, 0), 0.0);
    }

    #[test] fn test_recall() {
        assert!((recall(90, 10) - 0.9).abs() < 1e-10);
    }

    #[test] fn test_f1_score() {
        let p = precision(90, 10);
        let r = recall(90, 10);
        let f1 = f1_score(p, r);
        assert!((f1 - 0.9).abs() < 1e-10, "got {f1}");
        assert_eq!(f1_score(0.0, 0.0), 0.0);
    }
}
