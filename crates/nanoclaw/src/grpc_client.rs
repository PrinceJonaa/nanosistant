//! gRPC client that forwards `EdgeRequest`s to the `RuFlo` brain tier.
//!
//! For v0.1.0 this is a stub: the connection is always treated as
//! unavailable so that the interface can be finalised before wiring up
//! the real tonic channel in the integration layer.

use crate::edge::{EdgeRequest, EdgeResponse};
use thiserror::Error;

/// Errors that can occur when communicating with the `RuFlo` brain.
#[derive(Debug, Error)]
pub enum GrpcError {
    #[error("not connected to RuFlo at {endpoint}")]
    NotConnected { endpoint: String },

    #[error("transport error: {0}")]
    Transport(String),

    #[error("remote error: {0}")]
    Remote(String),
}

/// gRPC client for the `RuFlo` brain tier.
#[derive(Debug, Clone)]
pub struct GrpcClient {
    endpoint: String,
    /// Whether a live channel has been established.
    ///
    /// Always `false` in v0.1.0 (stub implementation).
    connected: bool,
}

impl GrpcClient {
    /// Create a new client targeting `endpoint`.
    ///
    /// The client starts in a disconnected state; real connection logic
    /// will be added during integration.
    #[must_use]
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_owned(),
            connected: false,
        }
    }

    /// Send an `EdgeRequest` to `RuFlo` and await a response.
    ///
    /// # Errors
    ///
    /// Always returns [`GrpcError::NotConnected`] in the v0.1.0 stub.
    pub async fn send(&self, _request: EdgeRequest) -> Result<EdgeResponse, GrpcError> {
        // v0.1.0 stub — real tonic call wired up in integration.
        Err(GrpcError::NotConnected {
            endpoint: self.endpoint.clone(),
        })
    }

    /// Returns `true` when the underlying channel is established.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_client_is_disconnected() {
        let client = GrpcClient::new("http://localhost:50051");
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn send_returns_not_connected() {
        let client = GrpcClient::new("http://localhost:50051");
        let req = EdgeRequest {
            message: "test".to_owned(),
            domain_hint: "general".to_owned(),
            session_id: "s1".to_owned(),
        };
        let err = client.send(req).await.unwrap_err();
        assert!(matches!(err, GrpcError::NotConnected { .. }));
    }
}
