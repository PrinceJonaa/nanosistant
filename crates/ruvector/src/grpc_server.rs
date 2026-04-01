//! gRPC service stub for the `RuVector` knowledge tier.
//!
//! For v0.1.0 the service wraps the `VectorStore` directly.  Full tonic
//! service trait implementations will be added when the proto codegen is
//! wired up for this crate.

use crate::store::{KnowledgeChunk, VectorStore};

/// gRPC service that answers knowledge queries using the in-memory store.
pub struct RuVectorGrpcService {
    store: VectorStore,
}

impl RuVectorGrpcService {
    /// Create a new service wrapping the given `store`.
    #[must_use]
    pub fn new(store: VectorStore) -> Self {
        Self { store }
    }

    /// Query the underlying store directly (used for testing / internal calls).
    #[must_use]
    pub fn query(
        &self,
        query_text: &str,
        domain_filter: &str,
        max_results: u32,
    ) -> Vec<KnowledgeChunk> {
        self.store.query(query_text, domain_filter, max_results)
    }

    /// Read-only access to the underlying store.
    #[must_use]
    pub fn store(&self) -> &VectorStore {
        &self.store
    }
}
