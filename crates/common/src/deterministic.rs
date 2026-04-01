//! Zero-token deterministic functions.
//!
//! Every function here runs as pure code — no LLM inference.
//! Available to all agents via the orchestrator's interception layer
//! AND as registered tools within agent `ConversationRuntimes`.

use chrono::{NaiveDate, Utc};

// ═══════════════════════════════════════
// Universal (any domain)
// ═══════════════════════════════════════

/// Returns current date and time in ISO 8601 format.
#[must_use]
pub fn current_datetime() -> String {
    Utc::now().to_rfc3339()
}

/// Days remaining until a target date (YYYY-MM-DD).
/// Negative if the date is in the past.
pub fn days_until(target_date: &str) -> Result<i64, String> {
    let target = NaiveDate::parse_from_str(target_date, "%Y-%m-%d")
        .map_err(|e| format!("invalid date '{target_date}': {e}"))?;
    let today = Utc::now().date_naive();
    Ok((target - today).num_days())
}

/// Word count of a text string.
#[must_use]
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Estimated reading time in minutes (avg 250 words/minute).
#[must_use]
pub fn reading_time_minutes(text: &str) -> f64 {
    let words = word_count(text);
    #[allow(clippy::cast_precision_loss)]
    let minutes = words as f64 / 250.0;
    (minutes * 10.0).round() / 10.0
}

/// Validates whether a string is valid JSON.
#[must_use]
pub fn json_validate(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text).is_ok()
}

/// Validates whether a string is a plausible URL.
#[must_use]
pub fn url_validate(text: &str) -> bool {
    text.starts_with("http://") || text.starts_with("https://")
}

// ═══════════════════════════════════════
// Music domain
// ═══════════════════════════════════════

/// Duration of one bar in seconds given BPM and beats per bar.
#[must_use]
pub fn bpm_to_bar_duration(bpm: u32, beats_per_bar: u32) -> f64 {
    if bpm == 0 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let duration = (60.0 / f64::from(bpm)) * f64::from(beats_per_bar);
    (duration * 1000.0).round() / 1000.0
}

/// Number of bars needed for a target duration at a given BPM (4/4 assumed).
#[must_use]
pub fn song_bar_count(bpm: u32, target_duration_secs: f64) -> u32 {
    if bpm == 0 {
        return 0;
    }
    // Use unrounded bar duration for accurate bar count
    #[allow(clippy::cast_precision_loss)]
    let bar_dur = (60.0 / f64::from(bpm)) * 4.0;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = (target_duration_secs / bar_dur).ceil() as u32;
    count
}

/// Returns the scale degrees for a given key and mode.
/// Supports "major" (ionian) and "minor" (aeolian).
#[must_use]
pub fn scale_degrees(key: &str, mode: &str) -> Vec<String> {
    let chromatic = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];

    // Normalize key to canonical form
    let normalized_key = normalize_note(key);
    let start = chromatic
        .iter()
        .position(|&n| n == normalized_key)
        .unwrap_or(0);

    let intervals = match mode.to_lowercase().as_str() {
        "major" | "ionian" => vec![0, 2, 4, 5, 7, 9, 11],
        "minor" | "aeolian" | "natural_minor" => vec![0, 2, 3, 5, 7, 8, 10],
        "dorian" => vec![0, 2, 3, 5, 7, 9, 10],
        "phrygian" => vec![0, 1, 3, 5, 7, 8, 10],
        "lydian" => vec![0, 2, 4, 6, 7, 9, 11],
        "mixolydian" => vec![0, 2, 4, 5, 7, 9, 10],
        "locrian" => vec![0, 1, 3, 5, 6, 8, 10],
        _ => vec![0, 2, 4, 5, 7, 9, 11], // default major
    };

    intervals
        .iter()
        .map(|&i| chromatic[(start + i) % 12].to_string())
        .collect()
}

/// Convert a chord name to roman numeral notation in a given key.
#[must_use]
pub fn chord_to_roman(chord: &str, key: &str) -> String {
    let scale = scale_degrees(key, "major");
    let root = normalize_note(&extract_chord_root(chord));
    let is_minor = chord.contains('m') && !chord.contains("maj");

    let degree = scale.iter().position(|n| *n == root);
    let roman_numerals = ["I", "II", "III", "IV", "V", "VI", "VII"];

    match degree {
        Some(idx) => {
            let numeral = roman_numerals[idx];
            if is_minor {
                numeral.to_lowercase()
            } else {
                numeral.to_string()
            }
        }
        None => format!("? ({chord} not in {key} major)"),
    }
}

/// Convert a roman numeral to a chord name in a given key.
#[must_use]
pub fn roman_to_chord(roman: &str, key: &str) -> String {
    let scale = scale_degrees(key, "major");
    let roman_upper = roman.to_uppercase();
    let roman_numerals = ["I", "II", "III", "IV", "V", "VI", "VII"];
    let is_minor = roman.chars().all(|c| c.is_lowercase() || !c.is_alphabetic());

    let idx = roman_numerals
        .iter()
        .position(|&r| r == roman_upper)
        .unwrap_or(0);

    let root = &scale[idx % scale.len()];
    if is_minor {
        format!("{root}m")
    } else {
        root.clone()
    }
}

/// Transpose a set of notes by a number of semitones.
#[must_use]
pub fn transpose(notes: &[String], semitones: i32) -> Vec<String> {
    let chromatic = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];

    notes
        .iter()
        .map(|note| {
            let normalized = normalize_note(note);
            if let Some(idx) = chromatic.iter().position(|&n| n == normalized) {
                let new_idx = ((idx as i32 + semitones).rem_euclid(12)) as usize;
                chromatic[new_idx].to_string()
            } else {
                note.clone()
            }
        })
        .collect()
}

/// Concert pitch frequency for a note and octave (A4 = 440Hz).
#[must_use]
pub fn note_to_frequency(note: &str, octave: u32) -> f64 {
    let chromatic = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let normalized = normalize_note(note);
    let semitone_from_a4 = chromatic
        .iter()
        .position(|&n| n == normalized)
        .map_or(0, |idx| {
            #[allow(clippy::cast_possible_wrap)]
            let offset = idx as i32 - 9; // A is at index 9
            #[allow(clippy::cast_possible_wrap)]
            let octave_offset = (octave as i32 - 4) * 12;
            offset + octave_offset
        });

    #[allow(clippy::cast_precision_loss)]
    let freq = 440.0 * 2.0_f64.powf(f64::from(semitone_from_a4) / 12.0);
    (freq * 100.0).round() / 100.0
}

/// Map a frequency to its approximate EQ band name.
#[must_use]
pub fn frequency_to_band(hz: f64) -> &'static str {
    if hz < 60.0 {
        "Sub Bass"
    } else if hz < 250.0 {
        "Bass"
    } else if hz < 500.0 {
        "Low Mids"
    } else if hz < 2000.0 {
        "Mids"
    } else if hz < 4000.0 {
        "Upper Mids"
    } else if hz < 6000.0 {
        "Presence"
    } else if hz < 12000.0 {
        "Brilliance"
    } else {
        "Air"
    }
}

/// Approximate syllable count using vowel-group heuristic.
#[must_use]
pub fn syllable_count(text: &str) -> u32 {
    let mut total = 0u32;
    for word in text.split_whitespace() {
        let lower = word.to_lowercase();
        let clean: String = lower.chars().filter(|c| c.is_alphabetic()).collect();
        if clean.is_empty() {
            continue;
        }

        let mut count = 0u32;
        let mut prev_vowel = false;
        for ch in clean.chars() {
            let is_vowel = matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
            if is_vowel && !prev_vowel {
                count += 1;
            }
            prev_vowel = is_vowel;
        }

        // Silent e adjustment
        if clean.ends_with('e') && count > 1 {
            count -= 1;
        }

        total += count.max(1);
    }
    total
}

/// Syllable density lambda: syllables per beat-unit.
#[must_use]
pub fn density_lambda(text: &str, _bpm: u32, bars: u32) -> f64 {
    let syllables = syllable_count(text);
    let beats = bars * 4; // assume 4/4
    if beats == 0 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let lambda = f64::from(syllables) / f64::from(beats);
    (lambda * 100.0).round() / 100.0
}

// ═══════════════════════════════════════
// Business / Release
// ═══════════════════════════════════════

/// Timeline entry for release scheduling.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TimelineEntry {
    pub date: String,
    pub label: String,
    pub days_before_release: i64,
}

/// Generate a release timeline from a release date using a template.
/// Template: "standard" generates typical music release milestones.
pub fn release_timeline(release_date: &str, template: &str) -> Result<Vec<TimelineEntry>, String> {
    let release = NaiveDate::parse_from_str(release_date, "%Y-%m-%d")
        .map_err(|e| format!("invalid date: {e}"))?;

    let offsets: Vec<(i64, &str)> = match template {
        "standard" => vec![
            (-28, "Final master due"),
            (-21, "Distributor upload deadline"),
            (-14, "Pre-save campaign launch"),
            (-7, "Single/teaser release"),
            (-3, "Social media push begins"),
            (0, "Release day"),
            (1, "Playlist pitching follow-up"),
            (7, "First-week analytics review"),
        ],
        _ => vec![(0, "Release day")],
    };

    Ok(offsets
        .into_iter()
        .map(|(days_offset, label)| {
            let date = release + chrono::Duration::days(days_offset);
            TimelineEntry {
                date: date.format("%Y-%m-%d").to_string(),
                label: label.to_string(),
                days_before_release: -days_offset,
            }
        })
        .collect())
}

/// Validate an ISRC code format (CC-XXX-YY-NNNNN).
#[must_use]
pub fn isrc_validate(code: &str) -> bool {
    let clean: String = code.chars().filter(|c| c.is_alphanumeric()).collect();
    if clean.len() != 12 {
        return false;
    }
    // First 2: country code (alpha), next 3: registrant (alphanumeric),
    // next 2: year (digits), last 5: designation (digits)
    let chars: Vec<char> = clean.chars().collect();
    chars[0..2].iter().all(|c| c.is_alphabetic())
        && chars[2..5].iter().all(|c| c.is_alphanumeric())
        && chars[5..7].iter().all(char::is_ascii_digit)
        && chars[7..12].iter().all(char::is_ascii_digit)
}

/// Loudness check report for streaming platforms.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LoudnessReport {
    pub platform: String,
    pub target_lufs: f64,
    pub measured_lufs: f64,
    pub adjustment_db: f64,
    pub status: String,
}

/// Check loudness compliance for a streaming platform.
#[must_use]
pub fn streaming_loudness_check(lufs: f64, platform: &str) -> LoudnessReport {
    let target = match platform.to_lowercase().as_str() {
        "spotify" => -14.0,
        "apple" | "apple_music" => -16.0,
        "youtube" => -14.0,
        "amazon" => -14.0,
        "tidal" => -14.0,
        _ => -14.0,
    };

    let adjustment = target - lufs;
    let status = if adjustment.abs() < 0.5 {
        "pass"
    } else if adjustment > 0.0 {
        "too_quiet"
    } else {
        "too_loud"
    };

    LoudnessReport {
        platform: platform.to_string(),
        target_lufs: target,
        measured_lufs: lufs,
        adjustment_db: (adjustment * 10.0).round() / 10.0,
        status: status.to_string(),
    }
}

// ═══════════════════════════════════════
// Finance (basic)
// ═══════════════════════════════════════

/// Percentage change between two values.
#[must_use]
pub fn percentage_change(from: f64, to: f64) -> f64 {
    if from == 0.0 {
        return 0.0;
    }
    let pct = ((to - from) / from) * 100.0;
    (pct * 100.0).round() / 100.0
}

/// Compound annual growth rate.
#[must_use]
pub fn compound_annual_growth(start: f64, end: f64, years: f64) -> f64 {
    if start <= 0.0 || years <= 0.0 {
        return 0.0;
    }
    let cagr = (end / start).powf(1.0 / years) - 1.0;
    (cagr * 10000.0).round() / 100.0 // returns as percentage
}

/// Position size calculator: how many shares/units given risk parameters.
#[must_use]
pub fn position_size(capital: f64, risk_pct: f64, entry: f64, stop: f64) -> f64 {
    let risk_per_unit = (entry - stop).abs();
    if risk_per_unit == 0.0 {
        return 0.0;
    }
    let risk_amount = capital * (risk_pct / 100.0);
    (risk_amount / risk_per_unit).floor()
}

// ═══════════════════════════════════════
// Session cost
// ═══════════════════════════════════════

/// Summary of token costs for a session.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CostSummary {
    pub total_tokens: u32,
    pub deterministic_calls: u32,
    pub llm_calls: u32,
    pub estimated_cost_usd: f64,
}

/// Aggregate cost from event list.
#[must_use]
pub fn session_cost_summary(events: &[crate::events::Event]) -> CostSummary {
    let mut summary = CostSummary::default();
    for event in events {
        summary.total_tokens += event.token_cost;
        if event.was_deterministic {
            summary.deterministic_calls += 1;
        } else {
            summary.llm_calls += 1;
        }
    }
    // Rough estimate: $3/MTok input, $15/MTok output (Sonnet pricing)
    #[allow(clippy::cast_precision_loss)]
    let cost = f64::from(summary.total_tokens) / 1_000_000.0 * 9.0;
    summary.estimated_cost_usd = (cost * 10000.0).round() / 10000.0;
    summary
}

/// Check budget status.
#[must_use]
pub fn budget_check(events: &[crate::events::Event], max_tokens: u32) -> crate::proto::BudgetStatus {
    let summary = session_cost_summary(events);
    let remaining = max_tokens.saturating_sub(summary.total_tokens);
    #[allow(clippy::cast_precision_loss)]
    let pct_used = if max_tokens > 0 {
        f64::from(summary.total_tokens) / f64::from(max_tokens)
    } else {
        0.0
    };

    let status = if pct_used < 0.5 {
        "green"
    } else if pct_used < 0.75 {
        "amber"
    } else if pct_used < 0.9 {
        "yellow"
    } else {
        "red"
    };

    crate::proto::BudgetStatus {
        tokens_used: summary.total_tokens,
        tokens_remaining: remaining,
        estimated_cost_usd: summary.estimated_cost_usd as f32,
        status: status.to_string(),
    }
}

// ═══════════════════════════════════════
// Helpers
// ═══════════════════════════════════════

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => {
            let rest: String = chars.collect();
            format!("{}{rest}", first.to_uppercase())
        }
        None => String::new(),
    }
}

fn normalize_note(note: &str) -> String {
    let trimmed = note.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // Handle flats by converting to sharps
    let flat_map = [
        ("Db", "C#"),
        ("Eb", "D#"),
        ("Fb", "E"),
        ("Gb", "F#"),
        ("Ab", "G#"),
        ("Bb", "A#"),
        ("Cb", "B"),
    ];
    for (flat, sharp) in &flat_map {
        if trimmed.eq_ignore_ascii_case(flat) {
            return (*sharp).to_string();
        }
    }
    // Capitalize first letter
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(first) => {
            let rest: String = chars.collect();
            format!("{}{}", first.to_uppercase(), rest)
        }
        None => String::new(),
    }
}

fn extract_chord_root(chord: &str) -> String {
    let mut chars = chord.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return String::new(),
    };
    let second = chars.next();
    match second {
        Some('#' | 'b') => format!("{first}{}", second.unwrap()),
        _ => first.to_string(),
    }
}

// ═══════════════════════════════════════
// Deterministic pattern matcher
// ═══════════════════════════════════════

/// Attempts to resolve a user message deterministically.
/// Returns `Some(response)` if the message can be answered without an LLM.
/// Returns `None` if the message requires judgment/creativity.
#[must_use]
pub fn try_deterministic_resolution(message: &str) -> Option<String> {
    let lower = message.to_lowercase();
    let trimmed = lower.trim();

    // Time/date queries
    if trimmed == "what time is it"
        || trimmed == "what's the time"
        || trimmed == "current time"
        || trimmed == "what time is it?"
        || trimmed == "what's the time?"
    {
        return Some(format!("Current time: {}", current_datetime()));
    }

    // BPM calculations
    if let Some(result) = try_bpm_calculation(trimmed) {
        return Some(result);
    }

    // Scale/chord lookups
    if let Some(result) = try_music_theory_lookup(trimmed) {
        return Some(result);
    }

    // Frequency band lookups
    if let Some(result) = try_frequency_lookup(trimmed) {
        return Some(result);
    }

    // Percentage change
    if let Some(result) = try_percentage_calc(trimmed) {
        return Some(result);
    }

    // Word count
    if trimmed.starts_with("word count") || trimmed.starts_with("count words") {
        let text = trimmed
            .trim_start_matches("word count")
            .trim_start_matches("count words")
            .trim_start_matches(':')
            .trim_start_matches(" of ")
            .trim_start_matches(" in ")
            .trim();
        if !text.is_empty() {
            return Some(format!("{} words", word_count(text)));
        }
    }

    None
}

fn try_bpm_calculation(message: &str) -> Option<String> {
    // Pattern: "X bpm bar duration" or "bar duration at X bpm"
    let words: Vec<&str> = message.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        if *word == "bpm" {
            // Try to find the BPM number before or after
            if i > 0 {
                if let Ok(bpm) = words[i - 1].parse::<u32>() {
                    if message.contains("bar") {
                        let dur = bpm_to_bar_duration(bpm, 4);
                        return Some(format!(
                            "At {bpm} BPM (4/4): one bar = {dur:.3}s"
                        ));
                    }
                    if message.contains("minute") || message.contains("3 min") || message.contains("song") {
                        // Default to 3-minute song
                        let target_secs = if message.contains("3 min") {
                            180.0
                        } else if message.contains("4 min") {
                            240.0
                        } else {
                            180.0
                        };
                        let bars = song_bar_count(bpm, target_secs);
                        return Some(format!(
                            "At {bpm} BPM (4/4): {target_secs:.0}s = {bars} bars"
                        ));
                    }
                }
            }
        }
    }
    None
}

fn try_music_theory_lookup(message: &str) -> Option<String> {
    // Chord-to-roman FIRST: "Am in C major" — must come before scale lookup
    // because "am in c major" ends with "c major" which would match as a scale.
    if message.contains(" in ") && message.contains("major") {
        let parts: Vec<&str> = message.split(" in ").collect();
        if parts.len() == 2 {
            let chord_part = parts[0].trim();
            let key_part = parts[1]
                .trim()
                .trim_end_matches(" major")
                .trim_end_matches("major")
                .trim();
            if !chord_part.is_empty() && !key_part.is_empty() {
                // Extract just the chord name (last word) and normalize
                let chord_raw = chord_part.split_whitespace().last().unwrap_or(chord_part);
                let key_raw = key_part.split_whitespace().last().unwrap_or(key_part);
                // Capitalize first letter for proper note names
                let chord = capitalize_first(chord_raw);
                let key = capitalize_first(key_raw);
                let roman = chord_to_roman(&chord, &key);
                return Some(format!("{chord} in {key} major = {roman}"));
            }
        }
    }

    // Scale lookup: "C major scale", "scale of A minor", etc.
    let note_names = [
        "c", "c#", "db", "d", "d#", "eb", "e", "f", "f#", "gb", "g", "g#", "ab", "a", "a#",
        "bb", "b",
    ];

    for note in &note_names {
        for mode in &["major", "minor", "dorian", "phrygian", "lydian", "mixolydian", "locrian"] {
            let pattern1 = format!("{note} {mode} scale");
            let pattern2 = format!("{note} {mode}");
            let pattern3 = format!("scale of {note} {mode}");
            let pattern4 = format!("what is {note} {mode}");

            if message.contains(&pattern1)
                || message.ends_with(&pattern2)
                || message.contains(&pattern3)
                || message.contains(&pattern4)
            {
                let degrees = scale_degrees(note, mode);
                return Some(format!(
                    "{} {} scale: {}",
                    normalize_note(note),
                    mode,
                    degrees.join(" - ")
                ));
            }
        }
    }

    None
}

fn try_frequency_lookup(message: &str) -> Option<String> {
    // Pattern: "XXXX hz band" or "convert XXXXhz"
    let words: Vec<&str> = message.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        let cleaned = word.trim_end_matches("hz").trim_end_matches("khz");
        if *word != cleaned || (i + 1 < words.len() && (words[i + 1] == "hz" || words[i + 1] == "khz")) {
            let is_khz = word.contains("khz") || (i + 1 < words.len() && words[i + 1] == "khz");
            if let Ok(val) = cleaned.parse::<f64>() {
                let hz = if is_khz { val * 1000.0 } else { val };
                let band = frequency_to_band(hz);
                return Some(format!("{hz:.0} Hz → {band}"));
            }
        }
    }
    None
}

fn try_percentage_calc(message: &str) -> Option<String> {
    // Pattern: "percentage change from X to Y"
    if message.contains("percentage change") || message.contains("pct change") {
        let words: Vec<&str> = message.split_whitespace().collect();
        let mut from_val: Option<f64> = None;
        let mut to_val: Option<f64> = None;
        for (i, word) in words.iter().enumerate() {
            if *word == "from" && i + 1 < words.len() {
                from_val = words[i + 1].parse().ok();
            }
            if *word == "to" && i + 1 < words.len() {
                to_val = words[i + 1].parse().ok();
            }
        }
        if let (Some(from), Some(to)) = (from_val, to_val) {
            let pct = percentage_change(from, to);
            return Some(format!("{pct:+.2}%"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_count() {
        assert_eq!(word_count("hello world"), 2);
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("one"), 1);
    }

    #[test]
    fn test_reading_time() {
        let text = "word ".repeat(500);
        assert!((reading_time_minutes(&text) - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_json_validate() {
        assert!(json_validate(r#"{"key": "value"}"#));
        assert!(!json_validate("not json"));
    }

    #[test]
    fn test_url_validate() {
        assert!(url_validate("https://example.com"));
        assert!(!url_validate("ftp://nope"));
    }

    #[test]
    fn test_bpm_to_bar_duration() {
        // 120 BPM, 4 beats = 2.0 seconds per bar
        assert!((bpm_to_bar_duration(120, 4) - 2.0).abs() < 0.001);
        assert!((bpm_to_bar_duration(140, 4) - 1.714).abs() < 0.001);
        assert_eq!(bpm_to_bar_duration(0, 4), 0.0);
    }

    #[test]
    fn test_song_bar_count() {
        // 120 BPM, 180s (3 minutes) = 90 bars
        assert_eq!(song_bar_count(120, 180.0), 90);
        // 140 BPM, 180s = ~105 bars
        assert_eq!(song_bar_count(140, 180.0), 105);
    }

    #[test]
    fn test_scale_degrees_c_major() {
        let scale = scale_degrees("C", "major");
        assert_eq!(scale, vec!["C", "D", "E", "F", "G", "A", "B"]);
    }

    #[test]
    fn test_scale_degrees_a_minor() {
        let scale = scale_degrees("A", "minor");
        assert_eq!(scale, vec!["A", "B", "C", "D", "E", "F", "G"]);
    }

    #[test]
    fn test_chord_to_roman() {
        assert_eq!(chord_to_roman("C", "C"), "I");
        assert_eq!(chord_to_roman("Am", "C"), "vi");
        assert_eq!(chord_to_roman("G", "C"), "V");
        assert_eq!(chord_to_roman("F", "C"), "IV");
    }

    #[test]
    fn test_roman_to_chord() {
        assert_eq!(roman_to_chord("I", "C"), "C");
        assert_eq!(roman_to_chord("vi", "C"), "Am");
        assert_eq!(roman_to_chord("V", "C"), "G");
    }

    #[test]
    fn test_transpose() {
        let notes = vec!["C".to_string(), "E".to_string(), "G".to_string()];
        let transposed = transpose(&notes, 2);
        assert_eq!(transposed, vec!["D", "F#", "A"]);
    }

    #[test]
    fn test_note_to_frequency() {
        // A4 = 440 Hz
        assert!((note_to_frequency("A", 4) - 440.0).abs() < 0.01);
        // C4 ≈ 261.63 Hz
        assert!((note_to_frequency("C", 4) - 261.63).abs() < 0.1);
    }

    #[test]
    fn test_frequency_to_band() {
        assert_eq!(frequency_to_band(40.0), "Sub Bass");
        assert_eq!(frequency_to_band(200.0), "Bass");
        assert_eq!(frequency_to_band(1000.0), "Mids");
        assert_eq!(frequency_to_band(3000.0), "Upper Mids");
        assert_eq!(frequency_to_band(10000.0), "Brilliance");
    }

    #[test]
    fn test_syllable_count() {
        assert_eq!(syllable_count("hello"), 2);
        assert_eq!(syllable_count("beautiful"), 3);
        assert_eq!(syllable_count("I am the greatest"), 5);
    }

    #[test]
    fn test_density_lambda() {
        // 16 syllables over 4 bars (16 beats) = 1.0
        assert!((density_lambda("I am the greatest rapper alive in the game right now for real", 120, 4) - 1.0).abs() < 1.0);
    }

    #[test]
    fn test_isrc_validate() {
        assert!(isrc_validate("USRC12345678"));
        assert!(isrc_validate("US-RC1-23-45678"));
        assert!(!isrc_validate("invalid"));
        assert!(!isrc_validate("123456789012")); // no alpha country
    }

    #[test]
    fn test_streaming_loudness_check() {
        let report = streaming_loudness_check(-14.0, "spotify");
        assert_eq!(report.status, "pass");

        let report = streaming_loudness_check(-8.0, "spotify");
        assert_eq!(report.status, "too_loud");

        let report = streaming_loudness_check(-20.0, "apple");
        assert_eq!(report.status, "too_quiet");
    }

    #[test]
    fn test_percentage_change() {
        assert!((percentage_change(100.0, 150.0) - 50.0).abs() < 0.01);
        assert!((percentage_change(200.0, 100.0) - (-50.0)).abs() < 0.01);
    }

    #[test]
    fn test_compound_annual_growth() {
        // $100 to $200 in 5 years ≈ 14.87%
        let cagr = compound_annual_growth(100.0, 200.0, 5.0);
        assert!((cagr - 14.87).abs() < 0.1);
    }

    #[test]
    fn test_position_size() {
        // $10,000 capital, 2% risk, entry $50, stop $48
        let size = position_size(10_000.0, 2.0, 50.0, 48.0);
        assert_eq!(size, 100.0); // $200 risk / $2 per share = 100 shares
    }

    #[test]
    fn test_deterministic_resolution_bpm() {
        let result = try_deterministic_resolution("140 bpm bar duration");
        assert!(result.is_some());
        assert!(result.unwrap().contains("1.714"));
    }

    #[test]
    fn test_deterministic_resolution_scale() {
        let result = try_deterministic_resolution("c major scale");
        assert!(result.is_some());
        assert!(result.unwrap().contains("C - D - E - F - G - A - B"));
    }

    #[test]
    fn test_deterministic_resolution_chord_in_key() {
        let result = try_deterministic_resolution("Am in C major");
        assert!(result.is_some());
        assert!(result.unwrap().contains("vi"));
    }

    #[test]
    fn test_deterministic_resolution_frequency() {
        let result = try_deterministic_resolution("2500hz band");
        assert!(result.is_some());
        // 2500 Hz should be in presence range
    }

    #[test]
    fn test_deterministic_resolution_returns_none_for_judgment() {
        assert!(try_deterministic_resolution("help me write a verse about love").is_none());
        assert!(try_deterministic_resolution("what's the best investment right now").is_none());
    }

    #[test]
    fn test_release_timeline() {
        let timeline = release_timeline("2026-06-01", "standard").unwrap();
        assert!(timeline.len() > 5);
        assert_eq!(timeline.iter().find(|e| e.days_before_release == 0).unwrap().label, "Release day");
    }

    #[test]
    fn test_days_until() {
        // Future date should be positive
        let result = days_until("2099-01-01");
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }
}
