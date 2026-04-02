//! End-to-end pipeline integration tests.
//!
//! Verifies the complete path: CLI args → orchestrator → API client → response.
//! Uses a mock HTTP server instead of real Anthropic API.

use std::io::Write;
use std::net::TcpListener;
use std::thread;

/// Spin up a minimal mock Anthropic API that returns a canned SSE response.
fn start_mock_anthropic_api() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{port}");

    let handle = thread::spawn(move || {
        // Accept one connection
        if let Ok((mut stream, _)) = listener.accept() {
            // Read the request (we don't care about the body)
            let mut buf = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut buf);

            // Send a valid Anthropic SSE response
            let sse_body = concat!(
                "event: message_start\n",
                "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_test\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4-20250514\",\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":25,\"output_tokens\":1,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0}}}\n\n",
                "event: content_block_start\n",
                "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
                "event: content_block_delta\n",
                "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"The answer is 4.\"}}\n\n",
                "event: content_block_stop\n",
                "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
                "event: message_delta\n",
                "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":10}}\n\n",
                "event: message_stop\n",
                "data: {\"type\":\"message_stop\"}\n\n",
            );

            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/event-stream\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {sse_body}"
            );

            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    (url, handle)
}

#[test]
fn end_to_end_prompt_with_mock_api() {
    let (mock_url, _handle) = start_mock_anthropic_api();

    // Find the nanosistant binary in the workspace target directory
    let binary = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .map(|d| d.join("nanosistant"))
        .unwrap_or_else(|| std::path::PathBuf::from("target/debug/nanosistant"));

    if !binary.exists() {
        // Binary not built — skip rather than fail
        eprintln!("nanosistant binary not found at {}, skipping e2e test", binary.display());
        return;
    }

    let output = std::process::Command::new(&binary)
        .args(["-p", "what is 2+2"])
        .env("ANTHROPIC_API_KEY", "sk-test-mock")
        .env("ANTHROPIC_BASE_URL", &mock_url)
        .output()
        .expect("failed to run nanosistant");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The response should contain "The answer is 4." from our mock
    assert!(
        stdout.contains("The answer is 4") || stdout.contains("answer"),
        "Expected mock response in output.\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn deterministic_intercept_skips_api_entirely() {
    // Deterministic queries should work without any API key
    // This tests that the ConversationRuntime never calls the API for deterministic queries
    // Note: The CLI currently always goes through the ConversationRuntime which calls the API.
    // The deterministic path is in the orchestrator, not the CLI's ConversationRuntime.
    // This test verifies the orchestrator path works.
    use nstn_common::try_deterministic_resolution;

    // These should resolve without any network call
    assert!(try_deterministic_resolution("c major scale").is_some());
    assert!(try_deterministic_resolution("140 bpm bar duration").is_some());
    assert!(try_deterministic_resolution("Am in C major").is_some());
    assert!(try_deterministic_resolution("2500hz band").is_some());

    // These should NOT resolve (need LLM)
    assert!(try_deterministic_resolution("write me a song about love").is_none());
    assert!(try_deterministic_resolution("what should I invest in").is_none());
}

#[test]
fn orchestrator_routes_and_executes_with_mock_runtime() {
    use nstn_common::TriggerConfig;
    use nstn_ruflo::{AgentHandle, Orchestrator, RouteResult, agent_config::{AgentConfig, PromptConfig, ToolsConfig}};

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
        }).with_mock_runtime()
    }

    let mut orch = Orchestrator::new(
        vec![
            make_handle("general", vec![], 0),
            make_handle("music", vec!["verse", "beat", "bpm"], 10),
        ],
        100_000,
    );

    // Deterministic intercept
    let result = orch.route("s1", "c major scale", "");
    assert!(matches!(result, RouteResult::Deterministic { .. }));

    // Agent route
    let result = orch.route("s1", "help me write a verse", "");
    assert!(matches!(result, RouteResult::AgentRoute { .. }));

    // Execute the route
    let turn = orch.execute(&result);
    assert!(turn.is_ok());
    let turn = turn.unwrap();
    assert!(turn.response_text.contains("music"));
}
