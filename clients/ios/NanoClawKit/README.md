# NanoClawKit

Swift Package for the [Nanosistant](../../README.md) iOS edge client. Mirrors the Rust `nanoclaw` crate: local-first deterministic resolution, HTTP transport to the RuFlo brain server, and an offline queue for when the server is unreachable.

Targets iOS 17+ and macOS 14+. Written in Swift 6.2 with strict concurrency (actors, `Sendable`).

---

## Architecture

```
User message
     │
     ▼
DeterministicResolver   ← zero network, zero cost
     │ nil (needs LLM)
     ▼
BrainClient             ← HTTP POST /api/message → axum server → RuFlo
     │ error / unreachable
     ▼
OfflineQueue            ← persisted to disk, flushed on reconnect
```

All three tiers are composed inside `NanoClawClient`, the single entry point your app calls.

---

## Adding the package

In Xcode 26, **File → Add Package Dependencies** and enter:

```
https://github.com/your-org/nanosistant
```

Or add it to your own `Package.swift`:

```swift
.package(url: "https://github.com/your-org/nanosistant", from: "0.3.0"),
```

Then add `NanoClawKit` to your target's dependencies:

```swift
.target(name: "MyApp", dependencies: [
    .product(name: "NanoClawKit", package: "nanosistant"),
]),
```

---

## Quick start

```swift
import NanoClawKit

// Create once per app lifetime (or per user session).
let client = NanoClawClient(
    brainURL: URL(string: "http://192.168.1.10:3000")!,
    sessionId: "user-abc-123"           // omit for a fresh UUID
)

// Process a message — never throws, always returns a response.
let response = await client.processMessage("c major scale", domainHint: "music")
print(response.responseText)
// "C major scale: C - D - E - F - G - A - B"
print(response.respondingAgent)
// "deterministic"  (answered locally, zero network)
```

### Handling the response

```swift
switch response.respondingAgent {
case "deterministic":
    // Answered locally — instant, no cost.
case "offline":
    // Queued; flush when connectivity returns.
    let pending = await client.offlineQueueCount
    print("\(pending) messages queued")
default:
    // Answered by the RuFlo brain tier.
    if let budget = response.budgetStatus {
        print("Tokens remaining: \(budget.tokensRemaining) (\(budget.status))")
    }
}
```

### Flushing the offline queue

Call `flushOfflineQueue()` when you detect the network has been restored (e.g., via `NWPathMonitor`):

```swift
let results = await client.flushOfflineQueue()
for result in results {
    if result.respondingAgent != "offline" {
        // Message was successfully processed.
    }
}
```

---

## Using the music-theory helpers directly

All functions in `MusicTheory` are static and match the Rust `deterministic.rs` implementations exactly:

```swift
// Bar duration
let dur = MusicTheory.bpmToBarDuration(bpm: 120) // → 2.0

// Scale degrees
let scale = MusicTheory.scaleDegrees(key: "G", mode: "major")
// → ["G", "A", "B", "C", "D", "E", "F#"]

// Roman numerals
MusicTheory.chordToRoman(chord: "Am", key: "C") // → "vi"
MusicTheory.romanToChord(roman: "V", key: "G")  // → "D"

// Transposition
MusicTheory.transpose(notes: ["C", "E", "G"], semitones: 2) // → ["D", "F#", "A"]

// Frequency
MusicTheory.noteToFrequency(note: "A", octave: 4) // → 440.0
MusicTheory.frequencyToBand(hz: 1000.0)           // → "Mids"

// Syllables
MusicTheory.syllableCount(text: "I am the greatest") // → 5
```

---

## Running the tests

```bash
cd clients/ios/NanoClawKit
swift test
```

All tests are offline (the `NanoClawClientTests` suite uses a `MockURLProtocol` to simulate the server).

---

## Offline persistence

Pass a `storageURL` to persist the offline queue across app restarts:

```swift
let cacheDir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
let queueURL = cacheDir.appendingPathComponent("nanoclaw_offline_queue.json")

let client = NanoClawClient(
    brainURL: URL(string: "http://localhost:3000")!,
    offlineStorageURL: queueURL
)
```

The queue is written atomically (`.atomic` write option) so a crash during save does not corrupt the file.

---

## Session management

`SessionManager` tracks conversation history and domain per session. Access it through the client:

```swift
let session = await client.currentSession
print("Turn \(session.turnCount) in domain '\(session.currentDomain)'")
```

Session state can be saved/loaded from disk for crash recovery in your app layer.

---

## v0.3.0 scope and roadmap

| Feature | Status |
|---|---|
| Deterministic resolver (music, BPM, frequency, %, word count) | ✅ |
| HTTP/JSON transport to axum server | ✅ |
| Offline queue with disk persistence | ✅ |
| Session state management | ✅ |
| Swift concurrency (actors, `Sendable`) | ✅ |
| gRPC transport (replacing HTTP) | Planned v0.4 |
| Proto-generated Swift types | Planned v0.4 |
| SwiftUI demo app | Planned v0.5 |
