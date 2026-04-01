// BrainClient.swift
// NanoClawKit
//
// HTTP/JSON transport to the Nanosistant axum server.
// gRPC is intentionally deferred to a future version; v0.3.0 uses the
// existing REST endpoints that the axum server already exposes.

import Foundation

// MARK: - BrainClientError

/// Errors produced by `BrainClient`.
public enum BrainClientError: Error, Sendable {
    /// The request completed but the HTTP status was not 2xx.
    case httpError(statusCode: Int, body: String)
    /// JSON encoding/decoding failed.
    case codecError(underlying: Error)
    /// A transport-level failure (no connectivity, DNS, TLS, etc.).
    case transportError(underlying: Error)
    /// The server returned an empty response body.
    case emptyResponse
}

extension BrainClientError: LocalizedError {
    public var errorDescription: String? {
        switch self {
        case .httpError(let code, let body):
            return "HTTP \(code): \(body)"
        case .codecError(let err):
            return "Codec error: \(err.localizedDescription)"
        case .transportError(let err):
            return "Transport error: \(err.localizedDescription)"
        case .emptyResponse:
            return "Empty response from brain"
        }
    }
}

// MARK: - BrainClient

/// HTTP/JSON client for the RuFlo brain server.
///
/// All methods are isolated to the actor; call them from any async context.
///
/// ```swift
/// let client = BrainClient(baseURL: URL(string: "http://192.168.1.10:3000")!)
/// let response = try await client.send(request)
/// ```
public actor BrainClient {

    // MARK: - Properties

    private let baseURL: URL
    private let session: URLSession
    private let encoder: JSONEncoder
    private let decoder: JSONDecoder

    // MARK: - Init

    /// Create a client targeting `baseURL`.
    ///
    /// - Parameter baseURL: Root URL of the axum server, e.g.
    ///   `http://localhost:3000`.  No trailing slash required.
    public init(
        baseURL: URL = URL(string: "http://localhost:3000")!,
        urlSessionConfiguration: URLSessionConfiguration = .default
    ) {
        self.baseURL = baseURL
        self.session = URLSession(configuration: urlSessionConfiguration)

        let enc = JSONEncoder()
        enc.keyEncodingStrategy = .convertToSnakeCase
        self.encoder = enc

        let dec = JSONDecoder()
        dec.keyDecodingStrategy = .convertFromSnakeCase
        self.decoder = dec
    }

    // MARK: - Public API

    /// Send `request` to the brain and decode the `EdgeResponse`.
    ///
    /// - Throws: `BrainClientError` on any transport or protocol failure.
    public func send(_ request: EdgeRequest) async throws -> EdgeResponse {
        let url = baseURL.appendingPathComponent("/api/message")
        var urlRequest = URLRequest(url: url)
        urlRequest.httpMethod = "POST"
        urlRequest.setValue("application/json", forHTTPHeaderField: "Content-Type")
        urlRequest.setValue("application/json", forHTTPHeaderField: "Accept")

        do {
            urlRequest.httpBody = try encoder.encode(request)
        } catch {
            throw BrainClientError.codecError(underlying: error)
        }

        let (data, response): (Data, URLResponse)
        do {
            (data, response) = try await session.data(for: urlRequest)
        } catch {
            throw BrainClientError.transportError(underlying: error)
        }

        if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
            let body = String(data: data, encoding: .utf8) ?? "<binary>"
            throw BrainClientError.httpError(statusCode: http.statusCode, body: body)
        }

        guard !data.isEmpty else {
            throw BrainClientError.emptyResponse
        }

        do {
            return try decoder.decode(EdgeResponse.self, from: data)
        } catch {
            throw BrainClientError.codecError(underlying: error)
        }
    }

    /// Returns `true` if the brain server is reachable.
    ///
    /// Performs a lightweight HEAD request to `/health`.  A non-throwing
    /// completion is treated as reachable regardless of status code so that
    /// callers get a simple Bool rather than handling errors.
    public func isReachable() async -> Bool {
        let url = baseURL.appendingPathComponent("/health")
        var req = URLRequest(url: url)
        req.httpMethod = "HEAD"
        req.timeoutInterval = 5.0

        do {
            let (_, response) = try await session.data(for: req)
            if let http = response as? HTTPURLResponse {
                return (200..<300).contains(http.statusCode)
            }
            return true
        } catch {
            return false
        }
    }
}
