//! Agent factory — constructs `AgentHandle` instances from `AgentConfig`.
//!
//! `AgentHandle` is a lightweight wrapper that holds the config and resolved
//! domain.  When the full runtime crate is available, a `ConversationRuntime`
//! will be attached here; for now the handle is the unit of orchestration.

use nstn_common::{Domain, DomainClassifier};

use crate::agent_config::AgentConfig;

// ─── AgentHandle ──────────────────────────────────────────────────────────────

/// A resolved agent with its config and domain identity.
///
/// The `domain` field is derived from the agent's `name` field — each agent
/// owns exactly one domain.  The `DomainClassifier` inside the orchestrator
/// routes messages to the correct handle.
#[derive(Debug, Clone)]
pub struct AgentHandle {
    pub config: AgentConfig,
    pub domain: Domain,
    // Runtime would be attached when the full runtime is available.
}

impl AgentHandle {
    /// Create an `AgentHandle` from an `AgentConfig`.
    ///
    /// The domain is taken directly from `config.name`.
    #[must_use]
    pub fn from_config(config: AgentConfig) -> Self {
        let domain = Domain::new(config.name.clone());
        Self { config, domain }
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
}
