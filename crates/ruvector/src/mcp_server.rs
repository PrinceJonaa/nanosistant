//! MCP (Model Context Protocol) server interface.
//!
//! This is a stub for v0.1.0.  The MCP tool interface will be added when
//! the MCP integration layer is complete.

use crate::store::VectorStore;

/// MCP server that exposes the knowledge store to MCP-compatible clients.
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

    // MCP tool interface will be added when MCP integration is complete.
}
