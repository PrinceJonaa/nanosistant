// DeterministicResolver.swift
// NanoClawKit
//
// Swift port of `try_deterministic_resolution` and its helpers from
// `crates/common/src/deterministic.rs`.
// Zero network, zero allocations beyond string construction.

import Foundation

// MARK: - DeterministicResolver

/// Attempts to answer user messages locally without any LLM or network call.
///
/// The resolution logic is a faithful Swift port of the Rust
/// `try_deterministic_resolution` function. Patterns are tried in the same
/// order as the Rust implementation.
///
/// Usage:
/// ```swift
/// if let answer = DeterministicResolver.shared.resolve("c major scale") {
///     print(answer) // "C major scale: C - D - E - F - G - A - B"
/// }
/// ```
public final class DeterministicResolver: @unchecked Sendable {

    /// Shared singleton. Because all state is immutable (no stored properties),
    /// this is safe for concurrent use without a lock.
    public static let shared = DeterministicResolver()

    public init() {}

    // MARK: - Public API

    /// Try to resolve `message` without any LLM or network.
    ///
    /// - Returns: A response string if the message matches a deterministic
    ///   pattern, or `nil` if an LLM is required.
    public func resolve(_ message: String) -> String? {
        let lower = message.lowercased().trimmingCharacters(in: .whitespaces)

        if let result = tryTimeQuery(lower)        { return result }
        if let result = tryBPMCalculation(lower)   { return result }
        if let result = tryMusicTheoryLookup(lower){ return result }
        if let result = tryFrequencyLookup(lower)  { return result }
        if let result = tryPercentageCalc(lower)   { return result }
        if let result = tryWordCount(lower)        { return result }

        return nil
    }

    // MARK: - Time / Date

    private func tryTimeQuery(_ message: String) -> String? {
        let timePatterns = [
            "what time is it",
            "what's the time",
            "current time",
            "what time is it?",
            "what's the time?"
        ]
        guard timePatterns.contains(message) else { return nil }

        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return "Current time: \(formatter.string(from: Date()))"
    }

    // MARK: - BPM / Bar

    /// Matches:
    /// - "120 bpm bar duration"
    /// - "bar duration at 120 bpm"
    /// - "120 bpm 3 min song"
    private func tryBPMCalculation(_ message: String) -> String? {
        let words = message.split(separator: " ").map(String.init)

        for (i, word) in words.enumerated() {
            guard word == "bpm" else { continue }

            // BPM number is the token immediately before "bpm"
            guard i > 0, let bpm = UInt32(words[i - 1]) else { continue }

            if message.contains("bar") {
                let dur = MusicTheory.bpmToBarDuration(bpm: bpm, beatsPerBar: 4)
                return "At \(bpm) BPM (4/4): one bar = \(String(format: "%.3f", dur))s"
            }

            if message.contains("minute") || message.contains("3 min") || message.contains("song") {
                let targetSecs: Double = message.contains("4 min") ? 240.0 : 180.0
                let bars = MusicTheory.songBarCount(bpm: bpm, targetDurationSecs: targetSecs)
                return "At \(bpm) BPM (4/4): \(Int(targetSecs))s = \(bars) bars"
            }
        }
        return nil
    }

    // MARK: - Music theory

    private func tryMusicTheoryLookup(_ message: String) -> String? {
        // ── Chord-in-key must come first: "Am in C major" ──────────────────
        // This check precedes the scale-lookup loop because "c major" would
        // match the scale pattern if we checked scales first.
        if message.contains(" in ") && message.contains("major") {
            let parts = message.components(separatedBy: " in ")
            if parts.count == 2 {
                let chordPart = parts[0].trimmingCharacters(in: .whitespaces)
                let keyPart = parts[1]
                    .trimmingCharacters(in: .whitespaces)
                    .replacingOccurrences(of: " major", with: "")
                    .replacingOccurrences(of: "major", with: "")
                    .trimmingCharacters(in: .whitespaces)

                if !chordPart.isEmpty && !keyPart.isEmpty {
                    let chordRaw = chordPart.split(separator: " ").last.map(String.init) ?? chordPart
                    let keyRaw   = keyPart.split(separator: " ").last.map(String.init) ?? keyPart
                    let chord    = MusicTheory.normalizeNote(chordRaw)
                    let key      = MusicTheory.normalizeNote(keyRaw)
                    // Re-append "m" suffix if the original chord was minor
                    let chordName = chordRaw.contains("m") && !chordRaw.contains("maj")
                        ? chord + "m"
                        : chord
                    let roman = MusicTheory.chordToRoman(chord: chordName, key: key)
                    return "\(chordName) in \(key) major = \(roman)"
                }
            }
        }

        // ── Scale lookup ───────────────────────────────────────────────────
        let noteNames = [
            "c", "c#", "db", "d", "d#", "eb", "e", "f",
            "f#", "gb", "g", "g#", "ab", "a", "a#", "bb", "b"
        ]
        let modes = [
            "major", "minor", "dorian", "phrygian",
            "lydian", "mixolydian", "locrian"
        ]

        for note in noteNames {
            for mode in modes {
                let p1 = "\(note) \(mode) scale"
                let p2 = "\(note) \(mode)"
                let p3 = "scale of \(note) \(mode)"
                let p4 = "what is \(note) \(mode)"

                if message.contains(p1)
                    || message.hasSuffix(p2)
                    || message.contains(p3)
                    || message.contains(p4) {
                    let degrees = MusicTheory.scaleDegrees(key: note, mode: mode)
                    let canonical = MusicTheory.normalizeNote(note)
                    return "\(canonical) \(mode) scale: \(degrees.joined(separator: " - "))"
                }
            }
        }

        return nil
    }

    // MARK: - Frequency band

    /// Matches patterns like "2500hz band", "2.5khz", "2500 hz"
    private func tryFrequencyLookup(_ message: String) -> String? {
        let words = message.split(separator: " ").map(String.init)

        for (i, word) in words.enumerated() {
            // Case 1: suffix attached — "2500hz" or "2.5khz"
            let cleanedHz  = word.hasSuffix("hz")  ? String(word.dropLast(2)) : nil
            let cleanedKhz = word.hasSuffix("khz") ? String(word.dropLast(3)) : nil

            if let s = cleanedHz, let val = Double(s) {
                let band = MusicTheory.frequencyToBand(hz: val)
                return "\(Int(val)) Hz → \(band)"
            }
            if let s = cleanedKhz, let val = Double(s) {
                let hz = val * 1000.0
                let band = MusicTheory.frequencyToBand(hz: hz)
                return "\(Int(hz)) Hz → \(band)"
            }

            // Case 2: separate token — "2500 hz" or "2.5 khz"
            if i + 1 < words.count {
                let unit = words[i + 1]
                if unit == "hz", let val = Double(word) {
                    let band = MusicTheory.frequencyToBand(hz: val)
                    return "\(Int(val)) Hz → \(band)"
                }
                if unit == "khz", let val = Double(word) {
                    let hz = val * 1000.0
                    let band = MusicTheory.frequencyToBand(hz: hz)
                    return "\(Int(hz)) Hz → \(band)"
                }
            }
        }
        return nil
    }

    // MARK: - Percentage change

    /// Matches "percentage change from X to Y" and "pct change from X to Y"
    private func tryPercentageCalc(_ message: String) -> String? {
        guard message.contains("percentage change") || message.contains("pct change") else {
            return nil
        }

        let words = message.split(separator: " ").map(String.init)
        var fromVal: Double?
        var toVal: Double?

        for (i, word) in words.enumerated() {
            if word == "from" && i + 1 < words.count {
                fromVal = Double(words[i + 1])
            }
            if word == "to" && i + 1 < words.count {
                toVal = Double(words[i + 1])
            }
        }

        guard let from = fromVal, let to = toVal else { return nil }
        let pct = percentageChange(from: from, to: to)
        return String(format: "%+.2f%%", pct)
    }

    // MARK: - Word count

    private func tryWordCount(_ message: String) -> String? {
        guard message.hasPrefix("word count") || message.hasPrefix("count words") else {
            return nil
        }
        var text = message
        for prefix in ["word count", "count words"] {
            if text.hasPrefix(prefix) {
                text = String(text.dropFirst(prefix.count))
                break
            }
        }
        text = text
            .trimmingCharacters(in: .whitespaces)
            .trimmingPrefix(":")
            .trimmingCharacters(in: .whitespaces)
        if text.hasPrefix("of ") { text = String(text.dropFirst(3)) }
        if text.hasPrefix("in ") { text = String(text.dropFirst(3)) }
        text = text.trimmingCharacters(in: .whitespaces)

        guard !text.isEmpty else { return nil }
        let count = text.split(separator: " ").count
        return "\(count) words"
    }

    // MARK: - Pure math helpers

    /// Percentage change from `from` to `to`, rounded to 2 decimal places.
    private func percentageChange(from: Double, to: Double) -> Double {
        guard from != 0.0 else { return 0.0 }
        let pct = ((to - from) / from) * 100.0
        return (pct * 100.0).rounded() / 100.0
    }
}

// MARK: - String helpers

private extension String {
    /// `trimmingPrefix` back-port: drop a leading substring if present.
    func trimmingPrefix(_ prefix: String) -> String {
        hasPrefix(prefix) ? String(dropFirst(prefix.count)) : self
    }
}
