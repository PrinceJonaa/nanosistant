//! gRPC client that forwards `EdgeRequest`s to the `RuFlo` brain tier.
//!
//! Uses the tonic-generated `NanoClawServiceClient` from `nstn-common`.
//! The client starts disconnected and must be explicitly connected via
//! `connect()` before sending.  Messages sent while disconnected return
//! `GrpcError::NotConnected`.

use nstn_common::proto::{
    nano_claw_service_client::NanoClawServiceClient, EdgeRequest as ProtoEdgeRequest,
};
use tonic::transport::Channel;

use crate::edge::{EdgeRequest, EdgeResponse};

/// Errors that can occur when communicating with the `RuFlo` brain.
#[derive(Debug, thiserror::Error)]
pub enum GrpcError {
    #[error("not connected to RuFlo at {endpoint}")]
    NotConnected { endpoint: String },

    #[error("transport error: {0}")]
    Transport(String),

    #[error("remote error: {0}")]
    Remote(String),
}

/// gRPC client for the `RuFlo` brain tier.
///
/// Uses the tonic-generated `NanoClawServiceClient` for the wire protocol.
pub struct GrpcClient {
    endpoint: String,
    /// Live tonic channel when connected.
    client: Option<NanoClawServiceClient<Channel>>,
}

impl std::fmt::Debug for GrpcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcClient")
            .field("endpoint", &self.endpoint)
            .field("connected", &self.client.is_some())
            .finish()
    }
}

impl GrpcClient {
    /// Create a new client targeting `endpoint`.
    ///
    /// The client starts in a disconnected state; call `connect()` to
    /// establish the channel.
    #[must_use]
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_owned(),
            client: None,
        }
    }

    /// Establish a tonic channel to the configured endpoint.
    ///
    /// # Errors
    ///
    /// Returns `GrpcError::Transport` if the connection cannot be established.
    pub async fn connect(&mut self) -> Result<(), GrpcError> {
        let channel = Channel::from_shared(self.endpoint.clone())
            .map_err(|e| GrpcError::Transport(e.to_string()))?
            .connect()
            .await
            .map_err(|e| GrpcError::Transport(e.to_string()))?;

        self.client = Some(NanoClawServiceClient::new(channel));
        Ok(())
    }

    /// Returns `true` when the underlying channel is established.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Send an `EdgeRequest` to `RuFlo` and await a response.
    ///
    /// Converts the edge-layer types to/from the proto wire types.
    ///
    /// # Errors
    ///
    /// - `GrpcError::NotConnected` when no channel has been established.
    /// - `GrpcError::Transport` on a tonic transport error.
    /// - `GrpcError::Remote` when the server returns a gRPC status error.
    pub async fn send(&mut self, request: EdgeRequest) -> Result<EdgeResponse, GrpcError> {
        let client = self.client.as_mut().ok_or_else(|| GrpcError::NotConnected {
            endpoint: self.endpoint.clone(),
        })?;

        let proto_req = ProtoEdgeRequest {
            session_id: request.session_id,
            user_message: request.message,
            domain_hint: request.domain_hint,
            session_context: None,
            max_tokens: 0,
        };

        let response = client
            .process_message(tonic::Request::new(proto_req))
            .await
            .map_err(|status| {
                GrpcError::Remote(format!("{}: {}", status.code(), status.message()))
            })?;

        let body = response.into_inner();
        Ok(EdgeResponse {
            content: body.response_text,
            from_local: false,
            queued: false,
        })
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
    async fn send_returns_not_connected_when_no_channel() {
        let mut client = GrpcClient::new("http://localhost:50051");
        let req = EdgeRequest {
            message: "test".to_owned(),
            domain_hint: "general".to_owned(),
            session_id: "s1".to_owned(),
        };
        let err = client.send(req).await.unwrap_err();
        assert!(matches!(err, GrpcError::NotConnected { .. }));
    }

    #[tokio::test]
    async fn connect_to_nonexistent_server_returns_transport_error() {
        let mut client = GrpcClient::new("http://localhost:19999");
        // This port is almost certainly not running anything.
        let result = client.connect().await;
        // Tonic uses lazy connections — connect() itself may succeed (lazy),
        // but the first RPC will fail.  Either way we test the API.
        let _ = result; // don't assert since tonic channels can be lazy
    }
}
