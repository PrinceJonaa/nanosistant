// MusicTheory.swift
// NanoClawKit
//
// Swift port of the music-domain deterministic functions in
// `crates/common/src/deterministic.rs`.  All results match the Rust
// originals to the same decimal precision.

import Foundation

// MARK: - MusicTheory

/// Pure, stateless music-theory helpers. Every function is a direct port of
/// the Rust deterministic crate — same rounding, same chromatic layout.
public enum MusicTheory {

    // MARK: - Chromatic constants

    /// Western 12-tone chromatic scale using sharps.
    public static let chromatic: [String] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"
    ]

    // MARK: - BPM / Timing

    /// Duration of one bar in seconds for the given BPM and time signature
    /// numerator (beats per bar).
    ///
    /// Matches Rust `bpm_to_bar_duration`:
    /// `round((60 / bpm) * beatsPerBar, to: 3 decimal places)`
    public static func bpmToBarDuration(bpm: UInt32, beatsPerBar: UInt32 = 4) -> Double {
        guard bpm > 0 else { return 0.0 }
        let duration = (60.0 / Double(bpm)) * Double(beatsPerBar)
        return (duration * 1000.0).rounded() / 1000.0
    }

    /// Number of bars (ceiled) required to fill `targetDurationSecs` at
    /// `bpm` in 4/4.
    ///
    /// Matches Rust `song_bar_count` — uses **unrounded** bar duration
    /// for the division to avoid accumulated rounding error.
    public static func songBarCount(bpm: UInt32, targetDurationSecs: Double) -> UInt32 {
        guard bpm > 0 else { return 0 }
        let barDur = (60.0 / Double(bpm)) * 4.0   // unrounded, same as Rust
        return UInt32(ceil(targetDurationSecs / barDur))
    }

    // MARK: - Scales

    /// Returns the seven diatonic notes for the given key and mode.
    ///
    /// Supported modes: major / ionian, minor / aeolian / natural_minor,
    /// dorian, phrygian, lydian, mixolydian, locrian.
    /// Falls back to major for unknown mode strings.
    public static func scaleDegrees(key: String, mode: String) -> [String] {
        let start = chromatic.firstIndex(of: normalizeNote(key)) ?? 0

        let intervals: [Int]
        switch mode.lowercased() {
        case "major", "ionian":
            intervals = [0, 2, 4, 5, 7, 9, 11]
        case "minor", "aeolian", "natural_minor":
            intervals = [0, 2, 3, 5, 7, 8, 10]
        case "dorian":
            intervals = [0, 2, 3, 5, 7, 9, 10]
        case "phrygian":
            intervals = [0, 1, 3, 5, 7, 8, 10]
        case "lydian":
            intervals = [0, 2, 4, 6, 7, 9, 11]
        case "mixolydian":
            intervals = [0, 2, 4, 5, 7, 9, 10]
        case "locrian":
            intervals = [0, 1, 3, 5, 6, 8, 10]
        default:
            intervals = [0, 2, 4, 5, 7, 9, 11]   // default: major
        }

        return intervals.map { chromatic[(start + $0) % 12] }
    }

    // MARK: - Chord / Roman numeral

    /// Convert a chord name to its Roman numeral in `key` major.
    ///
    /// - Minor chords (containing "m" but not "maj") produce lowercase numerals.
    /// - Returns `"? (<chord> not in <key> major)"` when the root is not
    ///   diatonic to the key.
    public static func chordToRoman(chord: String, key: String) -> String {
        let scale = scaleDegrees(key: key, mode: "major")
        let root  = normalizeNote(extractChordRoot(chord))
        let isMinor = chord.contains("m") && !chord.contains("maj")

        let romanNumerals = ["I", "II", "III", "IV", "V", "VI", "VII"]

        if let idx = scale.firstIndex(of: root) {
            let numeral = romanNumerals[idx]
            return isMinor ? numeral.lowercased() : numeral
        }
        return "? (\(chord) not in \(key) major)"
    }

    /// Convert a Roman numeral back to a chord name in `key` major.
    ///
    /// - Lowercase numerals produce minor chords (e.g. "vi" in C → "Am").
    /// - Unknown numerals resolve to the first scale degree.
    public static func romanToChord(roman: String, key: String) -> String {
        let scale = scaleDegrees(key: key, mode: "major")
        let romanUpper = roman.uppercased()
        let romanNumerals = ["I", "II", "III", "IV", "V", "VI", "VII"]
        // A roman numeral is minor when all alphabetic characters are lowercase.
        let isMinor = roman.unicodeScalars.allSatisfy {
            !CharacterSet.uppercaseLetters.contains($0)
        }

        let idx = romanNumerals.firstIndex(of: romanUpper) ?? 0
        let root = scale[idx % scale.count]
        return isMinor ? "\(root)m" : root
    }

    // MARK: - Transposition

    /// Transpose each note in `notes` by `semitones` (positive = up, negative = down).
    ///
    /// Notes that cannot be found in the chromatic scale are passed through unchanged.
    public static func transpose(notes: [String], semitones: Int) -> [String] {
        notes.map { note in
            let normalized = normalizeNote(note)
            if let idx = chromatic.firstIndex(of: normalized) {
                let newIdx = ((idx + semitones) % 12 + 12) % 12
                return chromatic[newIdx]
            }
            return note
        }
    }

    // MARK: - Frequency

    /// Concert pitch frequency (Hz) for a named note and octave.
    ///
    /// Uses A4 = 440 Hz as the reference.
    /// Result is rounded to 2 decimal places, matching the Rust implementation.
    public static func noteToFrequency(note: String, octave: UInt32) -> Double {
        let normalized = normalizeNote(note)
        let idx = chromatic.firstIndex(of: normalized) ?? 0
        // Semitones from A4: A is at index 9
        let semitoneFromA4 = (Int(idx) - 9) + (Int(octave) - 4) * 12
        let freq = 440.0 * pow(2.0, Double(semitoneFromA4) / 12.0)
        return (freq * 100.0).rounded() / 100.0
    }

    /// Map a frequency (Hz) to its approximate EQ band name.
    ///
    /// Bands match the Rust `frequency_to_band` thresholds exactly.
    public static func frequencyToBand(hz: Double) -> String {
        if hz < 60.0   { return "Sub Bass"    }
        if hz < 250.0  { return "Bass"        }
        if hz < 500.0  { return "Low Mids"    }
        if hz < 2000.0 { return "Mids"        }
        if hz < 4000.0 { return "Upper Mids"  }
        if hz < 6000.0 { return "Presence"    }
        if hz < 12000.0 { return "Brilliance" }
        return "Air"
    }

    // MARK: - Lyrics / Syllables

    /// Approximate syllable count using the vowel-group heuristic with
    /// silent-e correction. Matches the Rust `syllable_count` exactly.
    ///
    /// Words with no vowels are counted as 1 syllable.
    public static func syllableCount(text: String) -> UInt32 {
        var total: UInt32 = 0
        let vowels: Set<Character> = ["a", "e", "i", "o", "u", "y"]

        for word in text.split(separator: " ") {
            let lower = word.lowercased()
            let clean = lower.filter { $0.isLetter }
            guard !clean.isEmpty else { continue }

            var count: UInt32 = 0
            var prevVowel = false
            for ch in clean {
                let isVowel = vowels.contains(ch)
                if isVowel && !prevVowel { count += 1 }
                prevVowel = isVowel
            }

            // Silent-e: if the word ends in 'e' and has > 1 syllable, subtract one.
            if clean.last == "e" && count > 1 {
                count -= 1
            }

            total += max(count, 1)
        }
        return total
    }

    // MARK: - Internal helpers (mirrors Rust private helpers)

    /// Normalize a note name to the chromatic array's canonical form.
    ///
    /// - Converts flats to their enharmonic sharp equivalents.
    /// - Capitalises the first letter.
    public static func normalizeNote(_ note: String) -> String {
        let trimmed = note.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return "" }

        let flatMap: [(String, String)] = [
            ("Db", "C#"), ("Eb", "D#"), ("Fb", "E"),
            ("Gb", "F#"), ("Ab", "G#"), ("Bb", "A#"),
            ("Cb", "B")
        ]
        for (flat, sharp) in flatMap {
            if trimmed.caseInsensitiveCompare(flat) == .orderedSame {
                return sharp
            }
        }

        // Capitalise first letter, keep the rest as-is.
        let first = trimmed.prefix(1).uppercased()
        let rest  = trimmed.dropFirst()
        return first + rest
    }

    /// Extract the root note from a chord name (e.g. "Am7" → "A", "C#maj7" → "C#").
    public static func extractChordRoot(_ chord: String) -> String {
        guard let first = chord.first else { return "" }
        let second = chord.dropFirst().first
        if second == "#" || second == "b" {
            return String(first) + String(second!)
        }
        return String(first)
    }
}
