// SessionManager.swift
// NanoClawKit
//
// Lightweight session registry for tracking multi-turn conversation state
// on the device edge tier. Mirrors the intent of the Rust EdgeRuntime's
// SessionContext but with richer per-session metadata.

import Foundation

// MARK: - SessionManager

/// An actor that owns the collection of all active `SessionState` instances.
///
/// Sessions are created lazily on first access and can be persisted to disk
/// using JSON encoding for crash/restart recovery.
///
/// ```swift
/// let manager = SessionManager()
/// var session = await manager.getOrCreate(sessionId: "abc-123")
/// await manager.update(sessionId: "abc-123", domain: "music", turnCount: 1)
/// try await manager.save(to: storageURL)
/// ```
public actor SessionManager {

    // MARK: - SessionState

    /// Per-session state. `messages` records the full conversation history in
    /// `[role: content]` pairs — keep the array bounded for production use.
    public struct SessionState: Sendable {
        public let sessionId: String
        public var currentDomain: String
        public var turnCount: UInt32
        public var messages: [Message]
        public let createdAt: Date
        public var lastActiveAt: Date

        /// A single conversation turn.
        public struct Message: Codable, Sendable {
            public let role: String    // "user" | "assistant"
            public let content: String

            public init(role: String, content: String) {
                self.role    = role
                self.content = content
            }
        }

        public init(
            sessionId: String,
            currentDomain: String = "",
            turnCount: UInt32 = 0,
            messages: [Message] = [],
            createdAt: Date = Date(),
            lastActiveAt: Date = Date()
        ) {
            self.sessionId      = sessionId
            self.currentDomain  = currentDomain
            self.turnCount      = turnCount
            self.messages       = messages
            self.createdAt      = createdAt
            self.lastActiveAt   = lastActiveAt
        }
    }

    // MARK: - Storage

    private var sessions: [String: SessionState] = [:]

    public init() {}

    // MARK: - Public API

    /// Return the existing session for `sessionId`, or create a new one.
    public func getOrCreate(sessionId: String) -> SessionState {
        if let existing = sessions[sessionId] {
            return existing
        }
        let new = SessionState(sessionId: sessionId)
        sessions[sessionId] = new
        return new
    }

    /// Update the domain and turn count for `sessionId`.
    ///
    /// No-op if the session does not exist.
    public func update(sessionId: String, domain: String, turnCount: UInt32) {
        guard var state = sessions[sessionId] else { return }
        state.currentDomain = domain
        state.turnCount      = turnCount
        state.lastActiveAt   = Date()
        sessions[sessionId]  = state
    }

    /// Append a message to `sessionId`'s history and bump the last-active timestamp.
    public func appendMessage(
        sessionId: String,
        role: String,
        content: String
    ) {
        var state = getOrCreate(sessionId: sessionId)
        state.messages.append(.init(role: role, content: content))
        state.lastActiveAt = Date()
        if role == "user" {
            state.turnCount += 1
        }
        sessions[sessionId] = state
    }

    /// All active session IDs.
    public var sessionIds: [String] { Array(sessions.keys) }

    /// Remove a session entirely.
    @discardableResult
    public func remove(sessionId: String) -> SessionState? {
        sessions.removeValue(forKey: sessionId)
    }

    // MARK: - Persistence (Codable bridge)

    // SessionState is not directly Codable because we want a clean
    // JSON layout. Use a private CodableState shim instead.

    private struct CodableState: Codable {
        let sessionId: String
        let currentDomain: String
        let turnCount: UInt32
        let messages: [SessionState.Message]
        let createdAt: Date
        let lastActiveAt: Date

        init(from state: SessionState) {
            sessionId     = state.sessionId
            currentDomain = state.currentDomain
            turnCount     = state.turnCount
            messages      = state.messages
            createdAt     = state.createdAt
            lastActiveAt  = state.lastActiveAt
        }

        func toState() -> SessionState {
            SessionState(
                sessionId:     sessionId,
                currentDomain: currentDomain,
                turnCount:     turnCount,
                messages:      messages,
                createdAt:     createdAt,
                lastActiveAt:  lastActiveAt
            )
        }
    }

    /// Serialize all sessions to `url` as JSON.
    public func save(to url: URL) throws {
        let codable = sessions.values.map(CodableState.init)
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = .prettyPrinted
        let data = try encoder.encode(codable)
        try data.write(to: url, options: .atomic)
    }

    /// Deserialize sessions from `url`, merging with any in-memory sessions
    /// (in-memory takes precedence on collision).
    public func load(from url: URL) throws {
        guard FileManager.default.fileExists(atPath: url.path) else { return }
        let data = try Data(contentsOf: url)
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let loaded = try decoder.decode([CodableState].self, from: data)
        for codable in loaded {
            if sessions[codable.sessionId] == nil {
                sessions[codable.sessionId] = codable.toState()
            }
        }
    }
}
