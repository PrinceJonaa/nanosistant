// OfflineQueue.swift
// NanoClawKit
//
// Persistent offline queue for messages that cannot reach the brain tier.
// Thread-safe via the actor model; disk persistence uses JSON encoding.

import Foundation

// MARK: - OfflineQueue

/// A bounded FIFO queue that accumulates messages when the brain is unreachable.
///
/// The queue is automatically trimmed to `maxSize` on enqueue (oldest entries
/// are dropped first to make room). Optionally persists to disk using JSON so
/// that queued messages survive app restarts.
///
/// ```swift
/// let queue = OfflineQueue(maxSize: 50, storageURL: cacheURL)
/// try? await queue.load()
/// await queue.enqueue("my message", domainHint: "music")
/// let pending = await queue.drain()
/// ```
public actor OfflineQueue {

    // MARK: - QueuedMessage

    /// A single queued message with its metadata.
    public struct QueuedMessage: Codable, Sendable, Equatable {
        /// The raw user text.
        public let message: String
        /// Domain hint provided at enqueue time.
        public let domainHint: String
        /// Wall-clock time when the message was enqueued.
        public let timestamp: Date

        public init(message: String, domainHint: String, timestamp: Date = Date()) {
            self.message    = message
            self.domainHint = domainHint
            self.timestamp  = timestamp
        }

        enum CodingKeys: String, CodingKey {
            case message
            case domainHint = "domain_hint"
            case timestamp
        }
    }

    // MARK: - State

    private var queue: [QueuedMessage] = []
    private let maxSize: Int
    private let storageURL: URL?

    // MARK: - Init

    /// Create an offline queue.
    ///
    /// - Parameters:
    ///   - maxSize: Maximum number of messages to hold. When the queue is full,
    ///     the oldest entry is removed to make room for the new one. Default 100.
    ///   - storageURL: Optional file URL for JSON persistence. Pass `nil` for
    ///     in-memory-only operation.
    public init(maxSize: Int = 100, storageURL: URL? = nil) {
        self.maxSize    = max(1, maxSize)
        self.storageURL = storageURL
    }

    // MARK: - Public API

    /// Add `message` to the back of the queue.
    ///
    /// If the queue is already at `maxSize`, the oldest message is evicted.
    public func enqueue(_ message: String, domainHint: String = "") {
        if queue.count >= maxSize {
            queue.removeFirst()   // evict oldest
        }
        queue.append(QueuedMessage(message: message, domainHint: domainHint))
    }

    /// Remove and return all queued messages in FIFO order.
    ///
    /// The queue is empty after this call.
    @discardableResult
    public func drain() -> [QueuedMessage] {
        let drained = queue
        queue.removeAll()
        return drained
    }

    /// Peek at all queued messages without removing them.
    public func peek() -> [QueuedMessage] { queue }

    /// Number of messages currently in the queue.
    public var count: Int { queue.count }

    /// `true` when the queue holds no messages.
    public var isEmpty: Bool { queue.isEmpty }

    // MARK: - Persistence

    /// Serialize the queue to disk at `storageURL`.
    ///
    /// - Throws: `CocoaError` / `EncodingError` if the write fails.
    public func save() throws {
        guard let url = storageURL else { return }
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        let data = try encoder.encode(queue)
        try data.write(to: url, options: .atomic)
    }

    /// Deserialize the queue from disk at `storageURL`.
    ///
    /// Missing files are silently ignored (treated as an empty queue).
    /// - Throws: `DecodingError` if the file exists but cannot be decoded.
    public func load() throws {
        guard let url = storageURL else { return }
        guard FileManager.default.fileExists(atPath: url.path) else { return }

        let data = try Data(contentsOf: url)
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let loaded = try decoder.decode([QueuedMessage].self, from: data)

        // Merge, keeping up to maxSize entries (prefer newer)
        let combined = queue + loaded
        queue = Array(combined.suffix(maxSize))
    }
}
