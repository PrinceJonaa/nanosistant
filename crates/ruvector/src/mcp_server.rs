//! MCP (Model Context Protocol) server for the RuVector knowledge store.
//!
//! Implements a JSON-RPC 2.0 server that exposes the VectorStore as MCP tools.
//! Communicates over stdio (newline-delimited JSON).
//!
//! ## Exposed tools
//!
//! - `ruvector_query`   — keyword search over the knowledge store
//! - `ruvector_ingest`  — ingest a document into the store
//! - `ruvector_domains` — list available domains with chunk counts
//! - `ruvector_stats`   — get overall store statistics

use std::io::{BufRead, BufReader, Write};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ingest::DocumentIngester;
use crate::store::VectorStore;

// ─── JSON-RPC 2.0 types ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl JsonRpcResponse {
    fn ok(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Option<serde_json::Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

// ─── MCP tool argument structs ────────────────────────────────────────────────

#[derive(Deserialize)]
struct QueryArgs {
    query: String,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    max_results: Option<u32>,
}

#[derive(Deserialize)]
struct IngestArgs {
    content: String,
    document_path: String,
    domain: String,
    #[serde(default)]
    doc_type: Option<String>,
}

// ─── McpServer ────────────────────────────────────────────────────────────────

/// MCP server that exposes the `VectorStore` as MCP tools.
///
/// Implements a JSON-RPC 2.0 server communicating over stdio (newline-delimited JSON).
/// Exposes four tools:
/// - `ruvector_query`
/// - `ruvector_ingest`
/// - `ruvector_domains`
/// - `ruvector_stats`
///
/// To use from a `main` function:
/// ```no_run
/// let store = nstn_ruvector::VectorStore::new();
/// let mut server = nstn_ruvector::McpServer::new(store);
/// server.run_stdio().expect("MCP server failed");
/// ```
pub struct McpServer {
    store: VectorStore,
}

impl McpServer {
    /// Create a new MCP server wrapping the given `store`.
    #[must_use]
    pub fn new(store: VectorStore) -> Self {
        Self { store }
    }

    /// Read-only access to the underlying store (useful for inspection/testing).
    #[must_use]
    pub fn store(&self) -> &VectorStore {
        &self.store
    }

    /// Run the MCP server loop on stdin/stdout.
    ///
    /// Reads newline-delimited JSON-RPC 2.0 requests from stdin and writes
    /// responses to stdout.  Blocks until stdin is closed (EOF).
    ///
    /// # Errors
    /// Returns `Err` only if a hard I/O error occurs on stdin or stdout.
    pub fn run_stdio(&mut self) -> std::io::Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(req) => self.handle_request(&req),
                Err(e) => JsonRpcResponse::error(None, -32700, format!("Parse error: {e}")),
            };

            let response_json = serde_json::to_string(&response)?;
            writeln!(stdout, "{response_json}")?;
            stdout.flush()?;
        }
        Ok(())
    }

    // ── Request dispatch ──────────────────────────────────────────────────────

    fn handle_request(&mut self, req: &JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req),
            "notifications/initialized" => {
                // Notification — no response expected.  Return an empty result
                // (callers that send this as a request still get a valid response).
                JsonRpcResponse::ok(req.id.clone(), json!({}))
            }
            "tools/list" => self.handle_tools_list(req),
            "tools/call" => self.handle_tools_call(req),
            _ => JsonRpcResponse::error(
                req.id.clone(),
                -32601,
                format!("Method not found: {}", req.method),
            ),
        }
    }

    // ── MCP method handlers ───────────────────────────────────────────────────

    fn handle_initialize(&self, req: &JsonRpcRequest) -> JsonRpcResponse {
        JsonRpcResponse::ok(
            req.id.clone(),
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "ruvector",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
    }

    fn handle_tools_list(&self, req: &JsonRpcRequest) -> JsonRpcResponse {
        let tools = json!([
            {
                "name": "ruvector_query",
                "description": "Search the RuVector knowledge store using keyword matching.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query text."
                        },
                        "domain": {
                            "type": "string",
                            "description": "Optional domain filter (empty = search all domains)."
                        },
                        "max_results": {
                            "type": "number",
                            "description": "Maximum number of results to return (default 10)."
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "ruvector_ingest",
                "description": "Ingest a document into the RuVector knowledge store.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The document content (markdown)."
                        },
                        "document_path": {
                            "type": "string",
                            "description": "Path or name of the source document (for provenance)."
                        },
                        "domain": {
                            "type": "string",
                            "description": "The semantic domain to tag this document with."
                        },
                        "doc_type": {
                            "type": "string",
                            "description": "Optional freeform type descriptor."
                        }
                    },
                    "required": ["content", "document_path", "domain"]
                }
            },
            {
                "name": "ruvector_domains",
                "description": "List all available domains with their chunk counts.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "ruvector_stats",
                "description": "Get overall knowledge store statistics.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]);

        JsonRpcResponse::ok(req.id.clone(), json!({ "tools": tools }))
    }

    fn handle_tools_call(&mut self, req: &JsonRpcRequest) -> JsonRpcResponse {
        // Extract tool name and arguments from params
        let params = req.params.as_ref().and_then(|p| p.as_object());

        let tool_name = match params.and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
            Some(name) => name.to_string(),
            None => {
                return JsonRpcResponse::error(
                    req.id.clone(),
                    -32602,
                    "Missing 'name' in tools/call params",
                )
            }
        };

        let args = params
            .and_then(|p| p.get("arguments"))
            .cloned()
            .unwrap_or(json!({}));

        match tool_name.as_str() {
            "ruvector_query" => self.tool_query(req, args),
            "ruvector_ingest" => self.tool_ingest(req, args),
            "ruvector_domains" => self.tool_domains(req),
            "ruvector_stats" => self.tool_stats(req),
            _ => JsonRpcResponse::error(
                req.id.clone(),
                -32602,
                format!("Unknown tool: {tool_name}"),
            ),
        }
    }

    // ── Tool implementations ──────────────────────────────────────────────────

    fn tool_query(&self, req: &JsonRpcRequest, args: serde_json::Value) -> JsonRpcResponse {
        let parsed: QueryArgs = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => {
                return JsonRpcResponse::error(
                    req.id.clone(),
                    -32602,
                    format!("Invalid arguments for ruvector_query: {e}"),
                )
            }
        };

        let domain_filter = parsed.domain.as_deref().unwrap_or("");
        let max_results = parsed.max_results.unwrap_or(10);

        let chunks = self.store.query(&parsed.query, domain_filter, max_results);

        let results: Vec<serde_json::Value> = chunks
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "content": c.content,
                    "domain": c.domain,
                    "source_section": c.source_section,
                    "source_document": c.source_document,
                })
            })
            .collect();

        let content = json!([{
            "type": "text",
            "text": serde_json::to_string(&results).unwrap_or_default()
        }]);

        JsonRpcResponse::ok(req.id.clone(), json!({ "content": content }))
    }

    fn tool_ingest(&mut self, req: &JsonRpcRequest, args: serde_json::Value) -> JsonRpcResponse {
        let parsed: IngestArgs = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => {
                return JsonRpcResponse::error(
                    req.id.clone(),
                    -32602,
                    format!("Invalid arguments for ruvector_ingest: {e}"),
                )
            }
        };

        let doc_type = parsed.doc_type.as_deref().unwrap_or("document");

        let chunks =
            DocumentIngester::ingest(&parsed.content, &parsed.document_path, &parsed.domain, doc_type);

        let count = chunks.len();
        for chunk in chunks {
            // Ignore insert errors — the in-memory backend never fails;
            // a Qdrant error is logged by the backend itself.
            let _ = self.store.insert(chunk);
        }

        let result = json!({
            "chunks_ingested": count,
            "success": true
        });

        let content = json!([{
            "type": "text",
            "text": serde_json::to_string(&result).unwrap_or_default()
        }]);

        JsonRpcResponse::ok(req.id.clone(), json!({ "content": content }))
    }

    fn tool_domains(&self, req: &JsonRpcRequest) -> JsonRpcResponse {
        // Build domain → chunk count map by iterating the raw store.
        // We walk all chunks via a zero-max query trick: use query with
        // an empty query string which returns nothing, but we can count
        // via the store's len().  Instead, expose a public helper or just
        // build the map here using the public API we have available.
        //
        // The store doesn't expose an iterator, but we can use the
        // query API to count per domain by querying with a very long
        // max_results and an empty query... except empty queries score 0.
        //
        // Best approach: add a private helper via the store.  Since we
        // own this crate we can use `store.domain_counts()` which we
        // will route through the store module.  For now, we use the
        // approach of collecting domain names from `store.chunks()`.
        //
        // Actually the `VectorStore` field `chunks` is private.  We'll
        // add a `domain_counts` method to VectorStore (this file owns the
        // crate).  We call it below.

        let counts = self.store.domain_counts();

        let domains: Vec<serde_json::Value> = counts
            .iter()
            .map(|(name, count)| {
                json!({
                    "name": name,
                    "chunk_count": count
                })
            })
            .collect();

        let content = json!([{
            "type": "text",
            "text": serde_json::to_string(&domains).unwrap_or_default()
        }]);

        JsonRpcResponse::ok(req.id.clone(), json!({ "content": content }))
    }

    fn tool_stats(&self, req: &JsonRpcRequest) -> JsonRpcResponse {
        let counts = self.store.domain_counts();
        let total: usize = counts.values().sum();

        let domains_json: serde_json::Map<String, serde_json::Value> = counts
            .into_iter()
            .map(|(k, v)| (k, json!(v)))
            .collect();

        let result = json!({
            "total_chunks": total,
            "domains": domains_json
        });

        let content = json!([{
            "type": "text",
            "text": serde_json::to_string(&result).unwrap_or_default()
        }]);

        JsonRpcResponse::ok(req.id.clone(), json!({ "content": content }))
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_server() -> McpServer {
        McpServer::new(VectorStore::new())
    }

    fn call(server: &mut McpServer, method: &str, params: serde_json::Value) -> serde_json::Value {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        };
        let resp = server.handle_request(&req);
        serde_json::to_value(resp).unwrap()
    }

    // ── initialize ────────────────────────────────────────────────────────────

    #[test]
    fn initialize_returns_protocol_version() {
        let mut server = make_server();
        let resp = call(&mut server, "initialize", json!({}));
        assert_eq!(resp["jsonrpc"], "2.0");
        let result = &resp["result"];
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "ruvector");
    }

    #[test]
    fn initialize_returns_tools_capability() {
        let mut server = make_server();
        let resp = call(&mut server, "initialize", json!({}));
        let caps = &resp["result"]["capabilities"];
        assert!(caps.get("tools").is_some(), "capabilities should include tools");
    }

    // ── tools/list ────────────────────────────────────────────────────────────

    #[test]
    fn tools_list_returns_four_tools() {
        let mut server = make_server();
        let resp = call(&mut server, "tools/list", json!({}));
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4, "expected exactly 4 tools");
    }

    #[test]
    fn tools_list_contains_expected_names() {
        let mut server = make_server();
        let resp = call(&mut server, "tools/list", json!({}));
        let tools = resp["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(names.contains(&"ruvector_query"));
        assert!(names.contains(&"ruvector_ingest"));
        assert!(names.contains(&"ruvector_domains"));
        assert!(names.contains(&"ruvector_stats"));
    }

    // ── ruvector_query ────────────────────────────────────────────────────────

    #[test]
    fn query_tool_returns_empty_on_fresh_store() {
        let mut server = make_server();
        let resp = call(
            &mut server,
            "tools/call",
            json!({ "name": "ruvector_query", "arguments": { "query": "anything" } }),
        );
        assert!(resp.get("error").is_none(), "should not error: {resp}");
        // Parse the text content
        let text = &resp["result"]["content"][0]["text"];
        let results: Vec<serde_json::Value> =
            serde_json::from_str(text.as_str().unwrap()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn query_tool_returns_results_after_ingest() {
        let mut server = make_server();

        // Ingest first
        call(
            &mut server,
            "tools/call",
            json!({
                "name": "ruvector_ingest",
                "arguments": {
                    "content": "# Rust\n\n## Ownership\n\nRust ownership rules prevent dangling pointers.",
                    "document_path": "rust.md",
                    "domain": "development"
                }
            }),
        );

        // Then query
        let resp = call(
            &mut server,
            "tools/call",
            json!({ "name": "ruvector_query", "arguments": { "query": "ownership rules" } }),
        );
        assert!(resp.get("error").is_none(), "should not error: {resp}");
        let text = &resp["result"]["content"][0]["text"];
        let results: Vec<serde_json::Value> =
            serde_json::from_str(text.as_str().unwrap()).unwrap();
        assert!(!results.is_empty(), "expected at least one result");
    }

    // ── ruvector_ingest ───────────────────────────────────────────────────────

    #[test]
    fn ingest_tool_returns_chunks_ingested() {
        let mut server = make_server();
        let resp = call(
            &mut server,
            "tools/call",
            json!({
                "name": "ruvector_ingest",
                "arguments": {
                    "content": "# Doc\n\n## Section One\n\nContent one.\n\n## Section Two\n\nContent two.",
                    "document_path": "doc.md",
                    "domain": "framework"
                }
            }),
        );
        assert!(resp.get("error").is_none(), "should not error: {resp}");
        let text = &resp["result"]["content"][0]["text"];
        let result: serde_json::Value = serde_json::from_str(text.as_str().unwrap()).unwrap();
        assert_eq!(result["success"], true);
        // 1 preamble + 2 H2 sections = 3 chunks
        let count = result["chunks_ingested"].as_u64().unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn ingest_tool_updates_store_len() {
        let mut server = make_server();
        assert_eq!(server.store().len(), 0);

        call(
            &mut server,
            "tools/call",
            json!({
                "name": "ruvector_ingest",
                "arguments": {
                    "content": "# Doc\n\n## A\n\nText.",
                    "document_path": "a.md",
                    "domain": "music"
                }
            }),
        );

        // 2 chunks: preamble + section A
        assert_eq!(server.store().len(), 2);
    }

    // ── ruvector_domains ──────────────────────────────────────────────────────

    #[test]
    fn domains_tool_reflects_ingested_domains() {
        let mut server = make_server();

        call(
            &mut server,
            "tools/call",
            json!({
                "name": "ruvector_ingest",
                "arguments": {
                    "content": "# A\n\n## B\n\nFoo.",
                    "document_path": "a.md",
                    "domain": "music"
                }
            }),
        );

        let resp = call(
            &mut server,
            "tools/call",
            json!({ "name": "ruvector_domains", "arguments": {} }),
        );
        assert!(resp.get("error").is_none(), "should not error: {resp}");
        let text = &resp["result"]["content"][0]["text"];
        let domains: Vec<serde_json::Value> =
            serde_json::from_str(text.as_str().unwrap()).unwrap();
        let music_domain = domains.iter().find(|d| d["name"] == "music");
        assert!(music_domain.is_some(), "expected 'music' in domains list");
        assert!(music_domain.unwrap()["chunk_count"].as_u64().unwrap() >= 1);
    }

    // ── ruvector_stats ────────────────────────────────────────────────────────

    #[test]
    fn stats_tool_returns_total_chunks() {
        let mut server = make_server();

        call(
            &mut server,
            "tools/call",
            json!({
                "name": "ruvector_ingest",
                "arguments": {
                    "content": "# Doc\n\n## S1\n\nText.",
                    "document_path": "d.md",
                    "domain": "development"
                }
            }),
        );

        let resp = call(
            &mut server,
            "tools/call",
            json!({ "name": "ruvector_stats", "arguments": {} }),
        );
        assert!(resp.get("error").is_none(), "should not error: {resp}");
        let text = &resp["result"]["content"][0]["text"];
        let stats: serde_json::Value = serde_json::from_str(text.as_str().unwrap()).unwrap();
        let total = stats["total_chunks"].as_u64().unwrap();
        assert!(total >= 1);
        assert!(stats["domains"]["development"].as_u64().unwrap() >= 1);
    }

    // ── unknown method ────────────────────────────────────────────────────────

    #[test]
    fn unknown_method_returns_method_not_found_error() {
        let mut server = make_server();
        let resp = call(&mut server, "unknown/method", json!({}));
        assert!(resp.get("error").is_some(), "expected error for unknown method");
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn unknown_tool_returns_error() {
        let mut server = make_server();
        let resp = call(
            &mut server,
            "tools/call",
            json!({ "name": "nonexistent_tool", "arguments": {} }),
        );
        assert!(resp.get("error").is_some(), "expected error for unknown tool");
    }

    // ── notifications/initialized ─────────────────────────────────────────────

    #[test]
    fn notifications_initialized_returns_empty_result() {
        let mut server = make_server();
        let resp = call(&mut server, "notifications/initialized", json!({}));
        assert!(resp.get("error").is_none());
    }
}
