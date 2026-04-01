//! Vector store with pluggable backends.
//!
//! # v0.2 changes
//! - [`VectorBackend`] trait allowing multiple storage engines.
//! - [`InMemoryBackend`]: refactored from the v0.1 `VectorStore` with the
//!   same keyword (TF-IDF-style) query plus cosine-similarity for embedding
//!   queries.
//! - [`VectorStore`]: thin wrapper that owns a `Box<dyn VectorBackend>`.
//! - [`StoredChunk`]: gained an optional `embedding` field.

use std::collections::HashMap;

use crate::embeddings::cosine_similarity;
use crate::qdrant::QdrantBackend;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("backend error: {0}")]
    Backend(String),
    #[error("embedding error: {0}")]
    Embedding(String),
}

// ── Public data types ─────────────────────────────────────────────────────────

/// A knowledge chunk returned from a query.
#[derive(Debug, Clone)]
pub struct KnowledgeChunk {
    pub id: String,
    pub content: String,
    pub domain: String,
    pub source_section: String,
    pub source_document: String,
    pub metadata: HashMap<String, String>,
}

/// A single document chunk held in the store.
#[derive(Debug, Clone)]
pub struct StoredChunk {
    pub id: String,
    pub content: String,
    pub domain: String,
    pub source_section: String,
    pub source_document: String,
    pub metadata: HashMap<String, String>,
    /// Optional dense embedding vector for semantic similarity search.
    pub embedding: Option<Vec<f32>>,
}

impl StoredChunk {
    /// Rough TF-IDF-style relevance score against a query.
    ///
    /// Counts query tokens that appear in the chunk content (case-insensitive).
    #[must_use]
    fn score(&self, query_tokens: &[&str]) -> f64 {
        if query_tokens.is_empty() {
            return 0.0;
        }
        let content_lower = self.content.to_lowercase();
        let hits = query_tokens
            .iter()
            .filter(|t| content_lower.contains(*t))
            .count();
        #[allow(clippy::cast_precision_loss)]
        let score = hits as f64 / query_tokens.len() as f64;
        score
    }
}

// ── VectorBackend trait ───────────────────────────────────────────────────────

/// Backend trait for vector storage.
///
/// Implementors: [`InMemoryBackend`], [`QdrantBackend`].
pub trait VectorBackend: Send + Sync {
    /// Store a chunk.
    fn insert(&mut self, chunk: StoredChunk) -> Result<(), StoreError>;

    /// Keyword-based text query.
    fn query(&self, query_text: &str, domain_filter: &str, max_results: u32) -> Vec<KnowledgeChunk>;

    /// Semantic query using a pre-computed embedding.
    fn query_by_embedding(
        &self,
        embedding: &[f32],
        domain_filter: &str,
        max_results: u32,
    ) -> Vec<KnowledgeChunk>;

    /// Number of chunks stored (may be approximate for remote backends).
    fn len(&self) -> usize;

    /// Returns `true` when the store contains no chunks.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return a map of domain name → chunk count.
    ///
    /// Default implementation returns an empty map (remote backends may not
    /// support this cheaply).
    fn domain_counts(&self) -> HashMap<String, usize> {
        HashMap::new()
    }
}

// ── InMemoryBackend ───────────────────────────────────────────────────────────

/// In-process in-memory backend.
///
/// Uses the same keyword (TF-IDF-style) logic that existed in v0.1, plus
/// cosine-similarity search over stored embeddings.
#[derive(Debug, Default)]
pub struct InMemoryBackend {
    chunks: Vec<StoredChunk>,
}

impl InMemoryBackend {
    /// Create an empty backend.
    #[must_use]
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }
}

impl VectorBackend for InMemoryBackend {
    fn insert(&mut self, chunk: StoredChunk) -> Result<(), StoreError> {
        self.chunks.push(chunk);
        Ok(())
    }

    fn query(&self, query_text: &str, domain_filter: &str, max_results: u32) -> Vec<KnowledgeChunk> {
        let query_lower = query_text.to_lowercase();
        let tokens: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|t| t.len() > 2)
            .collect();

        let mut scored: Vec<(&StoredChunk, f64)> = self
            .chunks
            .iter()
            .filter(|c| domain_filter.is_empty() || c.domain == domain_filter)
            .map(|c| {
                let s = c.score(&tokens);
                (c, s)
            })
            .filter(|(_, s)| *s > 0.0)
            .collect();

        // Sort descending by score, then by id for deterministic ordering.
        scored.sort_by(|(a, sa), (b, sb)| {
            sb.partial_cmp(sa)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        scored
            .into_iter()
            .take(max_results as usize)
            .map(|(c, _score)| KnowledgeChunk {
                id: c.id.clone(),
                content: c.content.clone(),
                domain: c.domain.clone(),
                source_section: c.source_section.clone(),
                source_document: c.source_document.clone(),
                metadata: c.metadata.clone(),
            })
            .collect()
    }

    fn query_by_embedding(
        &self,
        embedding: &[f32],
        domain_filter: &str,
        max_results: u32,
    ) -> Vec<KnowledgeChunk> {
        let mut scored: Vec<(&StoredChunk, f32)> = self
            .chunks
            .iter()
            .filter(|c| domain_filter.is_empty() || c.domain == domain_filter)
            .filter_map(|c| {
                c.embedding.as_ref().map(|emb| {
                    if emb.len() == embedding.len() {
                        let sim = cosine_similarity(embedding, emb);
                        (c, sim)
                    } else {
                        (c, 0.0_f32)
                    }
                })
            })
            .filter(|(_, s)| *s > 0.0)
            .collect();

        scored.sort_by(|(a, sa), (b, sb)| {
            sb.partial_cmp(sa)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        scored
            .into_iter()
            .take(max_results as usize)
            .map(|(c, _score)| KnowledgeChunk {
                id: c.id.clone(),
                content: c.content.clone(),
                domain: c.domain.clone(),
                source_section: c.source_section.clone(),
                source_document: c.source_document.clone(),
                metadata: c.metadata.clone(),
            })
            .collect()
    }

    fn len(&self) -> usize {
        self.chunks.len()
    }

    fn domain_counts(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for chunk in &self.chunks {
            *counts.entry(chunk.domain.clone()).or_insert(0) += 1;
        }
        counts
    }
}

// ── VectorStore ───────────────────────────────────────────────────────────────

/// Thin wrapper around a [`VectorBackend`].
///
/// Use [`VectorStore::in_memory()`] for the default in-process store, or
/// [`VectorStore::qdrant()`] to connect to an external Qdrant instance.
pub struct VectorStore {
    backend: Box<dyn VectorBackend>,
}

impl std::fmt::Debug for VectorStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VectorStore").finish_non_exhaustive()
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::in_memory()
    }
}

impl VectorStore {
    /// Create a store backed by the in-memory backend.
    #[must_use]
    pub fn in_memory() -> Self {
        Self {
            backend: Box::new(InMemoryBackend::new()),
        }
    }

    /// Create a store backed by a Qdrant instance.
    ///
    /// Initialises the collection with a 0-sized placeholder; the caller
    /// should call `initialize` on the backend before inserting real vectors.
    pub fn qdrant(url: &str, collection: &str) -> Result<Self, StoreError> {
        let backend = QdrantBackend::new(url, collection);
        Ok(Self {
            backend: Box::new(backend),
        })
    }

    /// Create an empty store (identical to [`in_memory()`][Self::in_memory]).
    ///
    /// Provided for backwards compatibility with v0.1 call sites.
    #[must_use]
    pub fn new() -> Self {
        Self::in_memory()
    }

    /// Insert a chunk.
    ///
    /// # Errors
    ///
    /// Propagates [`StoreError`] from the backend.
    pub fn insert(&mut self, chunk: StoredChunk) -> Result<(), StoreError> {
        self.backend.insert(chunk)
    }

    /// Convenience insert that never fails (panics on error).
    ///
    /// Provided so existing test code that calls `store.insert(chunk)` without
    /// checking the result continues to compile after the signature change.
    pub fn insert_unchecked(&mut self, chunk: StoredChunk) {
        self.backend.insert(chunk).expect("insert failed");
    }

    /// Keyword query.
    #[must_use]
    pub fn query(&self, query_text: &str, domain_filter: &str, max_results: u32) -> Vec<KnowledgeChunk> {
        self.backend.query(query_text, domain_filter, max_results)
    }

    /// Semantic query using a pre-computed embedding.
    #[must_use]
    pub fn query_by_embedding(
        &self,
        embedding: &[f32],
        domain_filter: &str,
        max_results: u32,
    ) -> Vec<KnowledgeChunk> {
        self.backend
            .query_by_embedding(embedding, domain_filter, max_results)
    }

    /// Number of stored chunks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.backend.len()
    }

    /// `true` when the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.backend.is_empty()
    }

    /// Domain name → chunk count map.
    #[must_use]
    pub fn domain_counts(&self) -> HashMap<String, usize> {
        self.backend.domain_counts()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk(id: &str, content: &str, domain: &str) -> StoredChunk {
        StoredChunk {
            id: id.to_owned(),
            content: content.to_owned(),
            domain: domain.to_owned(),
            source_section: "sec".to_owned(),
            source_document: "doc.md".to_owned(),
            metadata: HashMap::new(),
            embedding: None,
        }
    }

    fn make_chunk_with_embedding(id: &str, content: &str, domain: &str, emb: Vec<f32>) -> StoredChunk {
        StoredChunk {
            embedding: Some(emb),
            ..make_chunk(id, content, domain)
        }
    }

    // ── VectorStore (in_memory) backward-compat tests ─────────────────────

    #[test]
    fn empty_store_returns_empty() {
        let store = VectorStore::new();
        let results = store.query("anything", "", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn keyword_matching_returns_relevant_results() {
        let mut store = VectorStore::new();
        store.insert(make_chunk("1", "the C major scale has seven notes", "music")).unwrap();
        store.insert(make_chunk("2", "BPM is beats per minute", "music")).unwrap();
        store.insert(make_chunk("3", "rust ownership and borrowing rules", "dev")).unwrap();

        let results = store.query("major scale notes", "music", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "1");
    }

    #[test]
    fn domain_filter_excludes_other_domains() {
        let mut store = VectorStore::new();
        store.insert(make_chunk("1", "scale and notes theory", "music")).unwrap();
        store.insert(make_chunk("2", "scale your application infrastructure", "dev")).unwrap();

        let results = store.query("scale", "music", 10);
        assert!(results.iter().all(|r| r.domain == "music"));
    }

    #[test]
    fn max_results_is_respected() {
        let mut store = VectorStore::new();
        for i in 0..20_u32 {
            store
                .insert(make_chunk(
                    &i.to_string(),
                    "keyword matching test content with search terms",
                    "framework",
                ))
                .unwrap();
        }
        let results = store.query("keyword matching", "framework", 5);
        assert!(results.len() <= 5);
    }

    // ── InMemoryBackend direct tests ──────────────────────────────────────

    #[test]
    fn in_memory_backend_insert_and_len() {
        let mut backend = InMemoryBackend::new();
        assert!(backend.is_empty());
        backend
            .insert(make_chunk("a", "hello world", "test"))
            .unwrap();
        assert_eq!(backend.len(), 1);
    }

    #[test]
    fn in_memory_backend_query_returns_match() {
        let mut backend = InMemoryBackend::new();
        backend
            .insert(make_chunk("1", "rust programming language features", "dev"))
            .unwrap();
        backend
            .insert(make_chunk("2", "python scripting and automation", "dev"))
            .unwrap();

        let results = backend.query("rust language", "dev", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "1");
    }

    // ── Embedding-based query tests ───────────────────────────────────────

    #[test]
    fn query_by_embedding_returns_cosine_closest() {
        let mut store = VectorStore::in_memory();
        // Two chunks with embeddings: first is [1,0], second is [0,1].
        store
            .insert(make_chunk_with_embedding("near", "near doc", "t", vec![1.0, 0.0]))
            .unwrap();
        store
            .insert(make_chunk_with_embedding("far", "far doc", "t", vec![0.0, 1.0]))
            .unwrap();

        // Query with [0.9, 0.1] — closer to [1,0].
        let results = store.query_by_embedding(&[0.9, 0.1], "t", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "near");
    }

    #[test]
    fn query_by_embedding_empty_when_no_embeddings_stored() {
        let mut store = VectorStore::in_memory();
        store.insert(make_chunk("1", "content here", "d")).unwrap();
        // No embeddings stored — should return empty rather than panic.
        let results = store.query_by_embedding(&[1.0, 0.0], "d", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn query_by_embedding_respects_domain_filter() {
        let mut store = VectorStore::in_memory();
        store
            .insert(make_chunk_with_embedding("music", "music note", "music", vec![1.0, 0.0]))
            .unwrap();
        store
            .insert(make_chunk_with_embedding("dev", "code stuff", "dev", vec![1.0, 0.0]))
            .unwrap();

        let results = store.query_by_embedding(&[1.0, 0.0], "music", 5);
        assert!(results.iter().all(|r| r.domain == "music"));
    }

    // ── VectorStore::in_memory() works like ::new() ───────────────────────

    #[test]
    fn in_memory_constructor_identical_to_new() {
        let mut a = VectorStore::in_memory();
        let mut b = VectorStore::new();
        a.insert(make_chunk("1", "hello world test", "x")).unwrap();
        b.insert(make_chunk("1", "hello world test", "x")).unwrap();
        let ra = a.query("hello world", "", 5);
        let rb = b.query("hello world", "", 5);
        assert_eq!(ra.len(), rb.len());
    }
}
