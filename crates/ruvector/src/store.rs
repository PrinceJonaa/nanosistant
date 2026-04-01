//! In-memory vector store.
//!
//! For v0.1.0 queries use simple keyword (TF-IDF-like) matching.
//! Real embedding similarity will be added when Qdrant is integrated.

use std::collections::HashMap;

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
    // In production an embedding vector would go here:
    // pub embedding: Vec<f32>,
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

/// In-memory store of document chunks.
#[derive(Debug, Default)]
pub struct VectorStore {
    chunks: Vec<StoredChunk>,
}

impl VectorStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Insert a chunk into the store.
    pub fn insert(&mut self, chunk: StoredChunk) {
        self.chunks.push(chunk);
    }

    /// Query the store with keyword matching.
    ///
    /// Applies `domain_filter` (empty string = no filter), scores each chunk,
    /// and returns up to `max_results` chunks in descending score order.
    #[must_use]
    pub fn query(
        &self,
        query_text: &str,
        domain_filter: &str,
        max_results: u32,
    ) -> Vec<KnowledgeChunk> {
        let query_lower = query_text.to_lowercase();
        let tokens: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|t| t.len() > 2) // skip very short tokens
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

    /// Number of chunks currently in the store.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Returns `true` when the store contains no chunks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}

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
        }
    }

    #[test]
    fn empty_store_returns_empty() {
        let store = VectorStore::new();
        let results = store.query("anything", "", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn keyword_matching_returns_relevant_results() {
        let mut store = VectorStore::new();
        store.insert(make_chunk("1", "the C major scale has seven notes", "music"));
        store.insert(make_chunk("2", "BPM is beats per minute", "music"));
        store.insert(make_chunk("3", "rust ownership and borrowing rules", "dev"));

        let results = store.query("major scale notes", "music", 5);
        assert!(!results.is_empty());
        // The most relevant chunk should be about the C major scale.
        assert_eq!(results[0].id, "1");
    }

    #[test]
    fn domain_filter_excludes_other_domains() {
        let mut store = VectorStore::new();
        store.insert(make_chunk("1", "scale and notes theory", "music"));
        store.insert(make_chunk("2", "scale your application infrastructure", "dev"));

        let results = store.query("scale", "music", 10);
        assert!(results.iter().all(|r| r.domain == "music"));
    }

    #[test]
    fn max_results_is_respected() {
        let mut store = VectorStore::new();
        for i in 0..20_u32 {
            store.insert(make_chunk(
                &i.to_string(),
                "keyword matching test content with search terms",
                "framework",
            ));
        }
        let results = store.query("keyword matching", "framework", 5);
        assert!(results.len() <= 5);
    }
}
