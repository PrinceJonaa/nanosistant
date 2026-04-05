//! Psychometrics, behavioral patterns, and cognitive science — pure deterministic functions.

// ═══════════════════════════════════════
// Psychometric Scoring
// ═══════════════════════════════════════

/// Mean of Likert-scale responses.  Returns `None` if any response is outside `1..=scale`.
#[must_use]
pub fn likert_mean(responses: &[u8], scale: u8) -> Option<f64> {
    if responses.is_empty() {
        return None;
    }
    for &r in responses {
        if r < 1 || r > scale {
            return None;
        }
    }
    #[allow(clippy::cast_precision_loss)]
    Some(responses.iter().map(|&r| r as f64).sum::<f64>() / responses.len() as f64)
}

/// Population standard deviation of Likert-scale responses.
/// Returns `None` if any response is outside `1..=scale`.
#[must_use]
pub fn likert_std(responses: &[u8], scale: u8) -> Option<f64> {
    let mean = likert_mean(responses, scale)?;
    #[allow(clippy::cast_precision_loss)]
    let variance = responses
        .iter()
        .map(|&r| {
            let d = r as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / responses.len() as f64;
    Some(variance.sqrt())
}

/// Cronbach's alpha — internal consistency of a multi-item scale.
///
/// `item_scores[i]` is a vector of scores for item *i* across all respondents.
/// Formula: α = (k / (k-1)) * (1 - Σvar_i / var_total)
///
/// Returns `None` if fewer than 2 items or all respondents have identical totals.
#[must_use]
pub fn cronbach_alpha(item_scores: &[Vec<f64>]) -> Option<f64> {
    let k = item_scores.len();
    if k < 2 {
        return None;
    }
    let n = item_scores[0].len();
    if n < 2 {
        return None;
    }
    // Ensure all items have the same number of responses
    if item_scores.iter().any(|v| v.len() != n) {
        return None;
    }

    #[allow(clippy::cast_precision_loss)]
    let pop_var = |v: &[f64]| -> f64 {
        let mean: f64 = v.iter().sum::<f64>() / v.len() as f64;
        v.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64
    };

    let sum_item_var: f64 = item_scores.iter().map(|v| pop_var(v)).sum();

    // Compute totals per respondent (sum across items for each person)
    let totals: Vec<f64> = (0..n)
        .map(|j| item_scores.iter().map(|item| item[j]).sum::<f64>())
        .collect();
    let var_total = pop_var(&totals);

    if var_total == 0.0 {
        return None;
    }

    #[allow(clippy::cast_precision_loss)]
    Some((k as f64 / (k as f64 - 1.0)) * (1.0 - sum_item_var / var_total))
}

/// Z-score normalise a slice of scores: `(x - mean) / std`.
///
/// Returns a vector of zeros if standard deviation is zero.
#[must_use]
pub fn z_score_normalize(scores: &[f64]) -> Vec<f64> {
    if scores.is_empty() {
        return Vec::new();
    }
    #[allow(clippy::cast_precision_loss)]
    let n = scores.len() as f64;
    let mean: f64 = scores.iter().sum::<f64>() / n;
    let std: f64 = (scores.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n).sqrt();
    if std == 0.0 {
        return vec![0.0; scores.len()];
    }
    scores.iter().map(|&x| (x - mean) / std).collect()
}

/// Percentile rank of `score` within `population` (0.0–100.0).
///
/// Formula: (number of scores strictly below `score` / N) * 100.
#[must_use]
pub fn percentile_rank_score(score: f64, population: &[f64]) -> f64 {
    if population.is_empty() {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let below = population.iter().filter(|&&x| x < score).count() as f64;
    (below / population.len() as f64) * 100.0
}

// ═══════════════════════════════════════
// Behavioral Patterns
// ═══════════════════════════════════════

/// Length of the current (trailing) streak of `true` values at the end of `events`.
#[must_use]
pub fn streak_length(events: &[bool]) -> usize {
    events.iter().rev().take_while(|&&e| e).count()
}

/// Longest unbroken run of `true` values anywhere in `events`.
#[must_use]
pub fn longest_streak(events: &[bool]) -> usize {
    let mut longest = 0usize;
    let mut current = 0usize;
    for &e in events {
        if e {
            current += 1;
            if current > longest {
                longest = current;
            }
        } else {
            current = 0;
        }
    }
    longest
}

/// Habit completion rate: `completed / total * 100.0`.
///
/// Returns `0.0` when `total` is zero.
#[must_use]
pub fn habit_score(completed: u32, total: u32) -> f64 {
    if total == 0 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let score = completed as f64 / total as f64 * 100.0;
    score
}

/// Consistency score: `1.0 - (std / mean)`, clamped to `[0.0, 1.0]`.
///
/// Measures how stable daily performance is relative to average.
/// Returns `0.0` if mean is zero or the slice is empty.
#[must_use]
pub fn consistency_score(daily_scores: &[f64]) -> f64 {
    if daily_scores.is_empty() {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let n = daily_scores.len() as f64;
    let mean: f64 = daily_scores.iter().sum::<f64>() / n;
    if mean == 0.0 {
        return 0.0;
    }
    let std: f64 = (daily_scores
        .iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>()
        / n)
        .sqrt();
    (1.0 - std / mean).clamp(0.0, 1.0)
}

/// Regression-to-the-mean prediction.
///
/// `predicted = mean + reliability * (score - mean)`
///
/// `reliability` is the test-retest reliability coefficient (0.0–1.0).
#[must_use]
pub fn regression_to_mean_prediction(score: f64, mean: f64, reliability: f64) -> f64 {
    mean + reliability * (score - mean)
}

// ═══════════════════════════════════════
// Cognitive Load / Attention
// ═══════════════════════════════════════

/// Cognitive load index combining element interactivity.
///
/// `index = items + sqrt(relations)` — after Sweller's intrinsic load model.
#[must_use]
pub fn cognitive_load_index(items: usize, relations: usize) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let index = items as f64 + (relations as f64).sqrt();
    index
}

/// Miller's Law check: can a working memory hold `chunk_count` items?
///
/// Returns `true` iff `chunk_count <= 9` (7 ± 2 capacity).
#[must_use]
pub fn working_memory_fit(chunk_count: usize) -> bool {
    chunk_count <= 9
}

/// Exponential attention decay model.
///
/// `A(t) = initial * 0.5^(elapsed / half_life)`
///
/// Returns `initial` unchanged if `half_life_minutes <= 0.0`.
#[must_use]
pub fn attention_decay(initial: f64, elapsed_minutes: f64, half_life_minutes: f64) -> f64 {
    if half_life_minutes <= 0.0 {
        return initial;
    }
    initial * (0.5f64).powf(elapsed_minutes / half_life_minutes)
}

/// Estimated reading time in minutes.
///
/// `minutes = text_words / wpm`
///
/// Returns `0.0` if `wpm <= 0.0`.
#[must_use]
pub fn reading_span_estimate(wpm: f64, text_words: usize) -> f64 {
    if wpm <= 0.0 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let minutes = text_words as f64 / wpm;
    minutes
}

// ═══════════════════════════════════════
// Personality / Archetype Scoring
// ═══════════════════════════════════════

/// Big Five (OCEAN) trait scores from a response slice.
///
/// Items are assumed interleaved in OCEAN order (item 0 → O, item 1 → C, …, item 4 → A,
/// item 5 → N, item 6 → O, …).  Each response is in `[-2, 2]`.
/// Returns scores scaled to `[0.0, 100.0]` where 50 = neutral.
///
/// Returns all-zero array if `responses` is empty.
#[must_use]
pub fn big_five_score(responses: &[i8]) -> [f64; 5] {
    const TRAITS: usize = 5;
    let mut sums = [0.0f64; TRAITS];
    let mut counts = [0usize; TRAITS];

    for (idx, &r) in responses.iter().enumerate() {
        let trait_idx = idx % TRAITS;
        sums[trait_idx] += r as f64;
        counts[trait_idx] += 1;
    }

    let mut out = [0.0f64; TRAITS];
    for i in 0..TRAITS {
        if counts[i] == 0 {
            continue;
        }
        // Mean in [-2, 2] → scale to [0, 100]: (mean + 2) / 4 * 100
        let mean = sums[i] / counts[i] as f64;
        out[i] = ((mean + 2.0) / 4.0 * 100.0).clamp(0.0, 100.0);
    }
    out
}

/// Index of the dominant (maximum) trait score. Returns `None` if `scores` is empty.
#[must_use]
pub fn dominant_trait(scores: &[f64]) -> Option<usize> {
    if scores.is_empty() {
        return None;
    }
    scores
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
}

/// Trait balance score: `1.0 - (std / max_std)`, clamped to `[0.0, 1.0]`.
///
/// Normalises the population standard deviation by the theoretical maximum
/// for `n` non-negative scores whose largest value is `max`:
/// `max_std = max × √(n−1) / n`.  This maps the full range correctly:
/// all-equal → 1.0, one-hot `[M, 0, …, 0]` → 0.0.
///
/// Returns `0.0` if `scores` is empty or max is zero.
#[must_use]
pub fn trait_balance_score(scores: &[f64]) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    let max = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if max == 0.0 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let n = scores.len() as f64;
    if n < 2.0 {
        return 1.0;
    }
    let mean: f64 = scores.iter().sum::<f64>() / n;
    let std: f64 = (scores.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n).sqrt();
    let max_std = max * (n - 1.0).sqrt() / n;
    if max_std == 0.0 {
        return 1.0;
    }
    (1.0 - std / max_std).clamp(0.0, 1.0)
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Psychometric scoring ───────────────────────────────────────────────

    #[test]
    fn test_likert_mean_valid() {
        let r = likert_mean(&[1, 2, 3, 4, 5], 5).unwrap();
        assert!((r - 3.0).abs() < 1e-10, "got {r}");
    }

    #[test]
    fn test_likert_mean_invalid() {
        // 6 is out of range for a 5-point scale
        assert!(likert_mean(&[1, 2, 6], 5).is_none());
        // empty slice
        assert!(likert_mean(&[], 5).is_none());
    }

    #[test]
    fn test_likert_std() {
        // Responses [1,3,5] on a 5-point scale: mean=3, deviations [-2,0,2], var=8/3
        let std = likert_std(&[1, 3, 5], 5).unwrap();
        let expected = (8.0f64 / 3.0).sqrt();
        assert!((std - expected).abs() < 1e-10, "got {std}");
    }

    #[test]
    fn test_cronbach_alpha_perfect() {
        // Identical items → raw alpha should be 1.0.
        // (Perfectly *correlated* but differently-scaled items give α < 1 under
        // raw alpha; that is the correct behaviour of the unstandardised formula.)
        let items = vec![
            vec![1.0, 2.0, 3.0, 4.0],
            vec![1.0, 2.0, 3.0, 4.0],
            vec![1.0, 2.0, 3.0, 4.0],
        ];
        let alpha = cronbach_alpha(&items).unwrap();
        assert!((alpha - 1.0).abs() < 1e-6, "alpha={alpha}");
    }

    #[test]
    fn test_cronbach_alpha_too_few_items() {
        let items = vec![vec![1.0, 2.0, 3.0]];
        assert!(cronbach_alpha(&items).is_none());
    }

    #[test]
    fn test_z_score_normalize() {
        let zs = z_score_normalize(&[2.0, 4.0, 6.0]);
        // mean=4, std=sqrt(8/3); z for 2 should be negative, for 6 positive
        assert!(zs[0] < 0.0 && zs[2] > 0.0, "zs={zs:?}");
        // Check that normalised scores have mean≈0
        let mean: f64 = zs.iter().sum::<f64>() / zs.len() as f64;
        assert!(mean.abs() < 1e-10, "mean={mean}");
    }

    #[test]
    fn test_z_score_normalize_constant() {
        // Constant input → all zeros
        let zs = z_score_normalize(&[5.0, 5.0, 5.0]);
        assert_eq!(zs, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_percentile_rank_score() {
        let pop = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        // 3 values below 40.0 → 60th percentile
        let p = percentile_rank_score(40.0, &pop);
        assert!((p - 60.0).abs() < 1e-10, "got {p}");
        // No values below minimum
        let p2 = percentile_rank_score(10.0, &pop);
        assert!((p2 - 0.0).abs() < 1e-10, "got {p2}");
    }

    // ── Behavioral patterns ────────────────────────────────────────────────

    #[test]
    fn test_streak_length() {
        let events = [false, true, true, false, true, true, true];
        assert_eq!(streak_length(&events), 3);
        assert_eq!(streak_length(&[false, false]), 0);
        assert_eq!(streak_length(&[]), 0);
    }

    #[test]
    fn test_longest_streak() {
        let events = [true, false, true, true, true, false, true];
        assert_eq!(longest_streak(&events), 3);
        assert_eq!(longest_streak(&[false; 5]), 0);
        assert_eq!(longest_streak(&[true; 4]), 4);
    }

    #[test]
    fn test_habit_score() {
        assert!((habit_score(7, 10) - 70.0).abs() < 1e-10);
        assert_eq!(habit_score(0, 0), 0.0);
        assert!((habit_score(10, 10) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_consistency_score() {
        // Identical daily scores → perfect consistency
        let c = consistency_score(&[5.0, 5.0, 5.0, 5.0]);
        assert!((c - 1.0).abs() < 1e-10, "got {c}");
        // High variation relative to mean
        let c2 = consistency_score(&[1.0, 9.0]);
        assert!(c2 < 0.5, "expected low consistency, got {c2}");
    }

    #[test]
    fn test_regression_to_mean_prediction() {
        // Perfect reliability: prediction = score
        assert!((regression_to_mean_prediction(120.0, 100.0, 1.0) - 120.0).abs() < 1e-10);
        // Zero reliability: prediction = mean
        assert!((regression_to_mean_prediction(120.0, 100.0, 0.0) - 100.0).abs() < 1e-10);
        // Partial reliability
        let pred = regression_to_mean_prediction(120.0, 100.0, 0.8);
        assert!((pred - 116.0).abs() < 1e-10, "got {pred}");
    }

    // ── Cognitive load / attention ─────────────────────────────────────────

    #[test]
    fn test_cognitive_load_index() {
        // 5 items, 16 relations → 5 + 4 = 9.0
        let cli = cognitive_load_index(5, 16);
        assert!((cli - 9.0).abs() < 1e-10, "got {cli}");
    }

    #[test]
    fn test_working_memory_fit() {
        assert!(working_memory_fit(7));
        assert!(working_memory_fit(9));
        assert!(!working_memory_fit(10));
    }

    #[test]
    fn test_attention_decay() {
        // After one half-life, intensity should be 50 %
        let a = attention_decay(1.0, 30.0, 30.0);
        assert!((a - 0.5).abs() < 1e-10, "got {a}");
        // After two half-lives → 25 %
        let a2 = attention_decay(1.0, 60.0, 30.0);
        assert!((a2 - 0.25).abs() < 1e-10, "got {a2}");
    }

    #[test]
    fn test_reading_span_estimate() {
        // 300 wpm, 600 words → 2.0 minutes
        let t = reading_span_estimate(300.0, 600);
        assert!((t - 2.0).abs() < 1e-10, "got {t}");
        assert_eq!(reading_span_estimate(0.0, 600), 0.0);
    }

    // ── Personality / archetype scoring ───────────────────────────────────

    #[test]
    fn test_big_five_score_neutral() {
        // All zeros → each trait maps to 50.0
        let scores = big_five_score(&[0i8; 10]);
        for &s in &scores {
            assert!((s - 50.0).abs() < 1e-10, "got {s}");
        }
    }

    #[test]
    fn test_big_five_score_max() {
        // All +2 → each trait maps to 100.0
        let scores = big_five_score(&[2i8; 15]);
        for &s in &scores {
            assert!((s - 100.0).abs() < 1e-10, "got {s}");
        }
    }

    #[test]
    fn test_dominant_trait() {
        assert_eq!(dominant_trait(&[30.0, 70.0, 50.0, 40.0, 60.0]), Some(1));
        assert_eq!(dominant_trait(&[]), None);
    }

    #[test]
    fn test_trait_balance_score() {
        // Perfectly equal scores → balance = 1.0
        let b = trait_balance_score(&[50.0; 5]);
        assert!((b - 1.0).abs() < 1e-10, "got {b}");
        // One very dominant trait → lower balance
        let b2 = trait_balance_score(&[100.0, 10.0, 10.0, 10.0, 10.0]);
        assert!(b2 < 0.5, "expected low balance, got {b2}");
    }
}
