//! Edge runtime — the entry point for all user messages on the device tier.
//!
//! Processing order:
//! 1. Try local deterministic resolution (zero network).
//! 2. If resolved → return immediately.
//! 3. If not resolved and a gRPC client is available → forward to `RuFlo`.
//! 4. If offline → enqueue for later delivery and return a queued notice.

use crate::grpc_client::{GrpcClient, GrpcError};
use crate::local::LocalExecutor;
use crate::sync::OfflineQueue;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Public request / response types ────────────────────────────────────────

/// A request forwarded to the `RuFlo` brain tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRequest {
    pub message: String,
    pub domain_hint: String,
    pub session_id: String,
}

/// The response returned to the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeResponse {
    /// The textual answer or status message.
    pub content: String,
    /// `true` when the answer was produced by a local deterministic function.
    pub from_local: bool,
    /// `true` when the message was accepted but queued for later delivery.
    pub queued: bool,
}

impl EdgeResponse {
    fn local(content: String) -> Self {
        Self { content, from_local: true, queued: false }
    }

    #[allow(dead_code)]
    fn remote(content: String) -> Self {
        Self { content, from_local: false, queued: false }
    }

    fn queued_notice(message: &str) -> Self {
        Self {
            content: format!("offline — message queued: \"{message}\""),
            from_local: false,
            queued: true,
        }
    }
}

// ─── Session context ─────────────────────────────────────────────────────────

/// Lightweight per-session state held at the edge.
#[derive(Debug, Default, Clone)]
pub struct SessionContext {
    /// Ordered history of (`user_message`, `assistant_response`) pairs.
    pub turns: Vec<(String, String)>,
}

impl SessionContext {
    fn record(&mut self, message: &str, response: &str) {
        self.turns.push((message.to_owned(), response.to_owned()));
    }
}

// ─── Errors ──────────────────────────────────────────────────────────────────

/// Errors produced by the edge runtime.
#[derive(Debug, Error)]
pub enum EdgeError {
    #[error("gRPC error: {0}")]
    Grpc(#[from] GrpcError),
}

// ─── Edge runtime ─────────────────────────────────────────────────────────────

/// The stateful edge runtime for a single user session.
pub struct EdgeRuntime {
    session_id: String,
    session_context: SessionContext,
    /// `None` when operating in offline / no-client mode.
    grpc_client: Option<GrpcClient>,
    offline_queue: OfflineQueue,
}

impl EdgeRuntime {
    /// Create a new edge runtime for `session_id` with no gRPC client.
    #[must_use]
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_owned(),
            session_context: SessionContext::default(),
            grpc_client: None,
            offline_queue: OfflineQueue::new(),
        }
    }

    /// Create an edge runtime with an explicit gRPC client.
    #[must_use]
    pub fn with_client(session_id: &str, client: GrpcClient) -> Self {
        Self {
            session_id: session_id.to_owned(),
            session_context: SessionContext::default(),
            grpc_client: Some(client),
            offline_queue: OfflineQueue::new(),
        }
    }

    /// Process an incoming `message`.
    ///
    /// Resolution order:
    /// 1. Local deterministic check.
    /// 2. Forward to `RuFlo` via gRPC if client present and connected.
    /// 3. Enqueue if offline.
    ///
    /// # Errors
    ///
    /// Returns [`EdgeError::Grpc`] only when a client is present, connected,
    /// and the remote call fails with a transport/remote error (not
    /// `NotConnected`).
    pub async fn process_message(
        &mut self,
        message: &str,
        domain_hint: &str,
    ) -> Result<EdgeResponse, EdgeError> {
        // Step 1 — deterministic local resolution.
        if let Some(answer) = LocalExecutor::try_resolve(message) {
            let resp = EdgeResponse::local(answer.clone());
            self.session_context.record(message, &answer);
            return Ok(resp);
        }

        // Step 2 — forward to RuFlo if a client is configured.
        if let Some(ref client) = self.grpc_client {
            let request = EdgeRequest {
                message: message.to_owned(),
                domain_hint: domain_hint.to_owned(),
                session_id: self.session_id.clone(),
            };
            match client.send(request).await {
                Ok(resp) => {
                    self.session_context.record(message, &resp.content);
                    return Ok(resp);
                }
                Err(crate::grpc_client::GrpcError::NotConnected { .. }) => {
                    // Fall through to offline queue.
                }
                Err(e) => return Err(EdgeError::Grpc(e)),
            }
        }

        // Step 3 — offline: enqueue for later delivery.
        self.offline_queue.enqueue(message, domain_hint);
        Ok(EdgeResponse::queued_notice(message))
    }

    /// Drain the offline queue (call after connectivity is restored).
    pub fn drain_offline_queue(&mut self) -> Vec<crate::sync::QueuedMessage> {
        self.offline_queue.drain()
    }

    /// Number of messages waiting in the offline queue.
    #[must_use]
    pub fn offline_queue_len(&self) -> usize {
        self.offline_queue.len()
    }

    /// The session identifier for this runtime.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Read-only view of the current session context.
    #[must_use]
    pub fn session_context(&self) -> &SessionContext {
        &self.session_context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Deterministic resolution ─────────────────────────────────────────────

    #[tokio::test]
    async fn local_deterministic_resolves_without_network() {
        let mut runtime = EdgeRuntime::new("test-session-1");
        // "c major scale" is handled by the deterministic library — no gRPC needed.
        let resp = runtime
            .process_message("c major scale", "music")
            .await
            .expect("should not error");
        assert!(resp.from_local, "expected a local (deterministic) response");
        assert!(!resp.queued);
        assert!(!resp.content.is_empty());
    }

    // ── Offline queue ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn offline_queue_persists_and_drains() {
        let mut runtime = EdgeRuntime::new("test-session-2");
        // Open-ended query → not deterministic → no client → queued.
        let resp = runtime
            .process_message("help me write a chorus", "music")
            .await
            .expect("should not error");
        assert!(resp.queued, "expected the message to be queued");
        assert_eq!(runtime.offline_queue_len(), 1);

        let drained = runtime.drain_offline_queue();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].message, "help me write a chorus");
        assert_eq!(runtime.offline_queue_len(), 0);
    }

    // ── Fallback order ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn edge_runtime_tries_local_first_then_falls_back_to_grpc() {
        // Provide a (stub) gRPC client.
        let client = GrpcClient::new("http://localhost:50051");
        let mut runtime = EdgeRuntime::with_client("test-session-3", client);

        // Deterministic query → answered locally, gRPC never called.
        let resp = runtime
            .process_message("c major scale", "music")
            .await
            .expect("should not error");
        assert!(resp.from_local);
        assert_eq!(runtime.offline_queue_len(), 0);

        // Non-deterministic query with disconnected client → queued.
        let resp2 = runtime
            .process_message("explain modal interchange", "music")
            .await
            .expect("should not error");
        assert!(resp2.queued, "stub client is not connected → should queue");
        assert_eq!(runtime.offline_queue_len(), 1);
    }
}
