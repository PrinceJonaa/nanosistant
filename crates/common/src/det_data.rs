//! Data analysis deterministic functions — pandas-style operations in pure Rust.
//!
//! No external dependencies. All operations work on simple &[f64] or Vec<Vec<f64>>.
//! For actual dataframes, wire to polars/arrow at the integration layer.

use serde::Serialize;
use std::collections::HashMap;

// ═══════════════════════════════════════
// Descriptive Statistics
// ═══════════════════════════════════════

/// Full descriptive statistics for a numeric series.
#[derive(Debug, Serialize)]
pub struct DescribeResult {
    pub count: usize,
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
    pub sum: f64,
    pub variance: f64,
    pub skewness: f64,
}

#[must_use]
pub fn describe(data: &[f64]) -> Option<DescribeResult> {
    if data.is_empty() { return None; }
    let count = data.len();
    let sum: f64 = data.iter().sum();
    let mean = sum / count as f64;
    let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / count as f64;
    let std = variance.sqrt();
    let skewness = if std == 0.0 { 0.0 } else {
        data.iter().map(|x| ((x - mean) / std).powi(3)).sum::<f64>() / count as f64
    };

    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    Some(DescribeResult {
        count,
        mean: r(mean),
        std: r(std),
        min: sorted[0],
        p25: r(percentile_sorted(&sorted, 25.0)),
        median: r(percentile_sorted(&sorted, 50.0)),
        p75: r(percentile_sorted(&sorted, 75.0)),
        p90: r(percentile_sorted(&sorted, 90.0)),
        p95: r(percentile_sorted(&sorted, 95.0)),
        p99: r(percentile_sorted(&sorted, 99.0)),
        max: *sorted.last().unwrap(),
        sum: r(sum),
        variance: r(variance),
        skewness: r(skewness),
    })
}

/// Compute a percentile (0-100) of a data series.
#[must_use]
pub fn percentile(data: &[f64], p: f64) -> Option<f64> {
    if data.is_empty() { return None; }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(r(percentile_sorted(&sorted, p)))
}

fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    if sorted.len() == 1 { return sorted[0]; }
    let idx = (p / 100.0) * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    let frac = idx - lo as f64;
    sorted[lo] + frac * (sorted[hi] - sorted[lo])
}

/// Mean.
#[must_use]
pub fn mean(data: &[f64]) -> Option<f64> {
    if data.is_empty() { return None; }
    Some(r(data.iter().sum::<f64>() / data.len() as f64))
}

/// Median.
#[must_use]
pub fn median(data: &[f64]) -> Option<f64> {
    percentile(data, 50.0)
}

/// Mode (most frequent value, rounded to 4 decimal places for float comparison).
#[must_use]
pub fn mode(data: &[f64]) -> Option<f64> {
    if data.is_empty() { return None; }
    let mut counts: HashMap<i64, (f64, usize)> = HashMap::new();
    for &v in data {
        let key = (v * 10000.0).round() as i64;
        let entry = counts.entry(key).or_insert((v, 0));
        entry.1 += 1;
    }
    counts.into_values().max_by_key(|(_, c)| *c).map(|(v, _)| r(v))
}

/// Standard deviation.
#[must_use]
pub fn std_dev(data: &[f64]) -> Option<f64> {
    if data.len() < 2 { return None; }
    let m = data.iter().sum::<f64>() / data.len() as f64;
    let var = data.iter().map(|x| (x - m).powi(2)).sum::<f64>() / data.len() as f64;
    Some(r(var.sqrt()))
}

/// Inter-quartile range.
#[must_use]
pub fn iqr(data: &[f64]) -> Option<f64> {
    let p75 = percentile(data, 75.0)?;
    let p25 = percentile(data, 25.0)?;
    Some(r(p75 - p25))
}

// ═══════════════════════════════════════
// Frequency & Aggregation
// ═══════════════════════════════════════

/// Frequency count of string items.
#[must_use]
pub fn freq_count(items: &[&str]) -> Vec<(String, usize)> {
    let mut map: HashMap<&str, usize> = HashMap::new();
    for &item in items { *map.entry(item).or_insert(0) += 1; }
    let mut result: Vec<(String, usize)> = map.into_iter()
        .map(|(k, v)| (k.to_string(), v)).collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result
}

/// Value counts (frequency) for numeric data, binned.
#[must_use]
pub fn value_counts_binned(data: &[f64], bins: usize) -> Vec<(String, usize)> {
    if data.is_empty() || bins == 0 { return vec![]; }
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if (max - min).abs() < 1e-10 { return vec![(format!("{min:.4}"), data.len())]; }
    let bin_width = (max - min) / bins as f64;
    let mut counts = vec![0usize; bins];
    for &v in data {
        let bin = ((v - min) / bin_width).floor() as usize;
        let bin = bin.min(bins - 1);
        counts[bin] += 1;
    }
    (0..bins).map(|i| {
        let lo = min + i as f64 * bin_width;
        let hi = lo + bin_width;
        (format!("[{lo:.2}, {hi:.2})"), counts[i])
    }).collect()
}

// ═══════════════════════════════════════
// Normalization & Scaling
// ═══════════════════════════════════════

/// Min-max normalize a series to [0, 1].
#[must_use]
pub fn normalize_minmax(data: &[f64]) -> Vec<f64> {
    if data.is_empty() { return vec![]; }
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range == 0.0 { return vec![0.0; data.len()]; }
    data.iter().map(|&x| r((x - min) / range)).collect()
}

/// Z-score standardization (mean=0, std=1).
#[must_use]
pub fn standardize(data: &[f64]) -> Vec<f64> {
    let m = mean(data).unwrap_or(0.0);
    let s = std_dev(data).unwrap_or(1.0);
    if s == 0.0 { return vec![0.0; data.len()]; }
    data.iter().map(|&x| r((x - m) / s)).collect()
}

/// Clip values to [lo, hi].
#[must_use]
pub fn clip(data: &[f64], lo: f64, hi: f64) -> Vec<f64> {
    data.iter().map(|&x| x.max(lo).min(hi)).collect()
}

/// Rolling window mean (same output length, None-padded as 0.0 at start).
#[must_use]
pub fn rolling_mean(data: &[f64], window: usize) -> Vec<f64> {
    if window == 0 || data.is_empty() { return data.to_vec(); }
    let mut result = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        let start = i.saturating_sub(window - 1);
        let slice = &data[start..=i];
        result.push(r(slice.iter().sum::<f64>() / slice.len() as f64));
    }
    result
}

/// Cumulative sum.
#[must_use]
pub fn cumsum(data: &[f64]) -> Vec<f64> {
    let mut acc = 0.0;
    data.iter().map(|&x| { acc += x; r(acc) }).collect()
}

/// Pairwise correlation coefficient (Pearson).
#[must_use]
pub fn correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.len() < 2 { return None; }
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let cov = x.iter().zip(y.iter()).map(|(a, b)| (a - mx) * (b - my)).sum::<f64>() / n;
    let sx = (x.iter().map(|a| (a - mx).powi(2)).sum::<f64>() / n).sqrt();
    let sy = (y.iter().map(|b| (b - my).powi(2)).sum::<f64>() / n).sqrt();
    if sx == 0.0 || sy == 0.0 { return None; }
    Some(r(cov / (sx * sy)))
}

/// Detect outliers using IQR method (returns indices of outliers).
#[must_use]
pub fn outlier_indices(data: &[f64], multiplier: f64) -> Vec<usize> {
    let q1 = percentile(data, 25.0).unwrap_or(0.0);
    let q3 = percentile(data, 75.0).unwrap_or(0.0);
    let iqr = q3 - q1;
    let lo = q1 - multiplier * iqr;
    let hi = q3 + multiplier * iqr;
    data.iter().enumerate()
        .filter(|(_, &v)| v < lo || v > hi)
        .map(|(i, _)| i)
        .collect()
}

/// Simple linear regression — returns (slope, intercept, r_squared).
#[must_use]
pub fn linear_regression(x: &[f64], y: &[f64]) -> Option<(f64, f64, f64)> {
    if x.len() != y.len() || x.len() < 2 { return None; }
    let n = x.len() as f64;
    let sx: f64 = x.iter().sum();
    let sy: f64 = y.iter().sum();
    let sxy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
    let sx2: f64 = x.iter().map(|a| a.powi(2)).sum();
    let denom = n * sx2 - sx * sx;
    if denom == 0.0 { return None; }
    let slope = (n * sxy - sx * sy) / denom;
    let intercept = (sy - slope * sx) / n;
    // R²
    let y_mean = sy / n;
    let ss_tot: f64 = y.iter().map(|b| (b - y_mean).powi(2)).sum();
    let ss_res: f64 = x.iter().zip(y.iter())
        .map(|(a, b)| (b - (slope * a + intercept)).powi(2)).sum();
    let r2 = if ss_tot == 0.0 { 1.0 } else { 1.0 - ss_res / ss_tot };
    Some((r(slope), r(intercept), r(r2)))
}

// ═══════════════════════════════════════
// Helpers
// ═══════════════════════════════════════

fn r(v: f64) -> f64 { (v * 10000.0).round() / 10000.0 }

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn describe_basic() {
        let d = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let desc = describe(&d).unwrap();
        assert_eq!(desc.count, 5);
        assert!((desc.mean - 3.0).abs() < 0.001);
        assert!((desc.median - 3.0).abs() < 0.001);
        assert_eq!(desc.min, 1.0);
        assert_eq!(desc.max, 5.0);
    }
    #[test] fn percentile_50_is_median() {
        let d = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&d, 50.0).unwrap() - 3.0).abs() < 0.001);
    }
    #[test] fn normalize_minmax_range() {
        let d = vec![0.0, 5.0, 10.0];
        let n = normalize_minmax(&d);
        assert!((n[0] - 0.0).abs() < 0.001);
        assert!((n[1] - 0.5).abs() < 0.001);
        assert!((n[2] - 1.0).abs() < 0.001);
    }
    #[test] fn correlation_perfect() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        assert!((correlation(&x, &y).unwrap() - 1.0).abs() < 0.001);
    }
    #[test] fn linear_regression_works() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let (slope, intercept, r2) = linear_regression(&x, &y).unwrap();
        assert!((slope - 2.0).abs() < 0.001);
        assert!((intercept).abs() < 0.001);
        assert!((r2 - 1.0).abs() < 0.001);
    }
    #[test] fn freq_count_sorted() {
        let items = vec!["a", "b", "a", "c", "a", "b"];
        let fc = freq_count(&items);
        assert_eq!(fc[0].0, "a");
        assert_eq!(fc[0].1, 3);
    }
    #[test] fn rolling_mean_works() {
        let d = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let rm = rolling_mean(&d, 3);
        assert!((rm[2] - 2.0).abs() < 0.001);
        assert!((rm[4] - 4.0).abs() < 0.001);
    }
    #[test] fn cumsum_correct() {
        let d = vec![1.0, 2.0, 3.0];
        let cs = cumsum(&d);
        assert_eq!(cs, vec![1.0, 3.0, 6.0]);
    }
}
