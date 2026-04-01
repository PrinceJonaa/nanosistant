// Models.swift
// NanoClawKit
//
// Mirror of the proto/Rust types used across the NanoClaw edge layer.
// All types are Codable for JSON transport and Sendable for Swift concurrency.

import Foundation

// MARK: - EdgeRequest

/// A request to be processed — either deterministically on-device or forwarded
/// to the RuFlo brain tier via HTTP.
public struct EdgeRequest: Codable, Sendable {
    /// Session identifier scoping the conversation.
    public let sessionId: String
    /// The raw user message text.
    public let userMessage: String
    /// Optional hint for the domain classifier (e.g. "music", "finance").
    public let domainHint: String
    /// Token budget for the response. Passed through to the brain.
    public let maxTokens: UInt32

    public init(
        sessionId: String,
        userMessage: String,
        domainHint: String = "",
        maxTokens: UInt32 = 4096
    ) {
        self.sessionId = sessionId
        self.userMessage = userMessage
        self.domainHint = domainHint
        self.maxTokens = maxTokens
    }

    // MARK: CodingKeys — snake_case ↔ camelCase
    enum CodingKeys: String, CodingKey {
        case sessionId      = "session_id"
        case userMessage    = "user_message"
        case domainHint     = "domain_hint"
        case maxTokens      = "max_tokens"
    }
}

// MARK: - EdgeResponse

/// A response from any tier: deterministic, confidence ladder, RuFlo, or offline queue.
public struct EdgeResponse: Codable, Sendable {
    /// Session identifier echoed back.
    public let sessionId: String
    /// The textual response to present to the user.
    public let responseText: String
    /// Name of the agent or system that produced the response.
    public let respondingAgent: String
    /// Confidence score in [0, 1]. 1.0 for deterministic, 0.0 for offline.
    public let confidence: Double
    /// Which tier resolved the request:
    ///   0 = deterministic, 1–4 = confidence ladder, 6 = ruflo, 99 = offline
    public let resolvedAtTier: UInt8
    /// Budget usage information, if available.
    public let budgetStatus: BudgetStatus?

    public init(
        sessionId: String,
        responseText: String,
        respondingAgent: String,
        confidence: Double,
        resolvedAtTier: UInt8,
        budgetStatus: BudgetStatus?
    ) {
        self.sessionId = sessionId
        self.responseText = responseText
        self.respondingAgent = respondingAgent
        self.confidence = confidence
        self.resolvedAtTier = resolvedAtTier
        self.budgetStatus = budgetStatus
    }

    enum CodingKeys: String, CodingKey {
        case sessionId       = "session_id"
        case responseText    = "response_text"
        case respondingAgent = "responding_agent"
        case confidence
        case resolvedAtTier  = "resolved_at_tier"
        case budgetStatus    = "budget_status"
    }
}

// MARK: - BudgetStatus

/// Token budget accounting for a session.
public struct BudgetStatus: Codable, Sendable {
    public let tokensUsed: UInt32
    public let tokensRemaining: UInt32
    /// Qualitative level: "green" | "amber" | "yellow" | "red"
    public let status: String

    public init(tokensUsed: UInt32, tokensRemaining: UInt32, status: String) {
        self.tokensUsed = tokensUsed
        self.tokensRemaining = tokensRemaining
        self.status = status
    }

    enum CodingKeys: String, CodingKey {
        case tokensUsed      = "tokens_used"
        case tokensRemaining = "tokens_remaining"
        case status
    }
}

// MARK: - RouteSource

/// Describes which resolution path produced an `EdgeResponse`.
public enum RouteSource: String, Codable, Sendable {
    /// Answered by a local deterministic function — zero network.
    case deterministic
    /// Answered by the local confidence ladder before reaching the brain.
    case confidenceLadder = "confidence_ladder"
    /// Forwarded to and answered by the RuFlo brain tier.
    case ruflo
    /// No network; message enqueued for later delivery.
    case offline
}
