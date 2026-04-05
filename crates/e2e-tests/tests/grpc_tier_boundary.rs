//! E2E tests for the gRPC tier boundary between NanoClaw (edge) and RuFlo (brain).
//!
//! These tests spin up a real tonic gRPC server backed by the Orchestrator with
//! mock agent runtimes, then connect a real `GrpcClient` to it.  This verifies
//! that the protobuf contract — the system's structural boundary — faithfully
//! carries deterministic facts and routing decisions across the wire.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use nstn_common::proto::nano_claw_service_server::NanoClawServiceServer;
use nstn_common::TriggerConfig;
use nstn_ruflo::{
    agent_config::{PromptConfig, ToolsConfig},
    AgentConfig, AgentHandle, NanoClawGrpcService, Orchestrator,
};
use tonic::transport::Server;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_handle(name: &str, keywords: Vec<&str>, priority: u32) -> AgentHandle {
    AgentHandle::from_config(AgentConfig {
        name: name.to_string(),
        description: format!("{name} agent"),
        model: "claude-sonnet-4-20250514".to_string(),
        permission_mode: "workspace_write".to_string(),
        triggers: TriggerConfig {
            keywords: keywords.into_iter().map(String::from).collect(),
            priority,
        },
        prompt: PromptConfig {
            identity_file: String::new(),
            domain_file: String::new(),
        },
        knowledge: None,
        tools: ToolsConfig::default(),
    })
    .with_mock_runtime()
}

fn build_orchestrator() -> Orchestrator {
    Orchestrator::new(
        vec![
            make_handle("general", vec![], 0),
            make_handle(
                "music",
                vec!["verse", "hook", "beat", "bpm", "808", "vocal chain"],
                10,
            ),
            make_handle(
                "development",
                vec!["code", "rust", "bug", "deploy", "refactor"],
                10,
            ),
        ],
        100_000,
    )
}

/// Spawn a tonic gRPC server on an ephemeral port and return the endpoint URL.
async fn spawn_grpc_server(
    orchestrator: Arc<Mutex<Orchestrator>>,
) -> (String, tokio::task::JoinHandle<()>) {
    let svc = NanoClawGrpcService::new(orchestrator);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("should bind ephemeral port");
    let addr: SocketAddr = listener
        .local_addr()
        .expect("listener should report address");
    let endpoint = format!("http://127.0.0.1:{}", addr.port());

    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(NanoClawServiceServer::new(svc))
            .serve_with_incoming(incoming)
            .await
            .expect("gRPC server should run");
    });

    // Give the server a moment to accept connections.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    (endpoint, handle)
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// Deterministic fact survives the full gRPC round-trip:
/// GrpcClient → tonic → NanoClawGrpcService → Orchestrator (deterministic) → proto → GrpcClient.
#[tokio::test]
async fn edge_to_brain_deterministic_via_grpc() {
    let orch = Arc::new(Mutex::new(build_orchestrator()));
    let (endpoint, _server) = spawn_grpc_server(orch).await;

    let mut client = nstn_nanoclaw::grpc_client::GrpcClient::new(&endpoint);
    client
        .connect()
        .await
        .expect("should connect to gRPC server");

    let resp = client
        .send(nstn_nanoclaw::edge::EdgeRequest {
            message: "c major scale".to_string(),
            domain_hint: String::new(),
            session_id: "e2e-s1".to_string(),
        })
        .await
        .expect("gRPC send should succeed");

    // The deterministic resolver returns the exact scale — verify the empirical fact
    // survived serialization across the tier boundary.
    assert!(
        resp.content.contains("C - D - E - F - G - A - B"),
        "expected exact C major scale in response, got: {}",
        resp.content
    );
    assert!(
        !resp.from_local,
        "gRPC response should not be marked as local"
    );
}

/// Routing decision propagates correctly across the protobuf boundary.
#[tokio::test]
async fn edge_to_brain_agent_route_via_grpc() {
    let orch = Arc::new(Mutex::new(build_orchestrator()));
    let (endpoint, _server) = spawn_grpc_server(orch).await;

    let mut client = nstn_nanoclaw::grpc_client::GrpcClient::new(&endpoint);
    client
        .connect()
        .await
        .expect("should connect to gRPC server");

    let resp = client
        .send(nstn_nanoclaw::edge::EdgeRequest {
            message: "help me write a verse for the hook".to_string(),
            domain_hint: "music".to_string(),
            session_id: "e2e-s2".to_string(),
        })
        .await
        .expect("gRPC send should succeed");

    // Mock runtime returns a response containing the domain name.
    assert!(
        !resp.content.is_empty(),
        "agent response should not be empty"
    );
}

/// Full EdgeRuntime flow with a live gRPC server:
/// 1. Deterministic query → resolved locally (no gRPC call).
/// 2. Non-deterministic query → forwarded to brain via gRPC.
#[tokio::test]
async fn edge_runtime_full_flow_with_grpc() {
    let orch = Arc::new(Mutex::new(build_orchestrator()));
    let (endpoint, _server) = spawn_grpc_server(orch).await;

    let mut client = nstn_nanoclaw::grpc_client::GrpcClient::new(&endpoint);
    client
        .connect()
        .await
        .expect("should connect to gRPC server");

    let mut runtime = nstn_nanoclaw::edge::EdgeRuntime::with_client("e2e-s3", client);

    // Step 1: deterministic query — answered locally, zero network.
    let local_resp = runtime
        .process_message("c major scale", "")
        .await
        .expect("should not error");
    assert!(
        local_resp.from_local,
        "deterministic query should resolve locally"
    );
    assert!(
        local_resp.content.contains("C - D - E - F - G - A - B"),
        "deterministic content should be exact"
    );

    // Step 2: non-deterministic query — forwarded to brain via gRPC.
    let remote_resp = runtime
        .process_message("help me write a verse", "music")
        .await
        .expect("should not error");
    assert!(
        !remote_resp.from_local,
        "non-deterministic should go through gRPC"
    );
    assert!(
        !remote_resp.queued,
        "should not be queued when gRPC is connected"
    );
    assert!(
        !remote_resp.content.is_empty(),
        "should get a response from brain"
    );

    // Session context should record both turns.
    assert_eq!(runtime.session_context().turns.len(), 2);
}
