//! E2E tests for session lifecycle: multi-turn budget tracking, session context
//! persistence, and offline queue drain-on-reconnect.

use std::sync::{Arc, Mutex};

use nstn_common::proto::nano_claw_service_server::NanoClawServiceServer;
use nstn_common::TriggerConfig;
use nstn_ruflo::{
    agent_config::{PromptConfig, ToolsConfig},
    AgentConfig, AgentHandle, NanoClawGrpcService, Orchestrator, RouteResult,
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

async fn spawn_grpc_server(
    orchestrator: Arc<Mutex<Orchestrator>>,
) -> (String, tokio::task::JoinHandle<()>) {
    let svc = NanoClawGrpcService::new(orchestrator);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("should bind");
    let addr = listener.local_addr().expect("should report address");
    let endpoint = format!("http://127.0.0.1:{}", addr.port());

    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(NanoClawServiceServer::new(svc))
            .serve_with_incoming(incoming)
            .await
            .expect("gRPC server should run");
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (endpoint, handle)
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// Budget is tracked cumulatively across multiple turns.  When exhausted,
/// the orchestrator returns `BudgetExhausted` — no more tokens spent.
#[test]
fn multi_turn_session_tracks_budget() {
    let mut orch = Orchestrator::new(
        vec![make_handle("music", vec!["verse", "hook"], 10)],
        250, // tight budget
    );

    // Turn 1: should route normally.
    let r1 = orch.route("budget-s1", "help me write a verse", "music");
    assert!(
        matches!(r1, RouteResult::AgentRoute { .. }),
        "first turn should route to agent"
    );
    let turn1 = orch.execute(&r1).expect("execute should succeed");
    assert!(!turn1.response_text.is_empty());

    // Record tokens (mock runtime uses 100 tokens per turn).
    orch.record_turn_tokens(100);

    // Turn 2: still within budget.
    let r2 = orch.route("budget-s1", "now write a hook", "music");
    assert!(
        matches!(r2, RouteResult::AgentRoute { .. }),
        "second turn should still route"
    );
    orch.record_turn_tokens(100);

    // Turn 3: record more tokens to exhaust budget.
    orch.record_turn_tokens(100);
    let r3 = orch.route("budget-s1", "one more verse", "music");
    assert!(
        matches!(r3, RouteResult::BudgetExhausted { .. }),
        "should be budget-exhausted after 300 tokens on a 250 budget"
    );
}

/// EdgeRuntime session context accumulates turns across multiple messages.
#[tokio::test]
async fn session_context_persists_across_turns() {
    let mut runtime = nstn_nanoclaw::edge::EdgeRuntime::new("ctx-s1");

    // All deterministic — no gRPC needed, but context should still track.
    let queries = ["c major scale", "140 bpm bar duration", "Am in C major"];

    for query in &queries {
        let resp = runtime
            .process_message(query, "")
            .await
            .expect("should not error");
        assert!(resp.from_local);
    }

    let turns = &runtime.session_context().turns;
    assert_eq!(
        turns.len(),
        3,
        "session context should record all three turns"
    );
    assert_eq!(turns[0].0, "c major scale");
    assert_eq!(turns[1].0, "140 bpm bar duration");
    assert_eq!(turns[2].0, "Am in C major");

    // Each response should contain a real answer, not empty strings.
    for (_, response) in turns {
        assert!(
            !response.is_empty(),
            "each turn response should be non-empty"
        );
    }
}

/// Offline queue accumulates when disconnected, then can drain on reconnect.
#[tokio::test]
async fn offline_queue_drains_on_reconnect() {
    let mut runtime = nstn_nanoclaw::edge::EdgeRuntime::new("offline-s1");

    // No gRPC client → non-deterministic queries get queued.
    let r1 = runtime
        .process_message("explain modal interchange", "music")
        .await
        .expect("should not error");
    assert!(r1.queued, "should be queued when offline");

    let r2 = runtime
        .process_message("what is voice leading", "music")
        .await
        .expect("should not error");
    assert!(r2.queued);

    assert_eq!(
        runtime.offline_queue_len(),
        2,
        "two messages should be queued"
    );

    // Drain the queue (simulating reconnect).
    let drained = runtime.drain_offline_queue();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].message, "explain modal interchange");
    assert_eq!(drained[1].message, "what is voice leading");
    assert_eq!(
        runtime.offline_queue_len(),
        0,
        "queue should be empty after drain"
    );
}

/// Full reconnect scenario: queue while offline, then connect and route through gRPC.
#[tokio::test]
async fn offline_then_online_routes_through_grpc() {
    // Start with no client — messages get queued.
    let mut runtime = nstn_nanoclaw::edge::EdgeRuntime::new("reconnect-s1");
    let r1 = runtime
        .process_message("help me write a verse", "music")
        .await
        .expect("should not error");
    assert!(r1.queued);

    // Deterministic queries still work offline.
    let r2 = runtime
        .process_message("c major scale", "")
        .await
        .expect("should not error");
    assert!(r2.from_local, "deterministic should still resolve offline");
    assert!(!r2.queued);

    // Now spin up a gRPC server and create a new runtime with a connected client.
    let orch = Arc::new(Mutex::new(Orchestrator::new(
        vec![make_handle("music", vec!["verse", "hook"], 10)],
        100_000,
    )));
    let (endpoint, _server) = spawn_grpc_server(orch).await;

    let mut client = nstn_nanoclaw::grpc_client::GrpcClient::new(&endpoint);
    client.connect().await.expect("should connect");

    let mut online_runtime = nstn_nanoclaw::edge::EdgeRuntime::with_client("reconnect-s1", client);

    // Non-deterministic query should now go through gRPC.
    let r3 = online_runtime
        .process_message("help me write a verse", "music")
        .await
        .expect("should not error");
    assert!(!r3.queued, "should not be queued when connected");
    assert!(!r3.from_local, "should go through gRPC");
    assert!(!r3.content.is_empty(), "should get a real response");
}
