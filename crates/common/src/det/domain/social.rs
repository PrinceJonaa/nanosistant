//! Social dynamics and network metrics — pure deterministic functions.

// ═══════════════════════════════════════
// Growth & Virality
// ═══════════════════════════════════════

/// Viral coefficient K = invites_per_user * conversion_rate.
/// K > 1 → exponential growth; K < 1 → linear growth.
#[must_use]
pub fn viral_coefficient(invites_per_user: f64, conversion_rate: f64) -> f64 {
    invites_per_user * conversion_rate
}

/// Net Promoter Score = (promoters - detractors) / total * 100.
#[must_use]
pub fn net_promoter_score(promoters: usize, detractors: usize, total: usize) -> f64 {
    if total == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let nps = (promoters as f64 - detractors as f64) / total as f64 * 100.0;
    nps
}

/// Engagement rate = interactions / reach.
#[must_use]
pub fn engagement_rate(interactions: usize, reach: usize) -> f64 {
    if reach == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let r = interactions as f64 / reach as f64;
    r
}

// ═══════════════════════════════════════
// Retention & Churn
// ═══════════════════════════════════════

/// Churn rate = lost / total_at_start.
#[must_use]
pub fn churn_rate(lost: usize, total_start: usize) -> f64 {
    if total_start == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let r = lost as f64 / total_start as f64;
    r
}

/// Retention rate = retained / total_at_start.
#[must_use]
pub fn retention_rate(retained: usize, total_start: usize) -> f64 {
    if total_start == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let r = retained as f64 / total_start as f64;
    r
}

/// Customer Lifetime Value = avg_revenue_per_period * avg_lifespan_periods.
#[must_use]
pub fn lifetime_value(avg_revenue: f64, avg_lifespan_periods: f64) -> f64 {
    avg_revenue * avg_lifespan_periods
}

/// Customer Acquisition Cost = marketing_spend / new_customers.
#[must_use]
pub fn acquisition_cost(marketing_spend: f64, new_customers: usize) -> f64 {
    if new_customers == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let cost = marketing_spend / new_customers as f64;
    cost
}

// ═══════════════════════════════════════
// SaaS / Business Metrics
// ═══════════════════════════════════════

/// SaaS magic number: new_ARR * 4 / S&M spend.
/// > 1.0 = efficient; < 0.5 = burning cash.
#[must_use]
pub fn magic_number(new_arr: f64, sales_marketing_spend: f64) -> f64 {
    if sales_marketing_spend == 0.0 { return 0.0; }
    (new_arr * 4.0) / sales_marketing_spend
}

/// Rule of 40: growth_rate_pct + profit_margin_pct.
/// Healthy SaaS company should exceed 40.
#[must_use]
pub fn rule_of_40(growth_rate_pct: f64, profit_margin_pct: f64) -> f64 {
    growth_rate_pct + profit_margin_pct
}

// ═══════════════════════════════════════
// Social Network Structure
// ═══════════════════════════════════════

/// Dunbar layer by relationship type.
#[must_use]
pub fn dunbar_social_layer(relationship_type: &str) -> u32 {
    match relationship_type.to_lowercase().as_str() {
        "intimate" | "inner_circle" | "support_clique" => 5,
        "sympathy_group" | "close_friends"             => 15,
        "affinity_group" | "friends"                   => 50,
        "dunbar" | "tribe" | "casual"                  => 150,
        "acquaintances" | "extended"                   => 500,
        _                                              => 0,
    }
}

/// Metcalfe's law: network value ∝ n*(n-1)/2 (number of unique pairs).
#[must_use]
pub fn network_effect_value(users: u64) -> f64 {
    if users == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let v = (users as f64) * (users - 1) as f64 / 2.0;
    v
}

/// Rogers' diffusion of innovations stage by adoption percentage.
#[must_use]
pub fn diffusion_of_innovations_stage(adoption_pct: f64) -> &'static str {
    if adoption_pct < 0.0    { "invalid" }
    else if adoption_pct < 2.5  { "Innovators" }
    else if adoption_pct < 16.0 { "Early Adopters" }
    else if adoption_pct < 50.0 { "Early Majority" }
    else if adoption_pct < 84.0 { "Late Majority" }
    else if adoption_pct <= 100.0 { "Laggards" }
    else                         { "invalid" }
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_viral_coefficient() {
        assert!((viral_coefficient(10.0, 0.1) - 1.0).abs() < 1e-10);
        assert!((viral_coefficient(5.0, 0.3) - 1.5).abs() < 1e-10);
    }

    #[test] fn test_nps() {
        // 70 promoters, 10 detractors, 100 total → (70-10)/100 * 100 = 60
        let nps = net_promoter_score(70, 10, 100);
        assert!((nps - 60.0).abs() < 1e-10, "got {nps}");
        assert_eq!(net_promoter_score(0, 0, 0), 0.0);
    }

    #[test] fn test_engagement_rate() {
        assert!((engagement_rate(500, 10_000) - 0.05).abs() < 1e-10);
        assert_eq!(engagement_rate(100, 0), 0.0);
    }

    #[test] fn test_churn_rate() {
        assert!((churn_rate(10, 100) - 0.1).abs() < 1e-10);
        assert_eq!(churn_rate(5, 0), 0.0);
    }

    #[test] fn test_retention_rate() {
        assert!((retention_rate(90, 100) - 0.9).abs() < 1e-10);
    }

    #[test] fn test_lifetime_value() {
        assert!((lifetime_value(50.0, 24.0) - 1200.0).abs() < 1e-10);
    }

    #[test] fn test_acquisition_cost() {
        assert!((acquisition_cost(10_000.0, 100) - 100.0).abs() < 1e-10);
        assert_eq!(acquisition_cost(5000.0, 0), 0.0);
    }

    #[test] fn test_magic_number() {
        // new_ARR=100k, S&M=200k → (100k*4)/200k = 2.0
        assert!((magic_number(100_000.0, 200_000.0) - 2.0).abs() < 1e-10);
        assert_eq!(magic_number(100.0, 0.0), 0.0);
    }

    #[test] fn test_rule_of_40() {
        assert!((rule_of_40(25.0, 20.0) - 45.0).abs() < 1e-10);
    }

    #[test] fn test_dunbar_social_layer() {
        assert_eq!(dunbar_social_layer("intimate"), 5);
        assert_eq!(dunbar_social_layer("close_friends"), 15);
        assert_eq!(dunbar_social_layer("friends"), 50);
        assert_eq!(dunbar_social_layer("tribe"), 150);
        assert_eq!(dunbar_social_layer("acquaintances"), 500);
        assert_eq!(dunbar_social_layer("unknown"), 0);
    }

    #[test] fn test_network_effect_value() {
        // n=4: 4*3/2 = 6
        assert!((network_effect_value(4) - 6.0).abs() < 1e-10);
        assert_eq!(network_effect_value(0), 0.0);
        assert!((network_effect_value(1) - 0.0).abs() < 1e-10);
    }

    #[test] fn test_diffusion_stage() {
        assert_eq!(diffusion_of_innovations_stage(1.0),  "Innovators");
        assert_eq!(diffusion_of_innovations_stage(10.0), "Early Adopters");
        assert_eq!(diffusion_of_innovations_stage(30.0), "Early Majority");
        assert_eq!(diffusion_of_innovations_stage(70.0), "Late Majority");
        assert_eq!(diffusion_of_innovations_stage(90.0), "Laggards");
        assert_eq!(diffusion_of_innovations_stage(-1.0), "invalid");
        assert_eq!(diffusion_of_innovations_stage(101.0), "invalid");
    }
}
