//! Document chunking and ingestion.
//!
//! Splits markdown documents on `## ` headers (H2) and tags each
//! resulting chunk with domain metadata.

use std::collections::HashMap;

use uuid::Uuid;

use crate::embeddings::EmbeddingProvider;
use crate::store::StoredChunk;

/// Stateless document ingester.
pub struct DocumentIngester;

impl DocumentIngester {
    /// Ingest `content`, splitting on `## ` headers.
    ///
    /// Each resulting chunk is tagged with the supplied `domain` and
    /// `document_path`, and receives a fresh UUID as its identifier.
    ///
    /// # Arguments
    ///
    /// * `content`       — Raw markdown text.
    /// * `document_path` — Path or name of the source document (for provenance).
    /// * `domain`        — Semantic domain tag (e.g. `"music"`, `"framework"`,
    ///                     `"development"`).
    /// * `doc_type`      — Freeform type descriptor stored in metadata.
    #[must_use]
    pub fn ingest(
        content: &str,
        document_path: &str,
        domain: &str,
        doc_type: &str,
    ) -> Vec<StoredChunk> {
        Self::ingest_with_embeddings(content, document_path, domain, doc_type, None)
    }

    /// Ingest `content` and optionally generate embeddings for each chunk.
    ///
    /// When `embedder` is `Some`, each chunk's `embedding` field is populated
    /// by calling [`EmbeddingProvider::embed`] on the chunk content.  Embedding
    /// errors are logged as warnings and do not abort ingestion — the chunk is
    /// stored with `embedding: None` instead.
    ///
    /// # Arguments
    ///
    /// * `content`       — Raw markdown text.
    /// * `document_path` — Path or name of the source document.
    /// * `domain`        — Semantic domain tag.
    /// * `doc_type`      — Freeform type descriptor stored in metadata.
    /// * `embedder`      — Optional embedding provider.
    #[must_use]
    pub fn ingest_with_embeddings(
        content: &str,
        document_path: &str,
        domain: &str,
        doc_type: &str,
        embedder: Option<&dyn EmbeddingProvider>,
    ) -> Vec<StoredChunk> {
        // Split the document into sections using `## ` as the delimiter.
        // We keep `## ` at the start of each section so the header is visible.
        let raw_sections: Vec<&str> = content.split("\n## ").collect();

        let mut chunks = Vec::new();

        for (idx, section) in raw_sections.iter().enumerate() {
            // Restore the `## ` prefix that was consumed by the split (except
            // for the very first segment, which precedes the first `## `).
            let section_text = if idx == 0 {
                section.trim().to_owned()
            } else {
                format!("## {}", section.trim())
            };

            if section_text.is_empty() {
                continue;
            }

            // Extract section heading (first line).
            let first_line = section_text.lines().next().unwrap_or("").trim();
            let section_title = first_line
                .trim_start_matches("## ")
                .trim_start_matches('#')
                .trim()
                .to_owned();

            let source_section = if section_title.is_empty() {
                format!("section-{idx}")
            } else {
                section_title
            };

            let mut metadata = HashMap::new();
            metadata.insert("doc_type".to_owned(), doc_type.to_owned());
            metadata.insert("section_index".to_owned(), idx.to_string());

            // Generate embedding if a provider was supplied.
            let embedding = embedder.and_then(|emb| {
                match emb.embed(&section_text) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!("embedding failed for section '{source_section}': {e}");
                        None
                    }
                }
            });

            chunks.push(StoredChunk {
                id: Uuid::new_v4().to_string(),
                content: section_text,
                domain: domain.to_owned(),
                source_section,
                source_document: document_path.to_owned(),
                metadata,
                embedding,
            });
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::HashEmbedding;

    const SAMPLE_DOC: &str = r#"# Introduction

This is the intro paragraph.

## Getting Started

Install the framework using the standard toolchain.

## Configuration

Set the environment variables before running.

## Advanced Usage

For power users who need custom pipelines.
"#;

    #[test]
    fn ingest_splits_on_h2_headers() {
        let chunks = DocumentIngester::ingest(SAMPLE_DOC, "guide.md", "framework", "docs");
        // Should produce 4 chunks: preamble + 3 sections.
        assert_eq!(chunks.len(), 4, "expected 4 chunks (preamble + 3 H2 sections)");
    }

    #[test]
    fn ingest_tags_domain_correctly() {
        let chunks = DocumentIngester::ingest(SAMPLE_DOC, "guide.md", "music", "lens");
        assert!(chunks.iter().all(|c| c.domain == "music"));
    }

    #[test]
    fn ingest_extracts_section_headings() {
        let chunks = DocumentIngester::ingest(SAMPLE_DOC, "guide.md", "framework", "docs");
        let sections: Vec<&str> = chunks.iter().map(|c| c.source_section.as_str()).collect();
        assert!(sections.contains(&"Getting Started"), "missing 'Getting Started'");
        assert!(sections.contains(&"Configuration"), "missing 'Configuration'");
        assert!(sections.contains(&"Advanced Usage"), "missing 'Advanced Usage'");
    }

    #[test]
    fn ingest_stores_document_path() {
        let chunks = DocumentIngester::ingest(SAMPLE_DOC, "path/to/doc.md", "development", "code");
        assert!(chunks.iter().all(|c| c.source_document == "path/to/doc.md"));
    }

    #[test]
    fn ingest_empty_content_returns_no_chunks() {
        let chunks = DocumentIngester::ingest("", "empty.md", "framework", "docs");
        assert!(chunks.is_empty());
    }

    #[test]
    fn ingest_doc_with_no_h2_headers_produces_single_chunk() {
        let content = "# Title\n\nJust a paragraph, no H2 sections here.";
        let chunks = DocumentIngester::ingest(content, "flat.md", "development", "docs");
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn ingest_metadata_contains_doc_type() {
        let chunks = DocumentIngester::ingest(SAMPLE_DOC, "guide.md", "framework", "tutorial");
        for chunk in &chunks {
            assert_eq!(chunk.metadata.get("doc_type").map(String::as_str), Some("tutorial"));
        }
    }

    #[test]
    fn ingest_without_embedder_produces_none_embeddings() {
        let chunks = DocumentIngester::ingest(SAMPLE_DOC, "guide.md", "framework", "docs");
        assert!(chunks.iter().all(|c| c.embedding.is_none()));
    }

    #[test]
    fn ingest_with_embeddings_populates_embedding_field() {
        let embedder = HashEmbedding::new(64);
        let chunks = DocumentIngester::ingest_with_embeddings(
            SAMPLE_DOC,
            "guide.md",
            "framework",
            "docs",
            Some(&embedder),
        );
        assert_eq!(chunks.len(), 4);
        for chunk in &chunks {
            let emb = chunk.embedding.as_ref().expect("embedding should be set");
            assert_eq!(emb.len(), 64);
        }
    }

    #[test]
    fn ingest_with_embeddings_consistent_for_same_content() {
        let embedder = HashEmbedding::new(32);
        let chunks_a = DocumentIngester::ingest_with_embeddings(
            SAMPLE_DOC,
            "guide.md",
            "framework",
            "docs",
            Some(&embedder),
        );
        let chunks_b = DocumentIngester::ingest_with_embeddings(
            SAMPLE_DOC,
            "guide.md",
            "framework",
            "docs",
            Some(&embedder),
        );
        // Embeddings must be deterministic.
        for (a, b) in chunks_a.iter().zip(chunks_b.iter()) {
            assert_eq!(a.embedding, b.embedding);
        }
    }
}
