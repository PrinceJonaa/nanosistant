//! `nstn-ruvector` — Knowledge-tier crate for the Nanosistant project.
//!
//! Provides:
//! - [`embeddings`]: Embedding trait, `HashEmbedding`, and cosine similarity.
//! - [`store`]: Vector store with pluggable backends (in-memory, Qdrant).
//! - [`qdrant`]: Qdrant HTTP REST backend.
//! - [`ingest`]: Document chunking and ingestion (splits on `## ` headers).
//! - [`mcp_server`]: MCP server stub (full interface pending MCP integration).
//! - [`grpc_server`]: gRPC service stub backed by the vector store.

pub mod embeddings;
pub mod grpc_server;
pub mod ingest;
pub mod mcp_server;
pub mod pipeline;
pub mod qdrant;
pub mod store;

pub use embeddings::{cosine_similarity, EmbeddingError, EmbeddingProvider, HashEmbedding};
pub use grpc_server::RuVectorGrpcService;
pub use ingest::DocumentIngester;
pub use mcp_server::McpServer;
pub use qdrant::QdrantBackend;
pub use pipeline::{IngestionPipeline, IngestionResult, IngestionSource};
pub use store::{InMemoryBackend, KnowledgeChunk, StoreError, StoredChunk, VectorBackend, VectorStore};
