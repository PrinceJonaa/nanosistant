//! Integration tests for the full routing pipeline.
//!
//! Tests the complete flow: deterministic → confidence ladder → ruflo fallback.

use nstn_common::{try_deterministic_resolution, router_from_trigger_configs, TriggerConfig};
use nstn_ruflo::{
    AgentConfig, AgentHandle, BudgetManager, Orchestrator, RouteResult,
    agent_config::PromptConfig,
    agent_config::ToolsConfig,
};

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
            identity_file: "config/prompts/identity.md".to_string(),
            domain_file: format!("config/prompts/{name}.md"),
        },
        knowledge: None,
        tools: ToolsConfig::default(),
    })
}

fn build_test_orchestrator() -> Orchestrator {
    Orchestrator::new(
        vec![
            make_handle("general", vec![], 0),
            make_handle("music", vec!["verse", "hook", "beat", "bpm", "808", "vocal chain"], 10),
            make_handle("investment", vec!["stock", "earnings", "revenue", "13F", "institutional"], 10),
            make_handle("development", vec!["code", "rust", "bug", "deploy", "refactor"], 10),
            make_handle("framework", vec!["distortion", "lattice", "archetype", "false prophet"], 15),
        ],
        100_000,
    )
}

// ── Full pipeline tests ──

#[test]
fn deterministic_intercepts_before_any_routing() {
    let mut orch = build_test_orchestrator();
    let result = orch.route("s1", "c major scale", "");
    match result {
        RouteResult::Deterministic { response, .. } => {
            assert!(response.contains("C - D - E - F - G - A - B"));
        }
        other => panic!("expected Deterministic, got {other:?}"),
    }
}

#[test]
fn confidence_ladder_routes_clear_domain_queries() {
    let mut orch = build_test_orchestrator();

    // Music
    let result = orch.route("s1", "help me write a verse for the hook", "");
    match &result {
        RouteResult::AgentRoute { domain, confidence, .. } => {
            assert_eq!(domain, "music");
            assert!(*confidence > 0.0);
        }
        other => panic!("expected AgentRoute to music, got {other:?}"),
    }

    // Investment
    let result = orch.route("s1", "what are the stock earnings?", "");
    match &result {
        RouteResult::AgentRoute { domain, .. } => assert_eq!(domain, "investment"),
        other => panic!("expected AgentRoute to investment, got {other:?}"),
    }

    // Development
    let result = orch.route("s1", "fix this rust bug in the code", "");
    match &result {
        RouteResult::AgentRoute { domain, .. } => assert_eq!(domain, "development"),
        other => panic!("expected AgentRoute to development, got {other:?}"),
    }
}

#[test]
fn domain_hint_overrides_everything() {
    let mut orch = build_test_orchestrator();
    let result = orch.route("s1", "help me write a verse", "investment");
    match result {
        RouteResult::AgentRoute { domain, confidence, .. } => {
            assert_eq!(domain, "investment");
            assert_eq!(confidence, 1.0);
        }
        other => panic!("expected AgentRoute with hint override, got {other:?}"),
    }
}

#[test]
fn budget_exhaustion_blocks_routing() {
    let mut orch = Orchestrator::new(
        vec![make_handle("general", vec![], 0)],
        100,
    );
    orch.record_turn_tokens(100);
    let result = orch.route("s1", "hello", "");
    assert!(matches!(result, RouteResult::BudgetExhausted { .. }));
}

#[test]
fn ambiguous_queries_without_ruflo_return_ambiguous() {
    let mut orch = build_test_orchestrator();
    // No ruflo running — ambiguous should fall through
    let result = orch.route("s1", "tell me about the weather in paris", "");
    match result {
        RouteResult::Ambiguous { best_guess, confidence, scores, .. } => {
            assert!(!best_guess.is_empty());
            assert!(confidence < 1.0);
            assert!(!scores.is_empty());
        }
        RouteResult::AgentRoute { .. } => {
            // Also acceptable if the ladder found a weak match
        }
        other => panic!("expected Ambiguous or weak AgentRoute, got {other:?}"),
    }
}

#[test]
fn deterministic_functions_work_standalone() {
    // BPM
    assert!(try_deterministic_resolution("140 bpm bar duration").is_some());
    // Scale
    assert!(try_deterministic_resolution("c major scale").is_some());
    // Chord in key
    assert!(try_deterministic_resolution("Am in C major").is_some());
    // Non-deterministic
    assert!(try_deterministic_resolution("help me with my taxes").is_none());
}

#[test]
fn confidence_ladder_builds_from_trigger_configs() {
    let configs = vec![
        ("music".to_string(), TriggerConfig {
            keywords: vec!["verse".into(), "hook".into()],
            priority: 10,
        }),
    ];
    let router = router_from_trigger_configs(&configs);
    let decision = router.route("write me a verse");
    assert!(decision.is_confident());
    assert_eq!(decision.domain.as_deref(), Some("music"));
}
