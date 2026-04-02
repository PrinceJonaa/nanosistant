//! Health metrics — pure math only. No medical advice.

// ═══════════════════════════════════════
// Body Composition
// ═══════════════════════════════════════

/// Body Mass Index: kg / m².
#[must_use]
pub fn bmi(weight_kg: f64, height_m: f64) -> f64 {
    if height_m <= 0.0 { return 0.0; }
    weight_kg / (height_m * height_m)
}

/// WHO BMI category.
#[must_use]
pub fn bmi_category(bmi: f64) -> &'static str {
    if bmi < 0.0  { "invalid" }
    else if bmi < 18.5 { "Underweight" }
    else if bmi < 25.0 { "Normal weight" }
    else if bmi < 30.0 { "Overweight" }
    else               { "Obese" }
}

/// Ideal body weight via Devine formula (kg).
/// Male:   50 + 2.3 * (height_cm/2.54 - 60)
/// Female: 45.5 + 2.3 * (height_cm/2.54 - 60)
#[must_use]
pub fn ideal_weight_devine(height_cm: f64, male: bool) -> f64 {
    let inches = height_cm / 2.54;
    let base = if male { 50.0 } else { 45.5 };
    let excess = (inches - 60.0).max(0.0);
    base + 2.3 * excess
}

// ═══════════════════════════════════════
// Energy Expenditure
// ═══════════════════════════════════════

/// Basal Metabolic Rate via Mifflin–St Jeor (kcal/day).
/// Male:   10*w + 6.25*h - 5*age + 5
/// Female: 10*w + 6.25*h - 5*age - 161
#[must_use]
pub fn bmr_mifflin(weight_kg: f64, height_cm: f64, age: u32, male: bool) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let base = 10.0 * weight_kg + 6.25 * height_cm - 5.0 * age as f64;
    if male { base + 5.0 } else { base - 161.0 }
}

/// Total Daily Energy Expenditure = BMR * activity multiplier.
#[must_use]
pub fn tdee(bmr: f64, activity_level: &str) -> f64 {
    let multiplier = match activity_level.to_lowercase().as_str() {
        "sedentary"                            => 1.2,
        "light" | "lightly_active"             => 1.375,
        "moderate" | "moderately_active"       => 1.55,
        "active"                               => 1.725,
        "very_active" | "very active"          => 1.9,
        _                                      => 1.2,
    };
    bmr * multiplier
}

// ═══════════════════════════════════════
// Cardiovascular
// ═══════════════════════════════════════

/// Estimated maximum heart rate: 220 - age.
#[must_use]
pub fn max_heart_rate(age: u32) -> u32 {
    220u32.saturating_sub(age)
}

/// Target heart rate zone: (220 - age) * intensity_fraction.
/// `intensity_pct` is in [0.0, 1.0].
#[must_use]
pub fn target_heart_rate_zone(age: u32, intensity_pct: f64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let max_hr = max_heart_rate(age) as f64;
    max_hr * intensity_pct
}

/// VO₂ max estimate (Uth–Sørensen–Overgaard–Pedersen formula).
/// VO₂max ≈ 15 * (HRmax / HRrest).
#[must_use]
pub fn vo2_max_estimate(resting_hr: u32, max_hr: u32) -> f64 {
    if resting_hr == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let v = 15.0 * (max_hr as f64 / resting_hr as f64);
    v
}

// ═══════════════════════════════════════
// Hydration & Recovery
// ═══════════════════════════════════════

/// Daily water intake estimate (ml).
/// Base: 35 ml/kg + 12 ml per minute of activity.
#[must_use]
pub fn water_intake_ml(weight_kg: f64, activity_minutes: u32) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let base = 35.0 * weight_kg + 12.0 * activity_minutes as f64;
    base
}

/// Macronutrient grams from total calories and percentage splits.
/// Returns (protein_g, carb_g, fat_g).
/// Protein & carbs: 4 kcal/g; fat: 9 kcal/g.
#[must_use]
pub fn macros_from_calories(
    calories: f64,
    protein_pct: f64,
    carb_pct: f64,
    fat_pct: f64,
) -> (f64, f64, f64) {
    let protein_g = calories * protein_pct / 4.0;
    let carb_g    = calories * carb_pct    / 4.0;
    let fat_g     = calories * fat_pct     / 9.0;
    (protein_g, carb_g, fat_g)
}

/// Approximate number of 90-minute sleep cycles in `hours`.
#[must_use]
pub fn sleep_cycles(hours: f64) -> f64 { hours / 1.5 }

/// Composite recovery score in [0, 100].
/// Weighted: sleep 50%, resting HR 30%, HRV 20%.
/// Assumes:
///   - Optimal sleep = 8h, min = 4h
///   - Optimal resting HR ≤ 50 bpm, worst = 100 bpm
///   - Optimal HRV ≥ 80 ms, worst = 0 ms
#[must_use]
pub fn recovery_score(sleep_hrs: f64, resting_hr: u32, hrv: f64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let sleep_score = ((sleep_hrs - 4.0) / 4.0).clamp(0.0, 1.0) * 100.0;
    let hr_score = {
        let hr = resting_hr as f64;
        (1.0 - ((hr - 50.0) / 50.0).clamp(0.0, 1.0)) * 100.0
    };
    let hrv_score = (hrv / 80.0).clamp(0.0, 1.0) * 100.0;
    0.5 * sleep_score + 0.3 * hr_score + 0.2 * hrv_score
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_bmi() {
        // 70 kg, 1.75 m → 22.86
        let b = bmi(70.0, 1.75);
        assert!((b - 22.857).abs() < 0.01, "got {b}");
        assert_eq!(bmi(70.0, 0.0), 0.0);
    }

    #[test] fn test_bmi_category() {
        assert_eq!(bmi_category(17.0), "Underweight");
        assert_eq!(bmi_category(22.0), "Normal weight");
        assert_eq!(bmi_category(27.0), "Overweight");
        assert_eq!(bmi_category(35.0), "Obese");
    }

    #[test] fn test_ideal_weight_devine() {
        // Male, 178 cm: 50 + 2.3 * (70.08 - 60) = 50 + 2.3*10.08 = 73.18
        let iw = ideal_weight_devine(178.0, true);
        assert!(iw > 70.0 && iw < 76.0, "got {iw}");
    }

    #[test] fn test_bmr_mifflin_male() {
        // 70kg, 175cm, 30yo male: 10*70 + 6.25*175 - 5*30 + 5 = 700 + 1093.75 - 150 + 5 = 1648.75
        let bmr = bmr_mifflin(70.0, 175.0, 30, true);
        assert!((bmr - 1648.75).abs() < 0.1, "got {bmr}");
    }

    #[test] fn test_bmr_mifflin_female() {
        let bmr = bmr_mifflin(60.0, 165.0, 25, false);
        // 10*60 + 6.25*165 - 5*25 - 161 = 600 + 1031.25 - 125 - 161 = 1345.25
        assert!((bmr - 1345.25).abs() < 0.1, "got {bmr}");
    }

    #[test] fn test_tdee() {
        let bmr = 1600.0;
        let t_sed = tdee(bmr, "sedentary");
        assert!((t_sed - 1920.0).abs() < 0.1);
        let t_active = tdee(bmr, "active");
        assert!((t_active - 2760.0).abs() < 0.1);
    }

    #[test] fn test_max_heart_rate() {
        assert_eq!(max_heart_rate(20), 200);
        assert_eq!(max_heart_rate(50), 170);
        assert_eq!(max_heart_rate(220), 0); // saturating_sub
    }

    #[test] fn test_target_heart_rate() {
        // Age 30, 70% intensity: (220-30)*0.7 = 133
        let thr = target_heart_rate_zone(30, 0.7);
        assert!((thr - 133.0).abs() < 0.1, "got {thr}");
    }

    #[test] fn test_vo2_max() {
        // 15 * (200/60) = 50
        let vo2 = vo2_max_estimate(60, 200);
        assert!((vo2 - 50.0).abs() < 0.1, "got {vo2}");
    }

    #[test] fn test_water_intake() {
        // 70kg, 30 min: 35*70 + 12*30 = 2450 + 360 = 2810 ml
        let w = water_intake_ml(70.0, 30);
        assert!((w - 2810.0).abs() < 0.1, "got {w}");
    }

    #[test] fn test_macros() {
        // 2000 kcal, 30% protein, 40% carb, 30% fat
        let (p, c, f) = macros_from_calories(2000.0, 0.30, 0.40, 0.30);
        assert!((p - 150.0).abs() < 0.1, "protein={p}");
        assert!((c - 200.0).abs() < 0.1, "carb={c}");
        assert!((f - 66.67).abs() < 0.1, "fat={f}");
    }

    #[test] fn test_sleep_cycles() {
        assert!((sleep_cycles(7.5) - 5.0).abs() < 1e-10);
        assert!((sleep_cycles(6.0) - 4.0).abs() < 1e-10);
    }

    #[test] fn test_recovery_score() {
        // Perfect: 8h sleep, 50 bpm resting HR, 80ms HRV → 100
        let score = recovery_score(8.0, 50, 80.0);
        assert!((score - 100.0).abs() < 0.1, "got {score}");
        // Very poor: 4h sleep, 100 bpm, 0 HRV → 0
        let score2 = recovery_score(4.0, 100, 0.0);
        assert!((score2 - 0.0).abs() < 0.1, "got {score2}");
    }
}
