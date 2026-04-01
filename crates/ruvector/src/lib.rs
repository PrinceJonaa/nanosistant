//! `nstn-ruvector` — Knowledge-tier crate for the Nanosistant project.
//!
//! Provides:
//! - [`store`]: In-memory vector store with keyword-based retrieval.
//! - [`ingest`]: Document chunking and ingestion (splits on `## ` headers).
//! - [`mcp_server`]: MCP server stub (full interface pending MCP integration).
//! - [`grpc_server`]: gRPC service stub backed by the vector store.

pub mod grpc_server;
pub mod ingest;
pub mod mcp_server;
pub mod store;

pub use grpc_server::RuVectorGrpcService;
pub use ingest::DocumentIngester;
pub use mcp_server::McpServer;
pub use store::{KnowledgeChunk, StoredChunk, VectorStore};
