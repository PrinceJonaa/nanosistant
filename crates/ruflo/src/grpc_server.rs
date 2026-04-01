//! gRPC service implementation for the `NanoClawService` trait.
//!
//! When a gRPC request arrives:
//! 1. Extract the `EdgeRequest` from the proto message.
//! 2. Call `orchestrator.route()` to get a routing decision.
//! 3. Call `orchestrator.execute()` to run the agent.
//! 4. Build a proto `EdgeResponse` and return it.

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use nstn_common::proto::{
    nano_claw_service_server::NanoClawService, CompletionStatus, EdgeRequest, EdgeResponse,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::orchestrator::Orchestrator;

// ─── NanoClawGrpcService ─────────────────────────────────────────────────────

/// gRPC service that implements [`NanoClawService`] by delegating to the
/// [`Orchestrator`].
pub struct NanoClawGrpcService {
    orchestrator: Arc<Mutex<Orchestrator>>,
}

impl NanoClawGrpcService {
    /// Create a new service wrapping the given orchestrator.
    #[must_use]
    pub fn new(orchestrator: Arc<Mutex<Orchestrator>>) -> Self {
        Self { orchestrator }
    }
}

#[tonic::async_trait]
impl NanoClawService for NanoClawGrpcService {
    async fn process_message(
        &self,
        request: Request<EdgeRequest>,
    ) -> Result<Response<EdgeResponse>, Status> {
        let req = request.into_inner();

        let (route_result, exec_result) = {
            let mut orch = self
                .orchestrator
                .lock()
                .map_err(|_| Status::internal("orchestrator lock poisoned"))?;

            let route = orch.route(&req.session_id, &req.user_message, &req.domain_hint);
            let exec = orch
                .execute(&route)
                .map_err(|e| Status::internal(format!("execute failed: {e}")))?;
            (route, exec)
        };

        // Determine the responding agent name from the route.
        let responding_agent = match &route_result {
            crate::orchestrator::RouteResult::AgentRoute { agent_name, .. } => {
                agent_name.clone()
            }
            crate::orchestrator::RouteResult::Deterministic { domain, .. } => domain.clone(),
            _ => "orchestrator".to_string(),
        };

        let resp = EdgeResponse {
            session_id: req.session_id,
            response_text: exec_result.response_text,
            responding_agent,
            events: vec![],
            budget: None,
            handoff: None,
            completion: CompletionStatus::Complete as i32,
        };

        Ok(Response::new(resp))
    }

    type StreamMessageStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<EdgeResponse, Status>> + Send + 'static>>;

    async fn stream_message(
        &self,
        request: Request<EdgeRequest>,
    ) -> Result<Response<Self::StreamMessageStream>, Status> {
        // For v0.2, wrap process_message result in a single-item stream.
        let inner_response = self.process_message(request).await?;
        let item = inner_response.into_inner();

        let (tx, rx) = mpsc::channel(1);
        tokio::spawn(async move {
            let _ = tx.send(Ok(item)).await;
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_factory::AgentHandle;
    use crate::agent_config::{AgentConfig, PromptConfig, ToolsConfig};
    use nstn_common::TriggerConfig;
    use nstn_common::proto::EdgeRequest;

    fn make_agent(name: &str) -> AgentHandle {
        let config = AgentConfig {
            name: name.to_string(),
            description: format!("{name} agent"),
            model: "claude-sonnet-4-20250514".to_string(),
            permission_mode: "read_only".to_string(),
            triggers: TriggerConfig {
                keywords: vec!["test".to_string()],
                priority: 10,
            },
            prompt: PromptConfig {
                identity_file: "identity.md".to_string(),
                domain_file: format!("{name}.md"),
            },
            knowledge: None,
            tools: ToolsConfig::default(),
        };
        AgentHandle::from_config(config).with_mock_runtime()
    }

    #[tokio::test]
    async fn grpc_server_process_message_deterministic() {
        let agents = vec![make_agent("music"), make_agent("general")];
        let orch = Arc::new(Mutex::new(Orchestrator::new(agents, 100_000)));
        let svc = NanoClawGrpcService::new(orch);

        let req = Request::new(EdgeRequest {
            session_id: "s1".to_string(),
            user_message: "c major scale".to_string(),
            domain_hint: String::new(),
            session_context: None,
            max_tokens: 1000,
        });

        let resp = svc.process_message(req).await.expect("should succeed");
        let body = resp.into_inner();
        assert!(!body.response_text.is_empty());
    }

    #[tokio::test]
    async fn grpc_server_process_message_agent_route() {
        let agents = vec![make_agent("music"), make_agent("general")];
        let orch = Arc::new(Mutex::new(Orchestrator::new(agents, 100_000)));
        let svc = NanoClawGrpcService::new(orch);

        let req = Request::new(EdgeRequest {
            session_id: "s2".to_string(),
            user_message: "help me write a verse about summer".to_string(),
            domain_hint: "music".to_string(),
            session_context: None,
            max_tokens: 1000,
        });

        let resp = svc.process_message(req).await.expect("should succeed");
        let body = resp.into_inner();
        assert!(!body.response_text.is_empty());
        assert_eq!(body.responding_agent, "music");
    }

    #[tokio::test]
    async fn grpc_server_stream_message_single_item() {
        use tokio_stream::StreamExt;

        let agents = vec![make_agent("general")];
        let orch = Arc::new(Mutex::new(Orchestrator::new(agents, 100_000)));
        let svc = NanoClawGrpcService::new(orch);

        let req = Request::new(EdgeRequest {
            session_id: "s3".to_string(),
            user_message: "c major scale".to_string(),
            domain_hint: String::new(),
            session_context: None,
            max_tokens: 1000,
        });

        let resp = svc.stream_message(req).await.expect("should succeed");
        let mut stream = resp.into_inner();
        let item = stream.next().await.expect("should have one item");
        let body = item.expect("should be ok");
        assert!(!body.response_text.is_empty());
    }
}
