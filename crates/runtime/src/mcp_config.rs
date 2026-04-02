use std::collections::BTreeMap;

use crate::config::ConfigSource;

/// OAuth configuration for a remote MCP server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpOAuthConfig {
    pub client_id: Option<String>,
    pub callback_port: Option<u16>,
    pub auth_server_metadata_url: Option<String>,
    pub xaa: Option<bool>,
}

/// Stdio (local process) MCP server config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpStdioServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

/// Remote HTTP/SSE MCP server config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRemoteServerConfig {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub headers_helper: Option<String>,
    pub oauth: Option<McpOAuthConfig>,
}

/// WebSocket MCP server config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpWebSocketServerConfig {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub headers_helper: Option<String>,
}

/// SDK-embedded MCP server config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpSdkServerConfig {
    pub name: String,
}

/// Managed proxy (platform-hosted) MCP server config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpManagedProxyConfig {
    pub url: String,
    pub id: String,
}

/// Discriminated union of all MCP server configuration variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),
    Sse(McpRemoteServerConfig),
    Http(McpRemoteServerConfig),
    Ws(McpWebSocketServerConfig),
    Sdk(McpSdkServerConfig),
    ManagedProxy(McpManagedProxyConfig),
}

/// Transport type tag (for routing / unsupported server reporting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransport {
    Stdio,
    Sse,
    Http,
    WebSocket,
    Sdk,
    ManagedProxy,
}

impl McpServerConfig {
    #[must_use]
    pub fn transport(&self) -> McpTransport {
        match self {
            Self::Stdio(_) => McpTransport::Stdio,
            Self::Sse(_) => McpTransport::Sse,
            Self::Http(_) => McpTransport::Http,
            Self::Ws(_) => McpTransport::WebSocket,
            Self::Sdk(_) => McpTransport::Sdk,
            Self::ManagedProxy(_) => McpTransport::ManagedProxy,
        }
    }
}

/// An MCP server config scoped to a config source (user / project / local).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedMcpServerConfig {
    pub scope: ConfigSource,
    pub config: McpServerConfig,
}

impl ScopedMcpServerConfig {
    #[must_use]
    pub fn transport(&self) -> McpTransport {
        self.config.transport()
    }
}

/// Top-level MCP configuration (a map of named servers).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpConfig {
    servers: BTreeMap<String, ScopedMcpServerConfig>,
}

impl McpConfig {
    #[must_use]
    pub fn new(servers: BTreeMap<String, ScopedMcpServerConfig>) -> Self {
        Self { servers }
    }

    #[must_use]
    pub fn servers(&self) -> &BTreeMap<String, ScopedMcpServerConfig> {
        &self.servers
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }
}
