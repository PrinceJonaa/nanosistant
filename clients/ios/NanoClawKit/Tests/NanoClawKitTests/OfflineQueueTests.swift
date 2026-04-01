// OfflineQueueTests.swift
// NanoClawKitTests
//
// Tests for OfflineQueue — enqueue, drain, max-size eviction, and disk persistence.

import XCTest
@testable import NanoClawKit

final class OfflineQueueTests: XCTestCase {

    // MARK: - Enqueue and drain

    func testEnqueueAndDrain() async {
        let queue = OfflineQueue()
        await queue.enqueue("hello", domainHint: "music")
        await queue.enqueue("world", domainHint: "general")

        let count = await queue.count
        XCTAssertEqual(count, 2)

        let messages = await queue.drain()
        XCTAssertEqual(messages.count, 2)
        XCTAssertEqual(messages[0].message, "hello")
        XCTAssertEqual(messages[1].message, "world")
        // Queue should be empty after drain
        let countAfter = await queue.count
        XCTAssertEqual(countAfter, 0)
    }

    func testDrainOrder_FIFO() async {
        let queue = OfflineQueue()
        for i in 0..<5 {
            await queue.enqueue("msg\(i)", domainHint: "")
        }
        let messages = await queue.drain()
        XCTAssertEqual(messages.map(\.message), ["msg0", "msg1", "msg2", "msg3", "msg4"])
    }

    // MARK: - Max size enforcement

    func testMaxSizeEvictsOldest() async {
        let queue = OfflineQueue(maxSize: 3)
        await queue.enqueue("first",  domainHint: "")
        await queue.enqueue("second", domainHint: "")
        await queue.enqueue("third",  domainHint: "")
        // This enqueue should evict "first"
        await queue.enqueue("fourth", domainHint: "")

        let count = await queue.count
        XCTAssertEqual(count, 3, "Queue should not exceed maxSize")

        let messages = await queue.drain()
        XCTAssertEqual(messages[0].message, "second", "Oldest entry should have been evicted")
        XCTAssertEqual(messages.last?.message, "fourth")
    }

    func testMaxSizeOne() async {
        let queue = OfflineQueue(maxSize: 1)
        await queue.enqueue("a", domainHint: "")
        await queue.enqueue("b", domainHint: "")
        let messages = await queue.drain()
        XCTAssertEqual(messages.count, 1)
        XCTAssertEqual(messages[0].message, "b")
    }

    // MARK: - Empty queue

    func testDrainEmptyReturnsEmptyArray() async {
        let queue = OfflineQueue()
        let messages = await queue.drain()
        XCTAssertTrue(messages.isEmpty)
    }

    func testIsEmpty() async {
        let queue = OfflineQueue()
        let isEmpty = await queue.isEmpty
        XCTAssertTrue(isEmpty)

        await queue.enqueue("x", domainHint: "")
        let isNotEmpty = await queue.isEmpty
        XCTAssertFalse(isNotEmpty)
    }

    // MARK: - Domain hint

    func testDomainHintPreserved() async {
        let queue = OfflineQueue()
        await queue.enqueue("test", domainHint: "finance")
        let messages = await queue.drain()
        XCTAssertEqual(messages[0].domainHint, "finance")
    }

    // MARK: - Disk persistence

    func testSaveAndLoad() async throws {
        let tmpURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("nanoclaw_test_queue_\(UUID().uuidString).json")
        defer { try? FileManager.default.removeItem(at: tmpURL) }

        // Populate and save
        let writeQueue = OfflineQueue(storageURL: tmpURL)
        await writeQueue.enqueue("msg A", domainHint: "music")
        await writeQueue.enqueue("msg B", domainHint: "general")
        try await writeQueue.save()

        // Load into a fresh queue
        let readQueue = OfflineQueue(storageURL: tmpURL)
        try await readQueue.load()

        let messages = await readQueue.drain()
        XCTAssertEqual(messages.count, 2)
        XCTAssertEqual(messages[0].message, "msg A")
        XCTAssertEqual(messages[1].message, "msg B")
    }

    func testLoadFromMissingFileDoesNotThrow() async {
        let missingURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("does_not_exist_\(UUID().uuidString).json")
        let queue = OfflineQueue(storageURL: missingURL)
        XCTAssertNoThrow(try Task { try await queue.load() }.result.get())
    }

    func testSaveWithNilStorageURLDoesNothing() async throws {
        let queue = OfflineQueue(storageURL: nil)
        await queue.enqueue("msg", domainHint: "")
        // Should not throw — no-op when storageURL is nil
        try await queue.save()
    }

    // MARK: - Timestamp

    func testTimestampIsSetOnEnqueue() async {
        let before = Date()
        let queue = OfflineQueue()
        await queue.enqueue("ts-test", domainHint: "")
        let after = Date()

        let messages = await queue.drain()
        XCTAssertEqual(messages.count, 1)
        let ts = messages[0].timestamp
        XCTAssertGreaterThanOrEqual(ts, before)
        XCTAssertLessThanOrEqual(ts, after)
    }
}
