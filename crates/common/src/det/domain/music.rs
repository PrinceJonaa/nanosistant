//! Music domain deterministic functions — expanded module.
//!
//! Zero-token music theory, production, and analysis utilities.
//! All functions are pure: same input always produces same output.

use serde::Serialize;

// ═══════════════════════════════════════
// Tempo & Rhythm
// ═══════════════════════════════════════

/// Duration of one bar in seconds.
#[must_use]
pub fn bpm_to_bar_duration(bpm: u32, beats_per_bar: u32) -> f64 {
    if bpm == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let d = (60.0 / bpm as f64) * beats_per_bar as f64;
    (d * 1000.0).round() / 1000.0
}

/// Number of bars needed for a target duration (4/4).
#[must_use]
pub fn song_bar_count(bpm: u32, target_duration_secs: f64) -> u32 {
    if bpm == 0 { return 0; }
    #[allow(clippy::cast_precision_loss)]
    let bar_dur = (60.0 / bpm as f64) * 4.0;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = (target_duration_secs / bar_dur).ceil() as u32;
    count
}

/// Convert tempo (BPM) to a human-readable feel label.
#[must_use]
pub fn tempo_feel(bpm: u32) -> &'static str {
    match bpm {
        0..=39    => "Grave (very slow)",
        40..=59   => "Largo (broad, slow)",
        60..=65   => "Larghetto",
        66..=75   => "Adagio (slow, stately)",
        76..=107  => "Andante (walking pace)",
        108..=119 => "Moderato",
        120..=155 => "Allegro (fast, lively)",
        156..=175 => "Vivace (lively, rapid)",
        176..=199 => "Presto (very fast)",
        200..=u32::MAX => "Prestissimo (as fast as possible)",
    }
}

/// Classify a BPM into a genre-typical range.
#[must_use]
pub fn bpm_genre_hint(bpm: u32) -> &'static str {
    match bpm {
        0..=69    => "Downtempo / Ambient",
        70..=89   => "Hip-Hop / R&B / Reggae",
        90..=109  => "Pop / Classic Hip-Hop",
        110..=129 => "Pop / Dance / House",
        130..=149 => "House / Trance",
        150..=174 => "Drum & Bass / Jersey Club / Footwork",
        175..=199 => "Drum & Bass / Jungle / Hardstyle",
        200..=u32::MAX => "Speedcore / Extratone",
    }
}

/// Milliseconds per beat at a given BPM.
#[must_use]
pub fn ms_per_beat(bpm: u32) -> f64 {
    if bpm == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let ms = 60_000.0 / bpm as f64;
    (ms * 100.0).round() / 100.0
}

/// Note duration in ms at a given BPM (quarter=1, half=2, eighth=0.5, etc.).
#[must_use]
pub fn note_duration_ms(bpm: u32, note_value: f64) -> f64 {
    if bpm == 0 || note_value <= 0.0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let quarter_ms = 60_000.0 / bpm as f64;
    let ms = quarter_ms * note_value;
    (ms * 100.0).round() / 100.0
}

/// Common delay times in ms for a given BPM (1/4, 1/8, dotted 1/8, 1/16).
#[derive(Debug, Serialize)]
pub struct DelayTimes {
    pub quarter_ms: f64,
    pub eighth_ms: f64,
    pub dotted_eighth_ms: f64,
    pub sixteenth_ms: f64,
    pub triplet_eighth_ms: f64,
}

#[must_use]
pub fn delay_times(bpm: u32) -> DelayTimes {
    let q = ms_per_beat(bpm);
    DelayTimes {
        quarter_ms: q,
        eighth_ms: (q / 2.0 * 100.0).round() / 100.0,
        dotted_eighth_ms: (q * 0.75 * 100.0).round() / 100.0,
        sixteenth_ms: (q / 4.0 * 100.0).round() / 100.0,
        triplet_eighth_ms: (q / 3.0 * 100.0).round() / 100.0,
    }
}

// ═══════════════════════════════════════
// Music Theory
// ═══════════════════════════════════════

const CHROMATIC: [&str; 12] = ["C","C#","D","D#","E","F","F#","G","G#","A","A#","B"];

fn note_index(note: &str) -> Option<usize> {
    let normalized = normalize_note_name(note);
    CHROMATIC.iter().position(|&n| n == normalized)
}

fn normalize_note_name(note: &str) -> String {
    let flat_map = [("Db","C#"),("Eb","D#"),("Fb","E"),("Gb","F#"),
                    ("Ab","G#"),("Bb","A#"),("Cb","B")];
    let trimmed = note.trim();
    for (flat, sharp) in &flat_map {
        if trimmed.eq_ignore_ascii_case(flat) { return (*sharp).to_string(); }
    }
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.collect::<String>()),
        None => String::new(),
    }
}

/// Scale degrees for a key and mode.
#[must_use]
pub fn scale_degrees(key: &str, mode: &str) -> Vec<String> {
    let start = note_index(key).unwrap_or(0);
    let intervals: Vec<usize> = match mode.to_lowercase().as_str() {
        "major" | "ionian"          => vec![0,2,4,5,7,9,11],
        "minor" | "aeolian"         => vec![0,2,3,5,7,8,10],
        "dorian"                    => vec![0,2,3,5,7,9,10],
        "phrygian"                  => vec![0,1,3,5,7,8,10],
        "lydian"                    => vec![0,2,4,6,7,9,11],
        "mixolydian"                => vec![0,2,4,5,7,9,10],
        "locrian"                   => vec![0,1,3,5,6,8,10],
        "harmonic_minor"            => vec![0,2,3,5,7,8,11],
        "melodic_minor"             => vec![0,2,3,5,7,9,11],
        "pentatonic_major"          => vec![0,2,4,7,9],
        "pentatonic_minor"          => vec![0,3,5,7,10],
        "blues"                     => vec![0,3,5,6,7,10],
        "whole_tone"                => vec![0,2,4,6,8,10],
        "diminished"                => vec![0,2,3,5,6,8,9,11],
        _                           => vec![0,2,4,5,7,9,11],
    };
    intervals.iter().map(|&i| CHROMATIC[(start + i) % 12].to_string()).collect()
}

/// Convert chord to roman numeral in a given key.
#[must_use]
pub fn chord_to_roman(chord: &str, key: &str) -> String {
    let scale = scale_degrees(key, "major");
    let root = extract_root(chord);
    let norm_root = normalize_note_name(&root);
    let is_minor = chord.contains('m') && !chord.to_lowercase().contains("maj");
    let roman = ["I","II","III","IV","V","VI","VII"];
    match scale.iter().position(|n| *n == norm_root) {
        Some(idx) => {
            if is_minor { roman[idx].to_lowercase() } else { roman[idx].to_string() }
        }
        None => format!("? ({chord} not in {key} major)"),
    }
}

/// Roman numeral → chord name.
#[must_use]
pub fn roman_to_chord(roman: &str, key: &str) -> String {
    let scale = scale_degrees(key, "major");
    let upper = roman.to_uppercase();
    let is_minor = roman.chars().next().map(|c| c.is_lowercase()).unwrap_or(false);
    let map = ["I","II","III","IV","V","VI","VII"];
    match map.iter().position(|&r| r == upper) {
        Some(idx) => {
            let root = &scale[idx];
            if is_minor { format!("{root}m") } else { root.clone() }
        }
        None => format!("? (unknown numeral: {roman})"),
    }
}

/// Transpose notes by semitones.
#[must_use]
pub fn transpose(notes: &[String], semitones: i32) -> Vec<String> {
    notes.iter().map(|n| {
        let normalized = normalize_note_name(n);
        if let Some(idx) = CHROMATIC.iter().position(|&c| c == normalized) {
            CHROMATIC[(idx as i32 + semitones).rem_euclid(12) as usize].to_string()
        } else { n.clone() }
    }).collect()
}

/// Concert pitch frequency (A4 = 440 Hz).
#[must_use]
pub fn note_to_frequency(note: &str, octave: u32) -> f64 {
    let norm = normalize_note_name(note);
    if let Some(idx) = CHROMATIC.iter().position(|&c| c == norm) {
        #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)]
        let semitone_from_a4 = (idx as i32 - 9) + (octave as i32 - 4) * 12;
        let freq = 440.0 * 2.0_f64.powf(semitone_from_a4 as f64 / 12.0);
        (freq * 100.0).round() / 100.0
    } else { 0.0 }
}

/// EQ band name for a frequency.
#[must_use]
pub fn frequency_to_band(hz: f64) -> &'static str {
    if hz < 20.0       { "Sub-bass (infrasonic edge)" }
    else if hz < 60.0  { "Sub Bass" }
    else if hz < 250.0 { "Bass" }
    else if hz < 500.0 { "Low Mids" }
    else if hz < 2000.0 { "Mids" }
    else if hz < 4000.0 { "Upper Mids" }
    else if hz < 6000.0 { "Presence" }
    else if hz < 12000.0 { "Brilliance" }
    else if hz < 20000.0 { "Air" }
    else                { "Ultrasonic" }
}

/// Chord quality from a chord name string.
#[must_use]
pub fn chord_quality(chord: &str) -> &'static str {
    let c = chord.to_lowercase();
    if c.contains("dim7")        { "Diminished 7th" }
    else if c.contains("dim")    { "Diminished" }
    else if c.contains("aug")    { "Augmented" }
    else if c.contains("maj7")   { "Major 7th" }
    else if c.contains("maj9")   { "Major 9th" }
    else if c.contains("m7")     { "Minor 7th" }
    else if c.contains("m9")     { "Minor 9th" }
    else if c.contains("m11")    { "Minor 11th" }
    else if c.contains("m13")    { "Minor 13th" }
    else if c.contains('m')      { "Minor" }
    else if c.contains("7")      { "Dominant 7th" }
    else if c.contains("9")      { "Dominant 9th" }
    else if c.contains("11")     { "11th" }
    else if c.contains("13")     { "13th" }
    else if c.contains("sus4")   { "Suspended 4th" }
    else if c.contains("sus2")   { "Suspended 2nd" }
    else if c.contains("add9")   { "Add 9" }
    else                         { "Major" }
}

/// Intervals in a chord (major, minor, dominant 7, etc.).
#[must_use]
pub fn chord_intervals(quality: &str) -> Vec<u32> {
    match quality.to_lowercase().as_str() {
        "major"           => vec![0, 4, 7],
        "minor"           => vec![0, 3, 7],
        "dominant7"       => vec![0, 4, 7, 10],
        "major7"          => vec![0, 4, 7, 11],
        "minor7"          => vec![0, 3, 7, 10],
        "diminished"      => vec![0, 3, 6],
        "diminished7"     => vec![0, 3, 6, 9],
        "augmented"       => vec![0, 4, 8],
        "sus2"            => vec![0, 2, 7],
        "sus4"            => vec![0, 5, 7],
        "major9"          => vec![0, 4, 7, 11, 14],
        "minor9"          => vec![0, 3, 7, 10, 14],
        _                 => vec![0, 4, 7],
    }
}

// ═══════════════════════════════════════
// Lyrics & Vocal
// ═══════════════════════════════════════

/// Approximate syllable count using vowel-group heuristic.
#[must_use]
pub fn syllable_count(text: &str) -> u32 {
    let mut total = 0u32;
    for word in text.split_whitespace() {
        let lower = word.to_lowercase();
        let clean: String = lower.chars().filter(|c| c.is_alphabetic()).collect();
        if clean.is_empty() { continue; }
        let mut count = 0u32;
        let mut prev_vowel = false;
        for ch in clean.chars() {
            let is_vowel = matches!(ch, 'a'|'e'|'i'|'o'|'u'|'y');
            if is_vowel && !prev_vowel { count += 1; }
            prev_vowel = is_vowel;
        }
        if clean.ends_with('e') && count > 1 { count -= 1; }
        total += count.max(1);
    }
    total
}

/// Syllable density: syllables per beat.
#[must_use]
pub fn density_lambda(text: &str, bars: u32) -> f64 {
    let syllables = syllable_count(text);
    let beats = bars * 4;
    if beats == 0 { return 0.0; }
    #[allow(clippy::cast_precision_loss)]
    let lambda = syllables as f64 / beats as f64;
    (lambda * 100.0).round() / 100.0
}

/// Rhyme scheme detector (A/B/C labels for last phoneme groups).
#[must_use]
pub fn rhyme_scheme(lines: &[&str]) -> Vec<String> {
    let mut scheme = Vec::new();
    let mut seen: Vec<(String, char)> = Vec::new();
    let mut next_label = b'A';

    for line in lines {
        let last = last_stressed_vowel_group(line.trim());
        let label = if let Some((_, lbl)) = seen.iter().find(|(k, _)| *k == last) {
            lbl.to_string()
        } else {
            let lbl = next_label as char;
            seen.push((last, lbl));
            next_label += 1;
            lbl.to_string()
        };
        scheme.push(label);
    }
    scheme
}

fn last_stressed_vowel_group(line: &str) -> String {
    // Simplified: last 3 chars of last word, lowercased
    let last_word: String = line.split_whitespace()
        .last().unwrap_or("").chars()
        .filter(|c| c.is_alphabetic()).collect();
    let lower = last_word.to_lowercase();
    if lower.len() >= 3 { lower[lower.len()-3..].to_string() }
    else { lower }
}

// ═══════════════════════════════════════
// Production / Mixing
// ═══════════════════════════════════════

/// Streaming platform loudness targets.
#[derive(Debug, Serialize)]
pub struct LoudnessReport {
    pub platform: String,
    pub target_lufs: f64,
    pub measured_lufs: f64,
    pub adjustment_db: f64,
    pub status: String,
}

#[must_use]
pub fn streaming_loudness_check(lufs: f64, platform: &str) -> LoudnessReport {
    let target = match platform.to_lowercase().as_str() {
        "spotify"       => -14.0,
        "apple" | "apple_music" => -16.0,
        "youtube"       => -14.0,
        "amazon"        => -14.0,
        "tidal"         => -14.0,
        "soundcloud"    => -8.0,
        "bandcamp"      => -14.0,
        _               => -14.0,
    };
    let adj = target - lufs;
    let status = if adj.abs() < 0.5 { "pass" }
                 else if adj > 0.0   { "too_quiet" }
                 else                { "too_loud" };
    LoudnessReport {
        platform: platform.to_string(),
        target_lufs: target,
        measured_lufs: lufs,
        adjustment_db: (adj * 10.0).round() / 10.0,
        status: status.to_string(),
    }
}

/// Reverberation time (RT60) estimate from room dimensions (Sabine formula).
#[must_use]
pub fn rt60_sabine(volume_m3: f64, total_absorption: f64) -> f64 {
    if total_absorption <= 0.0 { return 0.0; }
    let rt = 0.161 * volume_m3 / total_absorption;
    (rt * 1000.0).round() / 1000.0
}

/// Haas effect threshold — delays under 30ms perceived as one source.
#[must_use]
pub fn haas_threshold_ms() -> f64 { 30.0 }

/// Signal-to-noise ratio in dB.
#[must_use]
pub fn snr_db(signal_power: f64, noise_power: f64) -> f64 {
    if signal_power <= 0.0 || noise_power <= 0.0 { return 0.0; }
    10.0 * (signal_power / noise_power).log10()
}

// ═══════════════════════════════════════
// Release / Business
// ═══════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct TimelineEntry {
    pub date: String,
    pub label: String,
    pub days_before_release: i64,
}

pub fn release_timeline(release_date: &str, template: &str) -> Result<Vec<TimelineEntry>, String> {
    use chrono::NaiveDate;
    let release = NaiveDate::parse_from_str(release_date, "%Y-%m-%d")
        .map_err(|e| format!("invalid date: {e}"))?;
    let offsets: Vec<(i64, &str)> = match template {
        "standard" => vec![
            (-28, "Final master due"),
            (-21, "Distributor upload deadline"),
            (-14, "Pre-save campaign launch"),
            (-7,  "Single/teaser release"),
            (-3,  "Social media push begins"),
            (0,   "Release day"),
            (1,   "Playlist pitching follow-up"),
            (7,   "First-week analytics review"),
        ],
        "ep" => vec![
            (-42, "Track listing finalized"),
            (-35, "All masters submitted"),
            (-28, "Distributor upload"),
            (-14, "Promo rollout begins"),
            (-7,  "First single drops"),
            (0,   "EP release day"),
            (7,   "Analytics review"),
            (14,  "Follow-up pitch"),
        ],
        _ => vec![(0, "Release day")],
    };
    Ok(offsets.into_iter().map(|(days, label)| {
        let date = release + chrono::Duration::days(days);
        TimelineEntry {
            date: date.format("%Y-%m-%d").to_string(),
            label: label.to_string(),
            days_before_release: -days,
        }
    }).collect())
}

/// Validate ISRC code format.
#[must_use]
pub fn isrc_validate(code: &str) -> bool {
    let clean: String = code.chars().filter(|c| c.is_alphanumeric()).collect();
    if clean.len() != 12 { return false; }
    let chars: Vec<char> = clean.chars().collect();
    chars[0..2].iter().all(|c| c.is_alphabetic())
        && chars[2..5].iter().all(|c| c.is_alphanumeric())
        && chars[5..7].iter().all(|c| c.is_ascii_digit())
        && chars[7..12].iter().all(|c| c.is_ascii_digit())
}

// ═══════════════════════════════════════
// Helpers
// ═══════════════════════════════════════

fn extract_root(chord: &str) -> String {
    let mut chars = chord.chars();
    let first = match chars.next() { Some(c) => c, None => return String::new() };
    let second = chars.next();
    match second {
        Some('#') | Some('b') => format!("{first}{}", second.unwrap()),
        _ => first.to_string(),
    }
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn tempo_feel_labels() {
        assert_eq!(tempo_feel(120), "Allegro (fast, lively)");
        assert_eq!(tempo_feel(90), "Andante (walking pace)"); // 90 is in 76-107
        assert_eq!(tempo_feel(70), "Adagio (slow, stately)"); // 70 is in 66-75
        assert_eq!(tempo_feel(60), "Larghetto"); // 60 is in 60-65
    }
    #[test] fn bpm_genre_hint_works() {
        assert_eq!(bpm_genre_hint(90), "Pop / Classic Hip-Hop");
        assert_eq!(bpm_genre_hint(160), "Drum & Bass / Jersey Club / Footwork");
    }
    #[test] fn delay_times_at_120() {
        let d = delay_times(120);
        assert!((d.quarter_ms - 500.0).abs() < 0.1);
        assert!((d.eighth_ms - 250.0).abs() < 0.1);
        assert!((d.dotted_eighth_ms - 375.0).abs() < 0.1);
    }
    #[test] fn note_duration_ms_at_120() {
        assert!((note_duration_ms(120, 1.0) - 500.0).abs() < 0.1);
        assert!((note_duration_ms(120, 0.5) - 250.0).abs() < 0.1);
    }
    #[test] fn scale_pentatonic() {
        let s = scale_degrees("C", "pentatonic_major");
        assert_eq!(s, vec!["C","D","E","G","A"]);
    }
    #[test] fn scale_blues() {
        let s = scale_degrees("A", "blues");
        assert_eq!(s.len(), 6);
    }
    #[test] fn chord_quality_detection() {
        assert_eq!(chord_quality("Cmaj7"), "Major 7th");
        assert_eq!(chord_quality("Am7"), "Minor 7th");
        assert_eq!(chord_quality("Bdim"), "Diminished");
    }
    #[test] fn rhyme_scheme_basic() {
        // ABAB pattern — all different last-3
        let lines = vec!["this is the end", "around the bend", "we stand alone", "cast in stone"];
        let scheme = rhyme_scheme(&lines);
        // "end" and "bend" share last 3 chars "end"/"end" → same label
        assert_eq!(scheme[0], scheme[1]);
        // "lone" and "one" share last 3 "one"/"one" → same
        assert_eq!(scheme[2], scheme[3]);
    }
    #[test] fn rt60_sabine_calc() {
        let rt = rt60_sabine(100.0, 20.0);
        assert!((rt - 0.805).abs() < 0.01);
    }
    #[test] fn loudness_check_pass() {
        let r = streaming_loudness_check(-14.0, "spotify");
        assert_eq!(r.status, "pass");
    }
    #[test] fn isrc_valid() {
        assert!(isrc_validate("USRC12345678"));
        assert!(!isrc_validate("invalid"));
    }
}
