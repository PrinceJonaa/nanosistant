//! Agent configuration — loads per-agent TOML files describing identity,
//! model, domain triggers, prompts, knowledge retrieval, and available tools.

use std::path::Path;

use nstn_common::TriggerConfig;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum AgentConfigError {
    #[error("failed to read config file '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse TOML in '{path}': {source}")]
    Toml {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("config directory '{0}' does not exist")]
    DirNotFound(String),
}

// ─── Structs ──────────────────────────────────────────────────────────────────

/// Prompt file references for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub identity_file: String,
    pub domain_file: String,
}

/// Optional knowledge-retrieval settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeConfig {
    pub domain_filter: String,
    pub auto_retrieve: bool,
}

/// Tools available to this agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ToolsConfig {
    /// Claude-facing tool names to enable.
    pub include: Vec<String>,
    /// Deterministic function names exposed as tools.
    pub deterministic: Vec<String>,
}


/// Full configuration for a single agent, loaded from a TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub description: String,
    pub model: String,
    /// One of: `"read_only"`, `"workspace_write"`, `"danger_full_access"`.
    pub permission_mode: String,
    pub triggers: TriggerConfig,
    pub prompt: PromptConfig,
    pub knowledge: Option<KnowledgeConfig>,
    #[serde(default)]
    pub tools: ToolsConfig,
}

// ─── TOML wrapper ─────────────────────────────────────────────────────────────

/// TOML files wrap `AgentConfig` under an `[agent]` table.
#[derive(Debug, Deserialize)]
struct AgentConfigFile {
    agent: AgentConfig,
}

// ─── Loader ───────────────────────────────────────────────────────────────────

/// Read and parse all `*.toml` files from `config_dir`.
///
/// Returns the parsed `AgentConfig` list in filesystem-order.
/// Files that fail to parse are returned as errors.
///
/// # Errors
/// Returns [`AgentConfigError`] if the directory cannot be read or a TOML file
/// fails to parse.
pub fn load_agent_configs(config_dir: &Path) -> Result<Vec<AgentConfig>, AgentConfigError> {
    if !config_dir.exists() {
        return Err(AgentConfigError::DirNotFound(
            config_dir.display().to_string(),
        ));
    }

    let mut entries: Vec<_> = std::fs::read_dir(config_dir)
        .map_err(|e| AgentConfigError::Io {
            path: config_dir.display().to_string(),
            source: e,
        })?
        .filter_map(std::result::Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "toml")
        })
        .collect();

    // Sort for deterministic ordering.
    entries.sort_by_key(std::fs::DirEntry::path);

    let mut configs = Vec::with_capacity(entries.len());
    for entry in entries {
        let path = entry.path();
        let path_str = path.display().to_string();
        let content = std::fs::read_to_string(&path).map_err(|e| AgentConfigError::Io {
            path: path_str.clone(),
            source: e,
        })?;
        let file: AgentConfigFile =
            toml::from_str(&content).map_err(|e| AgentConfigError::Toml {
                path: path_str,
                source: e,
            })?;
        configs.push(file.agent);
    }

    Ok(configs)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_toml(dir: &Path, filename: &str, content: &str) {
        let path = dir.join(filename);
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    const MUSIC_TOML: &str = r#"
[agent]
name = "music"
description = "Music collaborator"
model = "claude-sonnet-4-20250514"
permission_mode = "workspace_write"

[agent.triggers]
keywords = ["verse", "hook", "beat"]
priority = 10

[agent.prompt]
identity_file = "config/prompts/identity.md"
domain_file = "config/prompts/music.md"

[agent.knowledge]
domain_filter = "music"
auto_retrieve = true

[agent.tools]
include = ["bash", "read_file", "write_file"]
deterministic = ["bpm_calculator", "chord_lookup"]
"#;

    const GENERAL_TOML: &str = r#"
[agent]
name = "general"
description = "General purpose assistant"
model = "claude-sonnet-4-20250514"
permission_mode = "read_only"

[agent.triggers]
keywords = []
priority = 0

[agent.prompt]
identity_file = "config/prompts/identity.md"
domain_file = "config/prompts/general.md"

[agent.tools]
include = ["read_file"]
deterministic = []
"#;

    #[test]
    fn loads_single_agent_config() {
        let dir = tempdir().unwrap();
        write_toml(dir.path(), "music.toml", MUSIC_TOML);

        let configs = load_agent_configs(dir.path()).unwrap();
        assert_eq!(configs.len(), 1);
        let cfg = &configs[0];
        assert_eq!(cfg.name, "music");
        assert_eq!(cfg.model, "claude-sonnet-4-20250514");
        assert_eq!(cfg.permission_mode, "workspace_write");
        assert_eq!(cfg.triggers.keywords, vec!["verse", "hook", "beat"]);
        assert_eq!(cfg.triggers.priority, 10);
        assert_eq!(cfg.prompt.identity_file, "config/prompts/identity.md");
        assert_eq!(cfg.prompt.domain_file, "config/prompts/music.md");
        assert!(cfg.knowledge.is_some());
        let knowledge = cfg.knowledge.as_ref().unwrap();
        assert_eq!(knowledge.domain_filter, "music");
        assert!(knowledge.auto_retrieve);
        assert_eq!(cfg.tools.include, vec!["bash", "read_file", "write_file"]);
        assert_eq!(cfg.tools.deterministic, vec!["bpm_calculator", "chord_lookup"]);
    }

    #[test]
    fn loads_multiple_configs_sorted() {
        let dir = tempdir().unwrap();
        write_toml(dir.path(), "music.toml", MUSIC_TOML);
        write_toml(dir.path(), "general.toml", GENERAL_TOML);

        let configs = load_agent_configs(dir.path()).unwrap();
        assert_eq!(configs.len(), 2);
        // Sorted alphabetically: general before music
        assert_eq!(configs[0].name, "general");
        assert_eq!(configs[1].name, "music");
    }

    #[test]
    fn ignores_non_toml_files() {
        let dir = tempdir().unwrap();
        write_toml(dir.path(), "music.toml", MUSIC_TOML);
        // Write a non-TOML file
        std::fs::write(dir.path().join("README.md"), "# readme").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "some notes").unwrap();

        let configs = load_agent_configs(dir.path()).unwrap();
        assert_eq!(configs.len(), 1);
    }

    #[test]
    fn returns_error_for_missing_dir() {
        let result = load_agent_configs(Path::new("/nonexistent/path/agents"));
        assert!(matches!(result, Err(AgentConfigError::DirNotFound(_))));
    }

    #[test]
    fn returns_error_for_invalid_toml() {
        let dir = tempdir().unwrap();
        write_toml(dir.path(), "bad.toml", "this is not valid toml [[[");

        let result = load_agent_configs(dir.path());
        assert!(matches!(result, Err(AgentConfigError::Toml { .. })));
    }

    #[test]
    fn agent_without_knowledge_loads_fine() {
        let dir = tempdir().unwrap();
        write_toml(dir.path(), "general.toml", GENERAL_TOML);

        let configs = load_agent_configs(dir.path()).unwrap();
        assert_eq!(configs.len(), 1);
        assert!(configs[0].knowledge.is_none());
    }

    #[test]
    fn tools_default_to_empty_when_omitted() {
        let toml_str = r#"
[agent]
name = "minimal"
description = "Minimal agent"
model = "claude-sonnet-4-20250514"
permission_mode = "read_only"

[agent.triggers]
keywords = []
priority = 0

[agent.prompt]
identity_file = "config/prompts/identity.md"
domain_file = "config/prompts/general.md"
"#;
        let dir = tempdir().unwrap();
        write_toml(dir.path(), "minimal.toml", toml_str);

        let configs = load_agent_configs(dir.path()).unwrap();
        assert_eq!(configs[0].tools.include.len(), 0);
        assert_eq!(configs[0].tools.deterministic.len(), 0);
    }
}
