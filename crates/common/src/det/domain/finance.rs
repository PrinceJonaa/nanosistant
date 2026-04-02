//! Finance domain deterministic functions — expanded module.
//!
//! Zero-token financial math: options pricing, risk metrics, moving averages,
//! position sizing, Kelly criterion, portfolio math. Pure functions.

use serde::Serialize;

// ═══════════════════════════════════════
// Basic Returns & Growth
// ═══════════════════════════════════════

#[must_use]
pub fn percentage_change(from: f64, to: f64) -> f64 {
    if from == 0.0 { return 0.0; }
    ((to - from) / from * 100.0 * 100.0).round() / 100.0
}

#[must_use]
pub fn compound_annual_growth(start: f64, end: f64, years: f64) -> f64 {
    if start <= 0.0 || years <= 0.0 { return 0.0; }
    ((end / start).powf(1.0 / years) - 1.0) * 100.0
}

/// Simple return: (end - start) / start.
#[must_use]
pub fn simple_return(start: f64, end: f64) -> f64 {
    if start == 0.0 { return 0.0; }
    (end - start) / start
}

/// Log return: ln(end/start).
#[must_use]
pub fn log_return(start: f64, end: f64) -> f64 {
    if start <= 0.0 || end <= 0.0 { return 0.0; }
    (end / start).ln()
}

// ═══════════════════════════════════════
// Position Sizing & Risk
// ═══════════════════════════════════════

/// Position size from capital, risk%, entry, stop.
#[must_use]
pub fn position_size(capital: f64, risk_pct: f64, entry: f64, stop: f64) -> f64 {
    let risk_per_unit = (entry - stop).abs();
    if risk_per_unit == 0.0 { return 0.0; }
    let risk_amount = capital * (risk_pct / 100.0);
    (risk_amount / risk_per_unit).floor()
}

/// Dollar value at risk.
#[must_use]
pub fn dollar_risk(capital: f64, risk_pct: f64) -> f64 {
    capital * (risk_pct / 100.0)
}

/// Risk/reward ratio.
#[must_use]
pub fn risk_reward(entry: f64, stop: f64, target: f64) -> f64 {
    let risk = (entry - stop).abs();
    let reward = (target - entry).abs();
    if risk == 0.0 { return 0.0; }
    (reward / risk * 100.0).round() / 100.0
}

/// Kelly criterion: optimal bet fraction.
/// win_rate in 0.0-1.0, win_loss_ratio = avg_win / avg_loss.
#[must_use]
pub fn kelly_fraction(win_rate: f64, win_loss_ratio: f64) -> f64 {
    if win_loss_ratio <= 0.0 { return 0.0; }
    let kelly = win_rate - (1.0 - win_rate) / win_loss_ratio;
    kelly.max(0.0)
}

/// Half-Kelly (more conservative position sizing).
#[must_use]
pub fn half_kelly(win_rate: f64, win_loss_ratio: f64) -> f64 {
    kelly_fraction(win_rate, win_loss_ratio) / 2.0
}

// ═══════════════════════════════════════
// Moving Averages
// ═══════════════════════════════════════

/// Simple Moving Average of a price series.
#[must_use]
pub fn sma(prices: &[f64], period: usize) -> Vec<f64> {
    if prices.len() < period || period == 0 { return vec![]; }
    let mut result = Vec::with_capacity(prices.len() - period + 1);
    for window in prices.windows(period) {
        let avg = window.iter().sum::<f64>() / period as f64;
        result.push((avg * 10000.0).round() / 10000.0);
    }
    result
}

/// Exponential Moving Average.
#[must_use]
pub fn ema(prices: &[f64], period: usize) -> Vec<f64> {
    if prices.is_empty() || period == 0 { return vec![]; }
    let k = 2.0 / (period as f64 + 1.0);
    let mut result = Vec::with_capacity(prices.len());
    let mut ema_val = prices[0];
    result.push((ema_val * 10000.0).round() / 10000.0);
    for &price in &prices[1..] {
        ema_val = price * k + ema_val * (1.0 - k);
        result.push((ema_val * 10000.0).round() / 10000.0);
    }
    result
}

/// Relative Strength Index (RSI).
#[must_use]
pub fn rsi(prices: &[f64], period: usize) -> Option<f64> {
    if prices.len() <= period || period == 0 { return None; }
    let gains: Vec<f64> = prices.windows(2).map(|w| (w[1] - w[0]).max(0.0)).collect();
    let losses: Vec<f64> = prices.windows(2).map(|w| (w[0] - w[1]).max(0.0)).collect();
    let avg_gain: f64 = gains[..period].iter().sum::<f64>() / period as f64;
    let avg_loss: f64 = losses[..period].iter().sum::<f64>() / period as f64;
    if avg_loss == 0.0 { return Some(100.0); }
    let rs = avg_gain / avg_loss;
    let rsi_val = 100.0 - 100.0 / (1.0 + rs);
    Some((rsi_val * 100.0).round() / 100.0)
}

/// MACD (returns macd line, signal line, histogram).
#[derive(Debug, Serialize)]
pub struct MacdResult {
    pub macd: f64,
    pub signal: f64,
    pub histogram: f64,
}

#[must_use]
pub fn macd(prices: &[f64], fast: usize, slow: usize, signal: usize) -> Option<MacdResult> {
    if prices.len() < slow { return None; }
    let fast_ema = ema(prices, fast);
    let slow_ema = ema(prices, slow);
    let min_len = fast_ema.len().min(slow_ema.len());
    if min_len == 0 { return None; }
    let macd_line: Vec<f64> = (0..min_len)
        .map(|i| fast_ema[fast_ema.len() - min_len + i] - slow_ema[slow_ema.len() - min_len + i])
        .collect();
    let signal_ema = ema(&macd_line, signal);
    if signal_ema.is_empty() { return None; }
    let macd_last = *macd_line.last()?;
    let signal_last = *signal_ema.last()?;
    Some(MacdResult {
        macd: (macd_last * 10000.0).round() / 10000.0,
        signal: (signal_last * 10000.0).round() / 10000.0,
        histogram: ((macd_last - signal_last) * 10000.0).round() / 10000.0,
    })
}

// ═══════════════════════════════════════
// Volatility & Risk Metrics
// ═══════════════════════════════════════

/// Annualized volatility from a returns series.
#[must_use]
pub fn annualized_volatility(returns: &[f64], trading_days: u32) -> f64 {
    if returns.len() < 2 { return 0.0; }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
        / (returns.len() - 1) as f64;
    let daily_vol = variance.sqrt();
    (daily_vol * (trading_days as f64).sqrt() * 10000.0).round() / 10000.0
}

/// Sharpe ratio: (return - risk_free) / volatility.
#[must_use]
pub fn sharpe_ratio(portfolio_return: f64, risk_free_rate: f64, volatility: f64) -> f64 {
    if volatility == 0.0 { return 0.0; }
    ((portfolio_return - risk_free_rate) / volatility * 1000.0).round() / 1000.0
}

/// Sortino ratio: like Sharpe but uses downside deviation.
#[must_use]
pub fn sortino_ratio(returns: &[f64], risk_free_rate: f64) -> f64 {
    if returns.is_empty() { return 0.0; }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let downside_sq: Vec<f64> = returns.iter()
        .filter(|&&r| r < risk_free_rate)
        .map(|&r| (r - risk_free_rate).powi(2))
        .collect();
    if downside_sq.is_empty() { return 0.0; }
    let downside_dev = (downside_sq.iter().sum::<f64>() / downside_sq.len() as f64).sqrt();
    if downside_dev == 0.0 { return 0.0; }
    ((mean - risk_free_rate) / downside_dev * 1000.0).round() / 1000.0
}

/// Maximum drawdown from a price series.
#[must_use]
pub fn max_drawdown(prices: &[f64]) -> f64 {
    if prices.len() < 2 { return 0.0; }
    let mut peak = prices[0];
    let mut max_dd = 0.0f64;
    for &price in &prices[1..] {
        if price > peak { peak = price; }
        let dd = (peak - price) / peak;
        if dd > max_dd { max_dd = dd; }
    }
    (max_dd * 10000.0).round() / 10000.0
}

/// Value at Risk (VaR) — parametric normal assumption.
/// confidence: e.g. 0.95 for 95% VaR.
#[must_use]
pub fn value_at_risk(portfolio_value: f64, daily_volatility: f64, confidence: f64) -> f64 {
    // z-scores for common confidence levels
    let z = if confidence >= 0.99      { 2.326 }
            else if confidence >= 0.975 { 1.960 }
            else if confidence >= 0.95  { 1.645 }
            else if confidence >= 0.90  { 1.282 }
            else                        { 1.0 };
    (portfolio_value * daily_volatility * z * 100.0).round() / 100.0
}

// ═══════════════════════════════════════
// Options (Black-Scholes approximation)
// ═══════════════════════════════════════

/// Black-Scholes call option price.
/// s=spot, k=strike, t=time_to_expiry_years, r=risk_free, sigma=volatility.
#[must_use]
pub fn black_scholes_call(s: f64, k: f64, t: f64, r: f64, sigma: f64) -> f64 {
    if t <= 0.0 { return (s - k).max(0.0); }
    let d1 = (s / k).ln() / (sigma * t.sqrt()) + (r + sigma * sigma / 2.0) * t / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();
    let call = s * normal_cdf(d1) - k * (-r * t).exp() * normal_cdf(d2);
    (call * 100.0).round() / 100.0
}

/// Black-Scholes put option price.
#[must_use]
pub fn black_scholes_put(s: f64, k: f64, t: f64, r: f64, sigma: f64) -> f64 {
    if t <= 0.0 { return (k - s).max(0.0); }
    let d1 = (s / k).ln() / (sigma * t.sqrt()) + (r + sigma * sigma / 2.0) * t / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();
    let put = k * (-r * t).exp() * normal_cdf(-d2) - s * normal_cdf(-d1);
    (put * 100.0).round() / 100.0
}

/// Approximate standard normal CDF using Hart's approximation.
fn normal_cdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let poly = t * (0.319_381_53 + t * (-0.356_563_782 + t * (1.781_477_937
        + t * (-1.821_255_978 + t * 1.330_274_429))));
    let pdf = (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();
    if x >= 0.0 { 1.0 - pdf * poly } else { pdf * poly }
}

/// Option delta.
#[must_use]
pub fn option_delta_call(s: f64, k: f64, t: f64, r: f64, sigma: f64) -> f64 {
    if t <= 0.0 { return if s > k { 1.0 } else { 0.0 }; }
    let d1 = (s / k).ln() / (sigma * t.sqrt()) + (r + sigma * sigma / 2.0) * t / (sigma * t.sqrt());
    (normal_cdf(d1) * 1000.0).round() / 1000.0
}

/// Implied volatility estimate (Newton-Raphson, 10 iterations).
#[must_use]
pub fn implied_volatility_call(market_price: f64, s: f64, k: f64, t: f64, r: f64) -> f64 {
    if t <= 0.0 { return 0.0; }
    let mut sigma = 0.2; // initial guess
    for _ in 0..20 {
        let price = black_scholes_call(s, k, t, r, sigma);
        let diff = market_price - price;
        if diff.abs() < 0.0001 { break; }
        // vega
        let d1 = (s / k).ln() / (sigma * t.sqrt()) + (r + sigma * sigma / 2.0) * t / (sigma * t.sqrt());
        let pdf = (-d1 * d1 / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();
        let vega = s * pdf * t.sqrt();
        if vega.abs() < 1e-10 { break; }
        sigma += diff / vega;
        if sigma <= 0.0 { sigma = 0.001; }
    }
    (sigma * 10000.0).round() / 10000.0
}

// ═══════════════════════════════════════
// Market Cap & Valuation
// ═══════════════════════════════════════

#[must_use]
pub fn market_cap(price: f64, shares_outstanding: f64) -> f64 {
    price * shares_outstanding
}

#[must_use]
pub fn pe_ratio(price: f64, eps: f64) -> f64 {
    if eps == 0.0 { return 0.0; }
    (price / eps * 100.0).round() / 100.0
}

#[must_use]
pub fn earnings_yield(eps: f64, price: f64) -> f64 {
    if price == 0.0 { return 0.0; }
    (eps / price * 100.0 * 100.0).round() / 100.0
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn kelly_works() {
        // Kelly = win_rate - (1 - win_rate) / win_loss_ratio
        // = 0.55 - 0.45/1.5 = 0.55 - 0.3 = 0.25
        let k = kelly_fraction(0.55, 1.5);
        assert!((k - 0.25).abs() < 0.001, "kelly: {k}");
    }
    #[test] fn sma_correct() {
        let prices = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&prices, 3);
        assert_eq!(result.len(), 3);
        assert!((result[0] - 2.0).abs() < 0.001);
    }
    #[test] fn rsi_overbought() {
        // All gains → RSI should be 100
        let prices: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let r = rsi(&prices, 14).unwrap();
        assert_eq!(r, 100.0);
    }
    #[test] fn max_drawdown_correct() {
        let prices = vec![100.0, 90.0, 80.0, 100.0, 95.0];
        let dd = max_drawdown(&prices);
        assert!((dd - 0.2).abs() < 0.001);
    }
    #[test] fn black_scholes_call_reasonable() {
        // ATM call, 1yr, 20% vol, 0% rate
        let price = black_scholes_call(100.0, 100.0, 1.0, 0.0, 0.2);
        assert!(price > 7.0 && price < 9.0, "price: {price}");
    }
    #[test] fn black_scholes_put_call_parity() {
        let s = 100.0; let k = 100.0; let t = 1.0; let r = 0.05; let sigma = 0.2;
        let call = black_scholes_call(s, k, t, r, sigma);
        let put = black_scholes_put(s, k, t, r, sigma);
        // Put-call parity: C - P = S - K*e^(-rT)
        let expected = s - k * (-r * t).exp();
        assert!((call - put - expected).abs() < 0.1, "parity: {call} - {put} = {}", call - put);
    }
    #[test] fn sharpe_works() {
        let s = sharpe_ratio(0.15, 0.05, 0.20);
        assert!((s - 0.5).abs() < 0.001);
    }
}
