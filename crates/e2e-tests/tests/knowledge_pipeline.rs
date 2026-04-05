//! E2E tests for the RuVector knowledge tier.
//!
//! Verifies the full pipeline: ingest documents → store chunks → query by
//! keyword or embedding → retrieve correct results with domain isolation.
//! Uses the in-memory backend — no external Qdrant needed.

use std::collections::HashMap;

use nstn_ruvector::{DocumentIngester, RuVectorGrpcService, StoredChunk, VectorStore};

// ── Helpers ──────────────────────────────────────────────────────────────────

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

// ── Tests ────────────────────────────────────────────────────────────────────

/// Ingest markdown → store → query via the gRPC service layer → verify results.
#[test]
fn ingest_and_query_returns_relevant_chunks() {
    let mut store = VectorStore::in_memory();

    let content = "\
## C Major Scale
The C major scale consists of the notes C - D - E - F - G - A - B.
It contains no sharps or flats.

## BPM and Tempo
BPM (beats per minute) measures the tempo of a musical piece.
A tempo of 120 BPM means 120 beats occur every minute.
";

    let chunks = DocumentIngester::ingest(content, "music_theory.md", "music", "reference");
    assert!(
        chunks.len() >= 2,
        "should produce at least 2 chunks from 2 H2 sections"
    );

    for chunk in chunks {
        store.insert(chunk).expect("insert should succeed");
    }

    let svc = RuVectorGrpcService::new(store);

    // Query for scale content — should find the C major scale chunk.
    let results = svc.query("major scale notes", "music", 5);
    assert!(!results.is_empty(), "should find scale-related chunk");
    assert!(
        results[0].content.contains("C - D - E - F - G - A - B"),
        "top result should contain the exact scale"
    );

    // Query for tempo content.
    let results = svc.query("beats per minute tempo", "music", 5);
    assert!(!results.is_empty(), "should find tempo-related chunk");
    assert!(
        results[0].content.contains("BPM"),
        "top result should contain BPM"
    );
}

/// Domain filter enforces strict isolation — no cross-contamination.
#[test]
fn domain_filtered_knowledge_query() {
    let mut store = VectorStore::in_memory();

    store
        .insert(make_chunk(
            "music-1",
            "the C major scale has seven notes in a diatonic pattern",
            "music",
        ))
        .unwrap();
    store
        .insert(make_chunk(
            "dev-1",
            "scale your application with horizontal scaling patterns",
            "development",
        ))
        .unwrap();
    store
        .insert(make_chunk(
            "framework-1",
            "the distortion lattice maps scale invariance across lenses",
            "framework",
        ))
        .unwrap();

    let svc = RuVectorGrpcService::new(store);

    // "scale" appears in all three domains — filter should isolate.
    let music_results = svc.query("scale", "music", 10);
    assert!(
        music_results.iter().all(|r| r.domain == "music"),
        "music filter should exclude other domains"
    );

    let dev_results = svc.query("scale", "development", 10);
    assert!(
        dev_results.iter().all(|r| r.domain == "development"),
        "dev filter should exclude other domains"
    );

    // Empty domain filter returns results from all domains.
    let all_results = svc.query("scale", "", 10);
    assert!(
        all_results.len() >= 3,
        "unfiltered query should return chunks from all domains"
    );
}

/// Embedding-based semantic query returns cosine-closest vector first.
#[test]
fn embedding_based_semantic_query() {
    let mut store = VectorStore::in_memory();

    // Two chunks with known embeddings: [1,0] and [0,1] — orthogonal vectors.
    store
        .insert(make_chunk_with_embedding(
            "near",
            "this chunk is semantically close",
            "test",
            vec![1.0, 0.0],
        ))
        .unwrap();
    store
        .insert(make_chunk_with_embedding(
            "far",
            "this chunk is semantically distant",
            "test",
            vec![0.0, 1.0],
        ))
        .unwrap();

    let svc = RuVectorGrpcService::new(store);

    // Query with [0.9, 0.1] — cosine similarity is higher to [1,0] than [0,1].
    // cos([0.9,0.1], [1,0]) = 0.9 / √0.82 ≈ 0.995
    // cos([0.9,0.1], [0,1]) = 0.1 / √0.82 ≈ 0.110
    let results = svc.store().query_by_embedding(&[0.9, 0.1], "test", 5);
    assert!(results.len() == 2, "should return both chunks");
    assert_eq!(
        results[0].id, "near",
        "closest vector should rank first by cosine similarity"
    );
    assert_eq!(results[1].id, "far", "distant vector should rank second");
}
