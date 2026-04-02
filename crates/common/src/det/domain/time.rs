//! Time domain deterministic functions — datetime math, calendar ops.

use chrono::{NaiveDate, NaiveDateTime, Weekday, Datelike, Timelike, Duration};
use serde::Serialize;

/// Current UTC datetime as ISO 8601.
#[must_use]
pub fn current_datetime() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Days until a target date (YYYY-MM-DD). Negative = past.
pub fn days_until(target: &str) -> Result<i64, String> {
    let t = NaiveDate::parse_from_str(target, "%Y-%m-%d")
        .map_err(|e| format!("invalid date '{target}': {e}"))?;
    Ok((t - chrono::Utc::now().date_naive()).num_days())
}

/// Business days between two dates (Monday-Friday, no holidays).
pub fn business_days_between(from: &str, to: &str) -> Result<i64, String> {
    let f = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|e| format!("invalid from '{from}': {e}"))?;
    let t = NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .map_err(|e| format!("invalid to '{to}': {e}"))?;
    let (start, end, sign) = if f <= t { (f, t, 1i64) } else { (t, f, -1i64) };
    let mut count = 0i64;
    let mut current = start;
    while current < end {
        match current.weekday() {
            Weekday::Sat | Weekday::Sun => {}
            _ => count += 1,
        }
        current += Duration::days(1);
    }
    Ok(count * sign)
}

/// Next occurrence of a weekday from a starting date.
pub fn next_weekday(from: &str, weekday: &str) -> Result<String, String> {
    let start = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|e| format!("invalid date: {e}"))?;
    let target = parse_weekday(weekday)?;
    let mut date = start + Duration::days(1);
    while date.weekday() != target { date += Duration::days(1); }
    Ok(date.format("%Y-%m-%d").to_string())
}

fn parse_weekday(s: &str) -> Result<Weekday, String> {
    match s.to_lowercase().as_str() {
        "mon" | "monday"    => Ok(Weekday::Mon),
        "tue" | "tuesday"   => Ok(Weekday::Tue),
        "wed" | "wednesday" => Ok(Weekday::Wed),
        "thu" | "thursday"  => Ok(Weekday::Thu),
        "fri" | "friday"    => Ok(Weekday::Fri),
        "sat" | "saturday"  => Ok(Weekday::Sat),
        "sun" | "sunday"    => Ok(Weekday::Sun),
        _ => Err(format!("unknown weekday: {s}")),
    }
}

/// ISO week number of a date.
pub fn iso_week(date: &str) -> Result<u32, String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("invalid date: {e}"))?;
    Ok(d.iso_week().week())
}

/// Quarter of a date (1-4).
pub fn quarter(date: &str) -> Result<u32, String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("invalid date: {e}"))?;
    Ok((d.month() + 2) / 3)
}

/// Whether a year is a leap year.
#[must_use]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days in a given month.
#[must_use]
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1|3|5|7|8|10|12 => 31,
        4|6|9|11        => 30,
        2               => if is_leap_year(year) { 29 } else { 28 },
        _               => 0,
    }
}

/// Human-readable duration from seconds.
#[must_use]
pub fn duration_human(total_seconds: u64) -> String {
    let secs = total_seconds % 60;
    let mins = (total_seconds / 60) % 60;
    let hrs  = (total_seconds / 3600) % 24;
    let days = total_seconds / 86400;
    if days > 0        { format!("{days}d {hrs}h {mins}m {secs}s") }
    else if hrs > 0    { format!("{hrs}h {mins}m {secs}s") }
    else if mins > 0   { format!("{mins}m {secs}s") }
    else               { format!("{secs}s") }
}

/// Seconds from a duration string like "2h30m" or "90s".
pub fn parse_duration_secs(input: &str) -> Result<u64, String> {
    let s = input.trim().to_lowercase();
    let mut total = 0u64;
    let mut num = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
        } else {
            let n: u64 = num.parse().unwrap_or(0);
            num.clear();
            match ch {
                'd' => total += n * 86400,
                'h' => total += n * 3600,
                'm' => total += n * 60,
                's' => total += n,
                _ => {}
            }
        }
    }
    Ok(total)
}

/// Unix timestamp to ISO 8601.
#[must_use]
pub fn unix_to_iso(timestamp_secs: i64) -> String {
    use chrono::TimeZone;
    match chrono::Utc.timestamp_opt(timestamp_secs, 0).single() {
        Some(dt) => dt.to_rfc3339(),
        None => "invalid timestamp".to_string(),
    }
}

/// Age in years from a birth date.
pub fn age_years(birth_date: &str) -> Result<i64, String> {
    let birth = NaiveDate::parse_from_str(birth_date, "%Y-%m-%d")
        .map_err(|e| format!("invalid date: {e}"))?;
    let today = chrono::Utc::now().date_naive();
    let years = today.year() as i64 - birth.year() as i64
        - if (today.month(), today.day()) < (birth.month(), birth.day()) { 1 } else { 0 };
    Ok(years)
}

/// Add N business days to a date.
pub fn add_business_days(from: &str, days: i64) -> Result<String, String> {
    let mut date = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|e| format!("invalid date: {e}"))?;
    let mut remaining = days.abs();
    let step = if days >= 0 { 1 } else { -1 };
    while remaining > 0 {
        date += Duration::days(step);
        match date.weekday() {
            Weekday::Sat | Weekday::Sun => {}
            _ => remaining -= 1,
        }
    }
    Ok(date.format("%Y-%m-%d").to_string())
}

/// Time zone offset string (simplified — named zones).
#[must_use]
pub fn timezone_offset(tz_name: &str) -> &'static str {
    match tz_name.to_uppercase().as_str() {
        "UTC"   => "+00:00",
        "EST"   => "-05:00",
        "EDT"   => "-04:00",
        "CST"   => "-06:00",
        "CDT"   => "-05:00",
        "MST"   => "-07:00",
        "MDT"   => "-06:00",
        "PST"   => "-08:00",
        "PDT"   => "-07:00",
        "GMT"   => "+00:00",
        "BST"   => "+01:00",
        "CET"   => "+01:00",
        "CEST"  => "+02:00",
        "JST"   => "+09:00",
        "IST"   => "+05:30",
        "AEST"  => "+10:00",
        "AEDT"  => "+11:00",
        _       => "unknown",
    }
}

/// Month name from number (1-12).
#[must_use]
pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January", 2 => "February", 3 => "March",
        4 => "April", 5 => "May", 6 => "June",
        7 => "July", 8 => "August", 9 => "September",
        10 => "October", 11 => "November", 12 => "December",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn leap_year() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
    }
    #[test] fn days_in_feb() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
    }
    #[test] fn duration_human_works() {
        assert_eq!(duration_human(90), "1m 30s");
        assert_eq!(duration_human(3661), "1h 1m 1s");
        assert_eq!(duration_human(90_000), "1d 1h 0m 0s");
    }
    #[test] fn parse_duration_secs_works() {
        assert_eq!(parse_duration_secs("2h30m").unwrap(), 9000);
        assert_eq!(parse_duration_secs("1d").unwrap(), 86400);
    }
    #[test] fn business_days_between_works() {
        // Mon to Fri = 4 business days
        let days = business_days_between("2026-04-06", "2026-04-10").unwrap();
        assert_eq!(days, 4);
    }
    #[test] fn quarter_works() {
        assert_eq!(quarter("2026-01-15").unwrap(), 1);
        assert_eq!(quarter("2026-07-01").unwrap(), 3);
        assert_eq!(quarter("2026-12-31").unwrap(), 4);
    }
    #[test] fn iso_week_works() {
        let w = iso_week("2026-01-01").unwrap();
        assert!(w >= 1 && w <= 53);
    }
}
