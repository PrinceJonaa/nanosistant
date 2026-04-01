// DeterministicResolverTests.swift
// NanoClawKitTests
//
// Verifies that DeterministicResolver produces results consistent with the
// Rust deterministic.rs unit tests.

import XCTest
@testable import NanoClawKit

final class DeterministicResolverTests: XCTestCase {

    var resolver: DeterministicResolver!

    override func setUp() {
        super.setUp()
        resolver = DeterministicResolver()
    }

    // MARK: - BPM calculations

    func testBPMBarDuration120() {
        let result = resolver.resolve("120 bpm bar duration")
        XCTAssertNotNil(result, "120 bpm bar duration should resolve")
        XCTAssertTrue(result!.contains("2.000"), "120 BPM bar duration should be 2.000s — got: \(result!)")
    }

    func testBPMBarDuration140() {
        let result = resolver.resolve("140 bpm bar duration")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("1.714"), "140 BPM bar duration should be 1.714s — got: \(result!)")
    }

    func testBPMBarCount120_3min() {
        let result = resolver.resolve("120 bpm 3 min song")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("90"), "120 BPM / 180 s should be 90 bars — got: \(result!)")
    }

    func testBPMBarCount140_3min() {
        let result = resolver.resolve("140 bpm 3 min song")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("105"), "140 BPM / 180 s should be 105 bars — got: \(result!)")
    }

    // MARK: - Scale lookups

    func testCMajorScale() {
        let result = resolver.resolve("c major scale")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("C - D - E - F - G - A - B"),
                      "C major scale mismatch: \(result!)")
    }

    func testAMinorScale() {
        let result = resolver.resolve("a minor scale")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("A - B - C - D - E - F - G"),
                      "A minor scale mismatch: \(result!)")
    }

    func testDDorianScale() {
        let result = resolver.resolve("d dorian scale")
        XCTAssertNotNil(result)
        // D dorian: D E F G A B C
        XCTAssertTrue(result!.contains("D - E - F - G - A - B - C"),
                      "D dorian scale mismatch: \(result!)")
    }

    // MARK: - Chord-in-key (Roman numeral)

    func testAmInCMajor() {
        let result = resolver.resolve("Am in C major")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("vi"), "Am in C major should equal vi — got: \(result!)")
    }

    func testGInCMajor() {
        let result = resolver.resolve("G in C major")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("V"), "G in C major should equal V — got: \(result!)")
    }

    func testFInCMajor() {
        let result = resolver.resolve("F in C major")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("IV"), "F in C major should equal IV — got: \(result!)")
    }

    func testCInCMajor() {
        let result = resolver.resolve("C in C major")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("I"), "C in C major should equal I — got: \(result!)")
    }

    // MARK: - Frequency band lookup

    func testSubBass() {
        let result = resolver.resolve("40hz band")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("Sub Bass"), "40 Hz should be Sub Bass — got: \(result!)")
    }

    func testBass() {
        let result = resolver.resolve("200hz band")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("Bass"), "200 Hz should be Bass — got: \(result!)")
    }

    func testMids() {
        let result = resolver.resolve("1000hz band")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("Mids"), "1000 Hz should be Mids — got: \(result!)")
    }

    func testUpperMids() {
        let result = resolver.resolve("3000hz band")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("Upper Mids"), "3000 Hz should be Upper Mids — got: \(result!)")
    }

    func testBrilliance() {
        let result = resolver.resolve("10000hz band")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("Brilliance"), "10000 Hz should be Brilliance — got: \(result!)")
    }

    func testKhzUnit() {
        let result = resolver.resolve("2.5khz band")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("Upper Mids"), "2.5 kHz should be Upper Mids — got: \(result!)")
    }

    // MARK: - Percentage change

    func testPercentageChangePositive() {
        let result = resolver.resolve("percentage change from 100 to 150")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("+50.00"), "100→150 should be +50% — got: \(result!)")
    }

    func testPercentageChangeNegative() {
        let result = resolver.resolve("percentage change from 200 to 100")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.contains("-50.00"), "200→100 should be -50% — got: \(result!)")
    }

    // MARK: - Non-deterministic (must return nil)

    func testOpenCreativeReturnsNil() {
        XCTAssertNil(resolver.resolve("help me write a verse about love"))
    }

    func testJudgmentQueryReturnsNil() {
        XCTAssertNil(resolver.resolve("what's the best investment right now"))
    }

    func testExplainModalInterchangeReturnsNil() {
        XCTAssertNil(resolver.resolve("explain modal interchange"))
    }

    func testEmptyStringReturnsNil() {
        XCTAssertNil(resolver.resolve(""))
    }
}

// MARK: - MusicTheory unit tests

final class MusicTheoryTests: XCTestCase {

    // MARK: - BPM

    func testBpmToBarDuration_120bpm() {
        let dur = MusicTheory.bpmToBarDuration(bpm: 120, beatsPerBar: 4)
        XCTAssertEqual(dur, 2.0, accuracy: 0.001)
    }

    func testBpmToBarDuration_140bpm() {
        let dur = MusicTheory.bpmToBarDuration(bpm: 140, beatsPerBar: 4)
        XCTAssertEqual(dur, 1.714, accuracy: 0.001)
    }

    func testBpmToBarDuration_zeroBpm() {
        XCTAssertEqual(MusicTheory.bpmToBarDuration(bpm: 0), 0.0)
    }

    func testSongBarCount_120bpm_180s() {
        XCTAssertEqual(MusicTheory.songBarCount(bpm: 120, targetDurationSecs: 180.0), 90)
    }

    func testSongBarCount_140bpm_180s() {
        XCTAssertEqual(MusicTheory.songBarCount(bpm: 140, targetDurationSecs: 180.0), 105)
    }

    func testSongBarCount_zeroBpm() {
        XCTAssertEqual(MusicTheory.songBarCount(bpm: 0, targetDurationSecs: 180.0), 0)
    }

    // MARK: - Scale degrees

    func testScaleDegrees_CMajor() {
        let scale = MusicTheory.scaleDegrees(key: "C", mode: "major")
        XCTAssertEqual(scale, ["C", "D", "E", "F", "G", "A", "B"])
    }

    func testScaleDegrees_AMinor() {
        let scale = MusicTheory.scaleDegrees(key: "A", mode: "minor")
        XCTAssertEqual(scale, ["A", "B", "C", "D", "E", "F", "G"])
    }

    func testScaleDegrees_GMajor() {
        let scale = MusicTheory.scaleDegrees(key: "G", mode: "major")
        XCTAssertEqual(scale, ["G", "A", "B", "C", "D", "E", "F#"])
    }

    func testScaleDegrees_DDorian() {
        let scale = MusicTheory.scaleDegrees(key: "D", mode: "dorian")
        XCTAssertEqual(scale, ["D", "E", "F", "G", "A", "B", "C"])
    }

    func testScaleDegrees_FSharpMajor() {
        let scale = MusicTheory.scaleDegrees(key: "F#", mode: "major")
        XCTAssertEqual(scale.count, 7)
        XCTAssertEqual(scale.first, "F#")
    }

    // MARK: - Chord to Roman

    func testChordToRoman_C_in_C() {
        XCTAssertEqual(MusicTheory.chordToRoman(chord: "C", key: "C"), "I")
    }

    func testChordToRoman_Am_in_C() {
        XCTAssertEqual(MusicTheory.chordToRoman(chord: "Am", key: "C"), "vi")
    }

    func testChordToRoman_G_in_C() {
        XCTAssertEqual(MusicTheory.chordToRoman(chord: "G", key: "C"), "V")
    }

    func testChordToRoman_F_in_C() {
        XCTAssertEqual(MusicTheory.chordToRoman(chord: "F", key: "C"), "IV")
    }

    func testChordToRoman_notInKey() {
        let result = MusicTheory.chordToRoman(chord: "C#", key: "C")
        XCTAssertTrue(result.hasPrefix("?"), "Out-of-key chord should return ?")
    }

    // MARK: - Roman to Chord

    func testRomanToChord_I_in_C() {
        XCTAssertEqual(MusicTheory.romanToChord(roman: "I", key: "C"), "C")
    }

    func testRomanToChord_vi_in_C() {
        XCTAssertEqual(MusicTheory.romanToChord(roman: "vi", key: "C"), "Am")
    }

    func testRomanToChord_V_in_C() {
        XCTAssertEqual(MusicTheory.romanToChord(roman: "V", key: "C"), "G")
    }

    // MARK: - Transpose

    func testTranspose_CMajorUp2() {
        let notes = ["C", "E", "G"]
        let transposed = MusicTheory.transpose(notes: notes, semitones: 2)
        XCTAssertEqual(transposed, ["D", "F#", "A"])
    }

    func testTranspose_wrapAround() {
        // B + 1 semitone = C
        XCTAssertEqual(MusicTheory.transpose(notes: ["B"], semitones: 1), ["C"])
    }

    func testTranspose_negativeInterval() {
        // C - 1 semitone = B
        XCTAssertEqual(MusicTheory.transpose(notes: ["C"], semitones: -1), ["B"])
    }

    // MARK: - Note to frequency

    func testA4_is_440Hz() {
        let freq = MusicTheory.noteToFrequency(note: "A", octave: 4)
        XCTAssertEqual(freq, 440.0, accuracy: 0.01)
    }

    func testC4_is_approx261Hz() {
        let freq = MusicTheory.noteToFrequency(note: "C", octave: 4)
        XCTAssertEqual(freq, 261.63, accuracy: 0.1)
    }

    func testA5_is_880Hz() {
        let freq = MusicTheory.noteToFrequency(note: "A", octave: 5)
        XCTAssertEqual(freq, 880.0, accuracy: 0.01)
    }

    // MARK: - Frequency to band

    func testFrequencyToBand_subBass() {
        XCTAssertEqual(MusicTheory.frequencyToBand(hz: 40.0), "Sub Bass")
    }

    func testFrequencyToBand_bass() {
        XCTAssertEqual(MusicTheory.frequencyToBand(hz: 200.0), "Bass")
    }

    func testFrequencyToBand_lowMids() {
        XCTAssertEqual(MusicTheory.frequencyToBand(hz: 400.0), "Low Mids")
    }

    func testFrequencyToBand_mids() {
        XCTAssertEqual(MusicTheory.frequencyToBand(hz: 1000.0), "Mids")
    }

    func testFrequencyToBand_upperMids() {
        XCTAssertEqual(MusicTheory.frequencyToBand(hz: 3000.0), "Upper Mids")
    }

    func testFrequencyToBand_presence() {
        XCTAssertEqual(MusicTheory.frequencyToBand(hz: 5000.0), "Presence")
    }

    func testFrequencyToBand_brilliance() {
        XCTAssertEqual(MusicTheory.frequencyToBand(hz: 10000.0), "Brilliance")
    }

    func testFrequencyToBand_air() {
        XCTAssertEqual(MusicTheory.frequencyToBand(hz: 16000.0), "Air")
    }

    // MARK: - Syllable count

    func testSyllableCount_hello() {
        XCTAssertEqual(MusicTheory.syllableCount(text: "hello"), 2)
    }

    func testSyllableCount_beautiful() {
        XCTAssertEqual(MusicTheory.syllableCount(text: "beautiful"), 3)
    }

    func testSyllableCount_sentence() {
        // "I am the greatest" → I(1) am(1) the(1) great-est(2) = 5
        XCTAssertEqual(MusicTheory.syllableCount(text: "I am the greatest"), 5)
    }

    // MARK: - Note normalisation

    func testNormalizeNote_flat() {
        XCTAssertEqual(MusicTheory.normalizeNote("Bb"), "A#")
        XCTAssertEqual(MusicTheory.normalizeNote("Eb"), "D#")
        XCTAssertEqual(MusicTheory.normalizeNote("Db"), "C#")
    }

    func testNormalizeNote_capitalisation() {
        XCTAssertEqual(MusicTheory.normalizeNote("c"), "C")
        XCTAssertEqual(MusicTheory.normalizeNote("f#"), "F#")
    }
}
