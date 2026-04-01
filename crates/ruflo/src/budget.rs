//! Token budget manager with circuit breakers.
//!
//! Tracks token usage across a session and enforces configurable thresholds.
//! The status ladder is: green → amber → yellow → red → exhausted.

use chrono::{DateTime, Utc};
use nstn_common::proto::BudgetStatus;
use thiserror::Error;

// ─── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BudgetError {
    #[error("budget exhausted: {tokens_used}/{max_tokens} tokens used")]
    Exhausted { tokens_used: u32, max_tokens: u32 },
}

// ─── BudgetState ──────────────────────────────────────────────────────────────

/// Current budget health level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BudgetState {
    /// < 50 % used.
    Green,
    /// ≥ 50 % used.
    Amber,
    /// ≥ 75 % used.
    Yellow,
    /// ≥ 90 % used.
    Red,
    /// 100 % used.
    Exhausted,
}

impl BudgetState {
    /// Return the canonical status string used in proto messages.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Green => "green",
            Self::Amber => "amber",
            Self::Yellow => "yellow",
            Self::Red => "red",
            Self::Exhausted => "exhausted",
        }
    }
}

impl std::fmt::Display for BudgetState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── BudgetManager ────────────────────────────────────────────────────────────

/// Session-scoped token budget tracker.
#[derive(Debug, Clone)]
pub struct BudgetManager {
    max_tokens: u32,
    tokens_used: u32,
    session_start: DateTime<Utc>,
}

impl BudgetManager {
    /// Create a new `BudgetManager` with the given token cap.
    #[must_use]
    pub fn new(max_tokens: u32) -> Self {
        Self {
            max_tokens,
            tokens_used: 0,
            session_start: Utc::now(),
        }
    }

    /// Record additional token usage.
    pub fn record_usage(&mut self, tokens: u32) {
        self.tokens_used = self.tokens_used.saturating_add(tokens);
    }

    /// Check whether the budget is still available.
    ///
    /// # Errors
    /// Returns [`BudgetError::Exhausted`] when `tokens_used >= max_tokens`.
    pub fn check(&self) -> Result<(), BudgetError> {
        if self.tokens_used >= self.max_tokens {
            Err(BudgetError::Exhausted {
                tokens_used: self.tokens_used,
                max_tokens: self.max_tokens,
            })
        } else {
            Ok(())
        }
    }

    /// Remaining tokens (saturates at zero).
    #[must_use]
    pub fn remaining(&self) -> u32 {
        self.max_tokens.saturating_sub(self.tokens_used)
    }

    /// Fraction of budget consumed, in [0.0, 1.0].
    #[must_use]
    pub fn utilization_pct(&self) -> f64 {
        if self.max_tokens == 0 {
            return 1.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let pct = f64::from(self.tokens_used) / f64::from(self.max_tokens);
        pct.min(1.0)
    }

    /// Current [`BudgetState`].
    #[must_use]
    pub fn state(&self) -> BudgetState {
        let pct = self.utilization_pct();
        if pct >= 1.0 {
            BudgetState::Exhausted
        } else if pct >= 0.90 {
            BudgetState::Red
        } else if pct >= 0.75 {
            BudgetState::Yellow
        } else if pct >= 0.50 {
            BudgetState::Amber
        } else {
            BudgetState::Green
        }
    }

    /// Serialize the current state to the proto `BudgetStatus` type.
    #[must_use]
    pub fn status(&self) -> BudgetStatus {
        #[allow(clippy::cast_precision_loss)]
        let cost = f64::from(self.tokens_used) / 1_000_000.0 * 9.0;
        BudgetStatus {
            tokens_used: self.tokens_used,
            tokens_remaining: self.remaining(),
            estimated_cost_usd: (cost * 10_000.0).round() as f32 / 10_000.0,
            status: self.state().to_string(),
        }
    }

    /// Tokens used so far.
    #[must_use]
    pub fn tokens_used(&self) -> u32 {
        self.tokens_used
    }

    /// Maximum token cap.
    #[must_use]
    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    /// Session start timestamp.
    #[must_use]
    pub fn session_start(&self) -> DateTime<Utc> {
        self.session_start
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_budget_is_green_and_empty() {
        let bm = BudgetManager::new(10_000);
        assert_eq!(bm.tokens_used(), 0);
        assert_eq!(bm.remaining(), 10_000);
        assert_eq!(bm.state(), BudgetState::Green);
        assert!(bm.check().is_ok());
    }

    #[test]
    fn record_usage_accumulates() {
        let mut bm = BudgetManager::new(1_000);
        bm.record_usage(300);
        bm.record_usage(200);
        assert_eq!(bm.tokens_used(), 500);
        assert_eq!(bm.remaining(), 500);
    }

    #[test]
    fn state_transitions_correctly() {
        let mut bm = BudgetManager::new(1_000);

        bm.record_usage(499);
        assert_eq!(bm.state(), BudgetState::Green);

        bm.record_usage(1); // 500 / 1000 = 50%
        assert_eq!(bm.state(), BudgetState::Amber);

        bm.record_usage(250); // 750 / 1000 = 75%
        assert_eq!(bm.state(), BudgetState::Yellow);

        bm.record_usage(150); // 900 / 1000 = 90%
        assert_eq!(bm.state(), BudgetState::Red);

        bm.record_usage(100); // 1000 / 1000 = 100%
        assert_eq!(bm.state(), BudgetState::Exhausted);
    }

    #[test]
    fn check_returns_error_when_exhausted() {
        let mut bm = BudgetManager::new(100);
        bm.record_usage(100);
        assert!(matches!(bm.check(), Err(BudgetError::Exhausted { .. })));
    }

    #[test]
    fn check_returns_error_when_over_limit() {
        let mut bm = BudgetManager::new(100);
        bm.record_usage(150); // saturating_add still ≥ max
        let err = bm.check().unwrap_err();
        assert!(matches!(err, BudgetError::Exhausted { .. }));
    }

    #[test]
    fn remaining_saturates_at_zero() {
        let mut bm = BudgetManager::new(100);
        bm.record_usage(200);
        assert_eq!(bm.remaining(), 0);
    }

    #[test]
    fn utilization_pct_capped_at_one() {
        let mut bm = BudgetManager::new(100);
        bm.record_usage(200);
        assert!((bm.utilization_pct() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn status_proto_matches_state() {
        let mut bm = BudgetManager::new(1_000);
        bm.record_usage(600);

        let status = bm.status();
        assert_eq!(status.tokens_used, 600);
        assert_eq!(status.tokens_remaining, 400);
        assert_eq!(status.status, "amber");
    }

    #[test]
    fn zero_max_tokens_is_immediately_exhausted() {
        let bm = BudgetManager::new(0);
        assert_eq!(bm.state(), BudgetState::Exhausted);
        assert!(bm.check().is_err());
    }

    #[test]
    fn budget_state_ordering() {
        assert!(BudgetState::Green < BudgetState::Amber);
        assert!(BudgetState::Amber < BudgetState::Yellow);
        assert!(BudgetState::Yellow < BudgetState::Red);
        assert!(BudgetState::Red < BudgetState::Exhausted);
    }

    #[test]
    fn budget_state_display() {
        assert_eq!(BudgetState::Green.to_string(), "green");
        assert_eq!(BudgetState::Amber.to_string(), "amber");
        assert_eq!(BudgetState::Yellow.to_string(), "yellow");
        assert_eq!(BudgetState::Red.to_string(), "red");
        assert_eq!(BudgetState::Exhausted.to_string(), "exhausted");
    }
}
