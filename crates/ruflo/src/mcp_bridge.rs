//! MCP stdio bridge — spawns ruflo's MCP server and communicates via JSON-RPC 2.0.
//!
//! This is the transport layer between the Rust orchestrator and the ruflo
//! TypeScript runtime. Ruflo runs as a child process; we write JSON-RPC
//! requests to its stdin and read responses from its stdout (newline-delimited).
//!
//! The bridge follows the same pattern as ruflo's own `StdioMcpClient`
//! (see ADR-033: RuVector + Ruflo MCP Integration).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ═══════════════════════════════════════
// JSON-RPC 2.0 types
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// ═══════════════════════════════════════
// MCP tool types
// ═══════════════════════════════════════

/// An MCP tool discovered from ruflo.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Option<serde_json::Value>,
}

/// Result of an MCP tool call.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

// ═══════════════════════════════════════
// Errors
// ═══════════════════════════════════════

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("ruflo process not running")]
    NotRunning,

    #[error("failed to spawn ruflo: {0}")]
    SpawnFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("JSON-RPC error {code}: {message}")]
    JsonRpc { code: i32, message: String },

    #[error("request timed out after {0}ms")]
    Timeout(u64),

    #[error("ruflo initialization failed: {0}")]
    InitFailed(String),
}

// ═══════════════════════════════════════
// Bridge configuration
// ═══════════════════════════════════════

/// Configuration for the MCP bridge to ruflo.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Path to the ruflo v3 directory (default: vendor/ruflo/v3).
    pub ruflo_dir: String,
    /// Command to start the MCP server (default: npx tsx mcp/server-entry.ts).
    pub start_command: String,
    /// Arguments to the start command.
    pub start_args: Vec<String>,
    /// Request timeout in milliseconds (default: 30_000).
    pub timeout_ms: u64,
    /// Whether to auto-discover tools on connect (default: true).
    pub auto_discover: bool,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            ruflo_dir: "vendor/ruflo/v3".to_string(),
            start_command: "npx".to_string(),
            start_args: vec![
                "tsx".to_string(),
                "mcp/server-entry.ts".to_string(),
                "--transport".to_string(),
                "stdio".to_string(),
            ],
            timeout_ms: 30_000,
            auto_discover: true,
        }
    }
}

// ═══════════════════════════════════════
// MCP Bridge
// ═══════════════════════════════════════

/// The MCP bridge manages a ruflo child process and provides
/// typed JSON-RPC communication over stdio.
pub struct McpBridge {
    config: BridgeConfig,
    child: Option<Child>,
    next_id: AtomicU64,
    /// Discovered tools from ruflo (name → tool).
    tools: HashMap<String, McpTool>,
    /// Whether the bridge has been initialized.
    initialized: bool,
}

impl McpBridge {
    /// Create a new bridge with the given config.
    /// Does NOT start the ruflo process — call `start()` for that.
    #[must_use]
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            config,
            child: None,
            next_id: AtomicU64::new(1),
            tools: HashMap::new(),
            initialized: false,
        }
    }

    /// Create a bridge that operates in offline/stub mode.
    /// All calls return `BridgeError::NotRunning`. Useful for
    /// environments where Node.js/ruflo isn't available.
    #[must_use]
    pub fn offline() -> Self {
        Self {
            config: BridgeConfig::default(),
            child: None,
            next_id: AtomicU64::new(1),
            tools: HashMap::new(),
            initialized: false,
        }
    }

    /// Start the ruflo MCP server as a child process.
    pub fn start(&mut self) -> Result<(), BridgeError> {
        if self.child.is_some() {
            return Ok(()); // Already running
        }

        let child = Command::new(&self.config.start_command)
            .args(&self.config.start_args)
            .current_dir(&self.config.ruflo_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| BridgeError::SpawnFailed(e.to_string()))?;

        self.child = Some(child);

        // Initialize the MCP connection
        self.mcp_initialize()?;

        // Auto-discover tools if configured
        if self.config.auto_discover {
            self.discover_tools()?;
        }

        self.initialized = true;
        tracing::info!(
            tools = self.tools.len(),
            "ruflo MCP bridge started and initialized"
        );

        Ok(())
    }

    /// Stop the ruflo process gracefully.
    pub fn stop(&mut self) -> Result<(), BridgeError> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            self.initialized = false;
            self.tools.clear();
            tracing::info!("ruflo MCP bridge stopped");
        }
        Ok(())
    }

    /// Whether the bridge is connected and initialized.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.initialized && self.child.is_some()
    }

    /// Get discovered tools.
    #[must_use]
    pub fn tools(&self) -> &HashMap<String, McpTool> {
        &self.tools
    }

    /// Call an MCP tool by name with the given arguments.
    pub fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, BridgeError> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments,
        });

        let response = self.send_request("tools/call", Some(params))?;

        serde_json::from_value(response).map_err(BridgeError::Json)
    }

    /// Send a raw JSON-RPC request and get the response.
    pub fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, BridgeError> {
        let child = self.child.as_mut().ok_or(BridgeError::NotRunning)?;

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        // Write request to stdin (newline-delimited JSON)
        let stdin = child.stdin.as_mut().ok_or(BridgeError::NotRunning)?;
        let request_json = serde_json::to_string(&request)?;
        writeln!(stdin, "{request_json}")?;
        stdin.flush()?;

        // Read response from stdout
        let stdout = child.stdout.as_mut().ok_or(BridgeError::NotRunning)?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        if line.trim().is_empty() {
            return Err(BridgeError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "empty response from ruflo",
            )));
        }

        let response: JsonRpcResponse = serde_json::from_str(line.trim())?;

        if let Some(error) = response.error {
            return Err(BridgeError::JsonRpc {
                code: error.code,
                message: error.message,
            });
        }

        response
            .result
            .ok_or_else(|| BridgeError::JsonRpc {
                code: -1,
                message: "empty result".to_string(),
            })
    }

    // ── Private helpers ──

    /// Send the MCP initialize handshake.
    fn mcp_initialize(&mut self) -> Result<(), BridgeError> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "nanosistant-rust",
                "version": "0.1.0"
            }
        });

        let result = self.send_request("initialize", Some(params))?;

        // Verify the server responded with capabilities
        if result.get("capabilities").is_none() && result.get("serverInfo").is_none() {
            return Err(BridgeError::InitFailed(
                "missing capabilities in initialize response".to_string(),
            ));
        }

        // Send initialized notification (no response expected for notifications,
        // but we send it as a request for simplicity in the stdio bridge)
        let _ = self.send_request("notifications/initialized", None);

        Ok(())
    }

    /// Discover available tools from ruflo.
    fn discover_tools(&mut self) -> Result<(), BridgeError> {
        let result = self.send_request("tools/list", None)?;

        #[derive(Deserialize)]
        struct ToolsListResult {
            tools: Vec<McpTool>,
        }

        let tools_result: ToolsListResult = serde_json::from_value(result)?;
        self.tools.clear();
        for tool in tools_result.tools {
            self.tools.insert(tool.name.clone(), tool);
        }

        Ok(())
    }
}

impl Drop for McpBridge {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_bridge_returns_not_running() {
        let mut bridge = McpBridge::offline();
        assert!(!bridge.is_running());
        assert!(bridge.tools().is_empty());

        let result = bridge.call_tool("test", serde_json::json!({}));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BridgeError::NotRunning));
    }

    #[test]
    fn json_rpc_request_serializes_correctly() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "tools/list".to_string(),
            params: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"tools/list\""));
    }

    #[test]
    fn json_rpc_response_deserializes_correctly() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, Some(1));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn json_rpc_error_response_deserializes_correctly() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32600);
        assert_eq!(err.message, "Invalid Request");
    }

    #[test]
    fn bridge_config_defaults_are_reasonable() {
        let config = BridgeConfig::default();
        assert_eq!(config.ruflo_dir, "vendor/ruflo/v3");
        assert_eq!(config.start_command, "npx");
        assert_eq!(config.timeout_ms, 30_000);
        assert!(config.auto_discover);
    }

    #[test]
    fn mcp_tool_result_deserializes() {
        let json = r#"{"content":[{"type":"text","text":"hello"}],"isError":false}"#;
        let result: McpToolResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].text.as_deref(), Some("hello"));
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn stop_on_offline_bridge_is_noop() {
        let mut bridge = McpBridge::offline();
        assert!(bridge.stop().is_ok());
    }
}
