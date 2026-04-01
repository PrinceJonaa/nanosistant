// NanoClawClient.swift
// NanoClawKit
//
// Main entry point for all user messages on the iOS edge tier.
// Mirrors the Rust EdgeRuntime processing pipeline:
//   1. Try local deterministic resolution (zero network, zero cost).
//   2. Forward to RuFlo brain via HTTP if reachable.
//   3. Enqueue offline if the brain is unavailable.

import Foundation

// MARK: - NanoClawClient

/// The primary client object for embedding Nanosistant in an iOS app.
///
/// Create one instance per app lifetime (or per user session if you need
/// strict isolation). All methods are `async` and actor-isolated.
///
/// ```swift
/// let client = NanoClawClient(brainURL: URL(string: "http://localhost:3000")!)
///
/// let response = await client.processMessage("c major scale", domainHint: "music")
/// print(response.responseText)   // "C major scale: C - D - E - F - G - A - B"
/// ```
public actor NanoClawClient {

    // MARK: - Sub-components

    private let resolver: DeterministicResolver
    private let brain: BrainClient
    private let offlineQueue: OfflineQueue
    private let sessionManager: SessionManager
    private let sessionId: String

    // MARK: - Init

    /// Create a `NanoClawClient`.
    ///
    /// - Parameters:
    ///   - brainURL: HTTP base URL of the axum server.
    ///   - sessionId: Identifier for this conversation session. Defaults to
    ///     a fresh UUID. Provide a stable ID to resume a previous session.
    ///   - maxOfflineQueueSize: Maximum number of messages to hold while
    ///     offline before the oldest is evicted. Default 100.
    ///   - offlineStorageURL: If provided, the offline queue is persisted to
    ///     this file URL so enqueued messages survive app restarts.
    public init(
        brainURL: URL = URL(string: "http://localhost:3000")!,
        sessionId: String = UUID().uuidString,
        maxOfflineQueueSize: Int = 100,
        offlineStorageURL: URL? = nil
    ) {
        self.resolver       = DeterministicResolver()
        self.brain          = BrainClient(baseURL: brainURL)
        self.offlineQueue   = OfflineQueue(maxSize: maxOfflineQueueSize, storageURL: offlineStorageURL)
        self.sessionManager = SessionManager()
        self.sessionId      = sessionId
    }

    /// Package-internal init that accepts a custom `URLSessionConfiguration`.
    /// Used in tests to inject a mock `URLProtocol` without touching the
    /// shared default session.
    init(
        brainURL: URL,
        sessionId: String,
        urlSessionConfiguration: URLSessionConfiguration,
        maxOfflineQueueSize: Int = 100,
        offlineStorageURL: URL? = nil
    ) {
        self.resolver       = DeterministicResolver()
        self.brain          = BrainClient(
            baseURL: brainURL,
            urlSessionConfiguration: urlSessionConfiguration
        )
        self.offlineQueue   = OfflineQueue(maxSize: maxOfflineQueueSize, storageURL: offlineStorageURL)
        self.sessionManager = SessionManager()
        self.sessionId      = sessionId
    }

    // MARK: - Core message processing

    /// Process `message`, returning a response from the best available tier.
    ///
    /// Resolution order:
    /// 1. **Deterministic** — answered locally with zero network.
    /// 2. **Brain (HTTP)** — forwarded to the RuFlo server.
    /// 3. **Offline queue** — enqueued for later delivery if the brain is
    ///    unreachable or returns an error.
    ///
    /// This method never throws; a queued or error response is always returned.
    public func processMessage(
        _ message: String,
        domainHint: String = ""
    ) async -> EdgeResponse {
        // ── Step 1: Deterministic local resolution ────────────────────────
        if let resolved = resolver.resolve(message) {
            await sessionManager.appendMessage(
                sessionId: sessionId, role: "user", content: message
            )
            await sessionManager.appendMessage(
                sessionId: sessionId, role: "assistant", content: resolved
            )
            return EdgeResponse(
                sessionId:       sessionId,
                responseText:    resolved,
                respondingAgent: "deterministic",
                confidence:      1.0,
                resolvedAtTier:  0,
                budgetStatus:    nil
            )
        }

        // ── Step 2: Forward to brain ──────────────────────────────────────
        let request = EdgeRequest(
            sessionId:   sessionId,
            userMessage: message,
            domainHint:  domainHint
        )

        do {
            let response = try await brain.send(request)
            await sessionManager.appendMessage(
                sessionId: sessionId, role: "user", content: message
            )
            await sessionManager.appendMessage(
                sessionId: sessionId, role: "assistant", content: response.responseText
            )
            return response
        } catch {
            // ── Step 3: Offline queue ──────────────────────────────────────
            await offlineQueue.enqueue(message, domainHint: domainHint)
            return EdgeResponse(
                sessionId:       sessionId,
                responseText:    "Queued for processing when online.",
                respondingAgent: "offline",
                confidence:      0.0,
                resolvedAtTier:  99,
                budgetStatus:    nil
            )
        }
    }

    // MARK: - Offline queue management

    /// Flush the offline queue by re-sending each message to the brain.
    ///
    /// Call this when you detect connectivity has been restored.
    /// Messages that still fail are silently dropped (the caller can inspect
    /// the returned array to detect gaps — successful responses have
    /// `respondingAgent != "offline"`).
    ///
    /// - Returns: One `EdgeResponse` per queued message, in FIFO order.
    public func flushOfflineQueue() async -> [EdgeResponse] {
        let pending = await offlineQueue.drain()
        guard !pending.isEmpty else { return [] }

        var results: [EdgeResponse] = []
        results.reserveCapacity(pending.count)

        for queued in pending {
            let request = EdgeRequest(
                sessionId:   sessionId,
                userMessage: queued.message,
                domainHint:  queued.domainHint
            )
            do {
                let response = try await brain.send(request)
                results.append(response)
            } catch {
                // Re-enqueue on failure so no messages are lost.
                await offlineQueue.enqueue(queued.message, domainHint: queued.domainHint)
                results.append(EdgeResponse(
                    sessionId:       sessionId,
                    responseText:    "Still offline — message re-queued.",
                    respondingAgent: "offline",
                    confidence:      0.0,
                    resolvedAtTier:  99,
                    budgetStatus:    nil
                ))
            }
        }
        return results
    }

    /// Number of messages currently waiting in the offline queue.
    public var offlineQueueCount: Int {
        get async { await offlineQueue.count }
    }

    /// `true` when the offline queue has no pending messages.
    public var isOfflineQueueEmpty: Bool {
        get async { await offlineQueue.isEmpty }
    }

    // MARK: - Connectivity check

    /// Returns `true` if the brain server responded to a `/health` check.
    public func isBrainReachable() async -> Bool {
        await brain.isReachable()
    }

    // MARK: - Session access

    /// Current session state (domain, turn count, history).
    public var currentSession: SessionManager.SessionState {
        get async {
            await sessionManager.getOrCreate(sessionId: sessionId)
        }
    }

    /// The session identifier for this client instance.
    public nonisolated var currentSessionId: String { sessionId }
}
