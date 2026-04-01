// NanoClawClientTests.swift
// NanoClawKitTests
//
// Integration-level tests for NanoClawClient.
//
// All network-touching tests use MockURLProtocol injected via the internal
// `urlSessionConfiguration:` init so they run fully offline in CI.

import XCTest
@testable import NanoClawKit

// MARK: - Mock URLProtocol

/// Intercepts URLSession requests in tests so we don't need a live server.
final class MockURLProtocol: URLProtocol {

    /// Set before each test to customise the mock response.
    /// `nonisolated(unsafe)` is safe here because tests are single-threaded.
    nonisolated(unsafe) static var requestHandler: ((URLRequest) throws -> (HTTPURLResponse, Data))?

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        guard let handler = MockURLProtocol.requestHandler else {
            client?.urlProtocol(self, didFailWithError: URLError(.badServerResponse))
            return
        }
        do {
            let (response, data) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

// MARK: - Helpers

/// Build a `NanoClawClient` whose URLSession is backed by `MockURLProtocol`.
private func makeMockClient(
    sessionId: String = UUID().uuidString,
    handler: @escaping (URLRequest) throws -> (HTTPURLResponse, Data)
) -> NanoClawClient {
    MockURLProtocol.requestHandler = handler
    let config = URLSessionConfiguration.ephemeral
    config.protocolClasses = [MockURLProtocol.self]
    return NanoClawClient(
        brainURL: URL(string: "http://mock.local")!,
        sessionId: sessionId,
        urlSessionConfiguration: config
    )
}

/// Convenience: client that always throws a network error.
private func makeOfflineClient(sessionId: String = UUID().uuidString) -> NanoClawClient {
    makeMockClient(sessionId: sessionId) { _ in
        throw URLError(.notConnectedToInternet)
    }
}

/// Build a valid JSON-encoded EdgeResponse payload.
private func makeResponseData(
    sessionId: String,
    responseText: String,
    respondingAgent: String = "ruflo",
    confidence: Double = 0.9,
    resolvedAtTier: UInt8 = 6
) throws -> Data {
    let json: [String: Any] = [
        "session_id":       sessionId,
        "response_text":    responseText,
        "responding_agent": respondingAgent,
        "confidence":       confidence,
        "resolved_at_tier": Int(resolvedAtTier),
        "budget_status": [
            "tokens_used":      100,
            "tokens_remaining": 3996,
            "status":           "green"
        ] as [String: Any]
    ]
    return try JSONSerialization.data(withJSONObject: json)
}

// MARK: - NanoClawClientTests

final class NanoClawClientTests: XCTestCase {

    // MARK: - Deterministic resolution

    func testDeterministicMessageResolvesLocally() async {
        // "c major scale" is handled entirely by DeterministicResolver — no
        // network call should ever be made.
        let client = makeMockClient(sessionId: "test-det") { _ in
            XCTFail("Brain should NOT be called for a deterministic message")
            throw URLError(.notConnectedToInternet)
        }
        let response = await client.processMessage("c major scale", domainHint: "music")
        XCTAssertEqual(response.respondingAgent, "deterministic")
        XCTAssertEqual(response.resolvedAtTier, 0)
        XCTAssertEqual(response.confidence, 1.0)
        XCTAssertTrue(response.responseText.contains("C - D - E - F - G - A - B"),
                      "Unexpected scale response: \(response.responseText)")
    }

    func testBPMResolvesLocally() async {
        let client = makeMockClient(sessionId: "test-bpm") { _ in
            XCTFail("Brain should NOT be called for BPM calculation")
            throw URLError(.notConnectedToInternet)
        }
        let response = await client.processMessage("120 bpm bar duration")
        XCTAssertEqual(response.respondingAgent, "deterministic")
        XCTAssertTrue(response.responseText.contains("2.000"))
    }

    func testChordInKeyResolvesLocally() async {
        let client = makeMockClient(sessionId: "test-chord") { _ in
            XCTFail("Brain should NOT be called for chord-in-key lookup")
            throw URLError(.notConnectedToInternet)
        }
        let response = await client.processMessage("Am in C major")
        XCTAssertEqual(response.respondingAgent, "deterministic")
        XCTAssertTrue(response.responseText.contains("vi"))
    }

    func testFrequencyLookupResolvesLocally() async {
        let client = makeMockClient(sessionId: "test-freq") { _ in
            XCTFail("Brain should NOT be called for frequency lookup")
            throw URLError(.notConnectedToInternet)
        }
        let response = await client.processMessage("1000hz band")
        XCTAssertEqual(response.respondingAgent, "deterministic")
        XCTAssertTrue(response.responseText.contains("Mids"))
    }

    // MARK: - Brain forwarding (mock network)

    func testNonDeterministicMessageForwardedToBrain() async throws {
        let sessionId = UUID().uuidString
        let client = makeMockClient(sessionId: sessionId) { request in
            // Validate it's a POST to /api/message
            XCTAssertEqual(request.httpMethod, "POST")
            XCTAssertEqual(request.url?.path, "/api/message")

            let data = try makeResponseData(
                sessionId: sessionId,
                responseText: "Modal interchange blends chords from parallel modes.",
                respondingAgent: "ruflo"
            )
            return (
                HTTPURLResponse(
                    url: request.url!,
                    statusCode: 200,
                    httpVersion: nil,
                    headerFields: ["Content-Type": "application/json"]
                )!,
                data
            )
        }
        let response = await client.processMessage("explain modal interchange", domainHint: "music")
        XCTAssertEqual(response.respondingAgent, "ruflo")
        XCTAssertTrue(response.responseText.contains("Modal interchange"))
        XCTAssertEqual(response.sessionId, sessionId)
    }

    func testBrainResponseIncludesBudgetStatus() async throws {
        let sessionId = UUID().uuidString
        let client = makeMockClient(sessionId: sessionId) { request in
            let data = try makeResponseData(
                sessionId: sessionId,
                responseText: "Here is the answer."
            )
            return (
                HTTPURLResponse(url: request.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!,
                data
            )
        }
        let response = await client.processMessage("write a hook for me")
        XCTAssertNotNil(response.budgetStatus)
        XCTAssertEqual(response.budgetStatus?.status, "green")
    }

    // MARK: - Offline queue on network failure

    func testNetworkFailureQueuesOffline() async {
        let client = makeOfflineClient(sessionId: "test-offline")
        let response = await client.processMessage("help me write a verse", domainHint: "music")
        XCTAssertEqual(response.respondingAgent, "offline")
        XCTAssertEqual(response.resolvedAtTier, 99)
        XCTAssertEqual(response.confidence, 0.0)
        XCTAssertTrue(response.responseText.lowercased().contains("queue"))

        let count = await client.offlineQueueCount
        XCTAssertEqual(count, 1, "Message should be in the offline queue")
    }

    func testMultipleOfflineMessagesAccumulate() async {
        let client = makeOfflineClient(sessionId: "test-multi-offline")
        _ = await client.processMessage("message one")
        _ = await client.processMessage("message two")
        _ = await client.processMessage("message three")
        let count = await client.offlineQueueCount
        XCTAssertEqual(count, 3)
    }

    // MARK: - Session ID

    func testSessionIdIsEchoed() async {
        let mySessionId = "my-custom-session-42"
        let client = makeMockClient(sessionId: mySessionId) { request in
            let data = try makeResponseData(sessionId: mySessionId, responseText: "answer")
            return (
                HTTPURLResponse(url: request.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!,
                data
            )
        }
        let response = await client.processMessage("what is a chord?")
        XCTAssertEqual(response.sessionId, mySessionId)
    }

    // MARK: - currentSessionId (nonisolated)

    func testCurrentSessionIdMatchesInit() {
        let id = "fixed-id-123"
        let client = NanoClawClient(
            brainURL: URL(string: "http://mock.local")!,
            sessionId: id
        )
        XCTAssertEqual(client.currentSessionId, id)
    }

    // MARK: - Flush queue (all succeed)

    func testFlushOfflineQueueSendsAllMessages() async throws {
        // 1. Populate the queue by failing twice.
        let client = makeOfflineClient(sessionId: "flush-test")
        _ = await client.processMessage("msg A", domainHint: "music")
        _ = await client.processMessage("msg B", domainHint: "music")

        let countBefore = await client.offlineQueueCount
        XCTAssertEqual(countBefore, 2)

        // 2. Swap to a successful handler.
        var callCount = 0
        MockURLProtocol.requestHandler = { request in
            callCount += 1
            let data = try makeResponseData(sessionId: "flush-test", responseText: "ok \(callCount)")
            return (
                HTTPURLResponse(url: request.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!,
                data
            )
        }

        let results = await client.flushOfflineQueue()
        XCTAssertEqual(results.count, 2)
        XCTAssertEqual(callCount, 2, "Both messages should have been forwarded to the brain")

        let countAfter = await client.offlineQueueCount
        XCTAssertEqual(countAfter, 0, "Queue should be empty after a successful flush")
    }

    // MARK: - Flush empty queue

    func testFlushEmptyQueueReturnsEmptyArray() async {
        let client = makeMockClient(sessionId: "empty-flush") { _ in
            XCTFail("No request should be made when flushing an empty queue")
            throw URLError(.notConnectedToInternet)
        }
        let results = await client.flushOfflineQueue()
        XCTAssertTrue(results.isEmpty)
    }

    // MARK: - isOfflineQueueEmpty

    func testIsOfflineQueueEmpty_startsEmpty() async {
        let client = makeOfflineClient()
        let empty = await client.isOfflineQueueEmpty
        XCTAssertTrue(empty)
    }

    func testIsOfflineQueueEmpty_afterEnqueue() async {
        let client = makeOfflineClient(sessionId: "empty-check")
        _ = await client.processMessage("queued message")
        let empty = await client.isOfflineQueueEmpty
        XCTAssertFalse(empty)
    }
}
