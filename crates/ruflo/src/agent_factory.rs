//! Agent factory — constructs `AgentHandle` instances from `AgentConfig`.
//!
//! `AgentHandle` now holds an optional `AgentRuntime` that the orchestrator
//! can call `run_turn()` on.  Use `with_mock_runtime()` to attach a
//! `MockAgentRuntime` for testing or offline mode.

use nstn_common::{Domain, DomainClassifier};

use crate::agent_config::AgentConfig;

// ─── AgentTurnResult ──────────────────────────────────────────────────────────

/// The result of one agent turn.
#[derive(Debug, Clone)]
pub struct AgentTurnResult {
    /// The textual response produced by the agent.
    pub response_text: String,
    /// Names of tools called during this turn.
    pub tool_calls: Vec<String>,
    /// Tokens consumed.
    pub tokens_used: u32,
    /// Agentic iterations performed.
    pub iterations: usize,
}

// ─── AgentRuntime trait ───────────────────────────────────────────────────────

/// Trait for abstracting over different runtime implementations.
pub trait AgentRuntime: Send {
    /// Execute one conversation turn for the given `message`.
    ///
    /// # Errors
    /// Returns an error string if the turn cannot be completed.
    fn run_turn(&mut self, message: &str) -> Result<AgentTurnResult, String>;

    /// Total tokens consumed across all turns so far.
    fn session_tokens(&self) -> usize;
}

// ─── MockAgentRuntime ─────────────────────────────────────────────────────────

/// A mock runtime that returns canned responses.
///
/// Used in tests and when no real API client is available.
pub struct MockAgentRuntime {
    domain: String,
    turn_count: usize,
}

impl MockAgentRuntime {
    /// Create a new mock runtime for the given domain.
    #[must_use]
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            turn_count: 0,
        }
    }
}

impl AgentRuntime for MockAgentRuntime {
    fn run_turn(&mut self, message: &str) -> Result<AgentTurnResult, String> {
        self.turn_count += 1;
        Ok(AgentTurnResult {
            response_text: format!(
                "[{}] Processed: {}",
                self.domain,
                &message[..message.len().min(50)]
            ),
            tool_calls: vec![],
            tokens_used: 100,
            iterations: 1,
        })
    }

    fn session_tokens(&self) -> usize {
        self.turn_count * 100
    }
}

// ─── AgentHandle ──────────────────────────────────────────────────────────────

/// A resolved agent with its config, domain identity, and optional runtime.
///
/// The `domain` field is derived from the agent's `name` field — each agent
/// owns exactly one domain.  The `DomainClassifier` inside the orchestrator
/// routes messages to the correct handle.
pub struct AgentHandle {
    pub config: AgentConfig,
    pub domain: Domain,
    /// The actual runtime for this agent.
    ///
    /// `None` until the agent is initialised with an API client.
    /// Use `with_mock_runtime()` for testing/offline mode.
    runtime: Option<Box<dyn AgentRuntime>>,
}

impl std::fmt::Debug for AgentHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentHandle")
            .field("config", &self.config)
            .field("domain", &self.domain)
            .field("has_runtime", &self.runtime.is_some())
            .finish()
    }
}

impl AgentHandle {
    /// Create an `AgentHandle` from an `AgentConfig` with no runtime attached.
    ///
    /// The domain is taken directly from `config.name`.
    #[must_use]
    pub fn from_config(config: AgentConfig) -> Self {
        let domain = Domain::new(config.name.clone());
        Self {
            config,
            domain,
            runtime: None,
        }
    }

    /// Attach a `MockAgentRuntime` (for testing or offline mode).
    #[must_use]
    pub fn with_mock_runtime(mut self) -> Self {
        let domain = self.domain.name().to_string();
        self.runtime = Some(Box::new(MockAgentRuntime::new(domain)));
        self
    }

    /// Attach a custom runtime.
    #[must_use]
    pub fn with_runtime(mut self, runtime: Box<dyn AgentRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Get a mutable reference to the runtime, if one is attached.
    pub fn runtime_mut(&mut self) -> Option<&mut dyn AgentRuntime> {
        match self.runtime {
            Some(ref mut r) => Some(r.as_mut()),
            None => None,
        }
    }

    /// Return the agent name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Return the agent's permission mode string.
    #[must_use]
    pub fn permission_mode(&self) -> &str {
        &self.config.permission_mode
    }
}

// ─── AgentFactory ─────────────────────────────────────────────────────────────

/// Builds a set of `AgentHandle` instances from configs and registers them
/// with a `DomainClassifier`.
pub struct AgentFactory;

impl AgentFactory {
    /// Convert a list of `AgentConfig` into `AgentHandle` instances and
    /// populate a `DomainClassifier` with their trigger configurations.
    ///
    /// All handles are created without a runtime attached; attach one with
    /// `AgentHandle::with_mock_runtime()` or `AgentHandle::with_runtime()`.
    #[must_use]
    pub fn build(configs: Vec<AgentConfig>) -> (Vec<AgentHandle>, DomainClassifier) {
        let mut classifier = DomainClassifier::new();
        let mut handles = Vec::with_capacity(configs.len());

        for config in configs {
            classifier.register(&config.name, config.triggers.clone());
            handles.push(AgentHandle::from_config(config));
        }

        (handles, classifier)
    }

    /// Like `build`, but attaches mock runtimes to all handles.
    ///
    /// Convenient for tests and offline scenarios.
    #[must_use]
    pub fn build_with_mocks(configs: Vec<AgentConfig>) -> (Vec<AgentHandle>, DomainClassifier) {
        let (handles, classifier) = Self::build(configs);
        let handles = handles
            .into_iter()
            .map(AgentHandle::with_mock_runtime)
            .collect();
        (handles, classifier)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_config::{PromptConfig, ToolsConfig};
    use nstn_common::TriggerConfig;

    fn make_config(name: &str, keywords: Vec<&str>, priority: u32) -> AgentConfig {
        AgentConfig {
            name: name.to_string(),
            description: format!("{name} agent"),
            model: "claude-sonnet-4-20250514".to_string(),
            permission_mode: "read_only".to_string(),
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
        }
    }

    #[test]
    fn handle_from_config_sets_domain_from_name() {
        let cfg = make_config("music", vec!["verse", "beat"], 10);
        let handle = AgentHandle::from_config(cfg);
        assert_eq!(handle.domain.name(), "music");
        assert_eq!(handle.name(), "music");
    }

    #[test]
    fn factory_builds_handles_and_classifier() {
        let configs = vec![
            make_config("music", vec!["verse", "hook", "beat"], 10),
            make_config("investment", vec!["stock", "earnings"], 10),
            make_config("general", vec![], 0),
        ];

        let (handles, classifier) = AgentFactory::build(configs);
        assert_eq!(handles.len(), 3);

        // Classifier should route music keywords to "music"
        let domain = classifier.classify("help me write a verse");
        assert_eq!(domain.name(), "music");

        // Classifier should route investment keywords to "investment"
        let domain = classifier.classify("analyze the stock earnings");
        assert_eq!(domain.name(), "investment");
    }

    #[test]
    fn factory_registers_all_domains() {
        let configs = vec![
            make_config("music", vec!["verse"], 10),
            make_config("development", vec!["code"], 10),
            make_config("framework", vec!["distortion"], 15),
        ];

        let (handles, classifier) = AgentFactory::build(configs);
        let names: Vec<&str> = handles.iter().map(AgentHandle::name).collect();
        assert!(names.contains(&"music"));
        assert!(names.contains(&"development"));
        assert!(names.contains(&"framework"));

        let mut domain_names = classifier.domain_names();
        domain_names.sort();
        assert!(domain_names.contains(&"music".to_string()));
        assert!(domain_names.contains(&"development".to_string()));
    }

    #[test]
    fn handle_permission_mode_accessible() {
        let cfg = make_config("music", vec![], 0);
        let mut cfg = cfg;
        cfg.permission_mode = "workspace_write".to_string();
        let handle = AgentHandle::from_config(cfg);
        assert_eq!(handle.permission_mode(), "workspace_write");
    }

    // ── New runtime tests ────────────────────────────────────────────────────

    #[test]
    fn mock_runtime_returns_domain_prefixed_response() {
        let mut rt = MockAgentRuntime::new("music");
        let result = rt.run_turn("write me a verse").expect("should succeed");
        assert!(result.response_text.starts_with("[music]"));
        assert_eq!(result.tokens_used, 100);
        assert_eq!(result.iterations, 1);
    }

    #[test]
    fn mock_runtime_tracks_session_tokens() {
        let mut rt = MockAgentRuntime::new("general");
        assert_eq!(rt.session_tokens(), 0);
        let _ = rt.run_turn("first turn");
        assert_eq!(rt.session_tokens(), 100);
        let _ = rt.run_turn("second turn");
        assert_eq!(rt.session_tokens(), 200);
    }

    #[test]
    fn handle_with_mock_runtime_can_run_turn() {
        let cfg = make_config("music", vec!["verse"], 10);
        let mut handle = AgentHandle::from_config(cfg).with_mock_runtime();
        let rt = handle.runtime_mut().expect("should have runtime");
        let result = rt.run_turn("help me write a chorus").expect("should succeed");
        assert!(!result.response_text.is_empty());
    }

    #[test]
    fn handle_without_runtime_returns_none() {
        let cfg = make_config("music", vec!["verse"], 10);
        let mut handle = AgentHandle::from_config(cfg);
        assert!(handle.runtime_mut().is_none());
    }

    #[test]
    fn factory_build_with_mocks_attaches_runtimes() {
        let configs = vec![
            make_config("music", vec!["verse"], 10),
            make_config("general", vec![], 0),
        ];
        let (mut handles, _) = AgentFactory::build_with_mocks(configs);
        for handle in &mut handles {
            assert!(handle.runtime_mut().is_some(), "each handle should have a runtime");
        }
    }

    #[test]
    fn mock_runtime_truncates_long_message() {
        let mut rt = MockAgentRuntime::new("general");
        let long_msg = "a".repeat(200);
        let result = rt.run_turn(&long_msg).expect("should succeed");
        // The preview is capped at 50 chars
        assert!(result.response_text.len() < 100);
    }
}
