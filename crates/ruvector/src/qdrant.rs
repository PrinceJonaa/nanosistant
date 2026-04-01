//! Qdrant vector-database backend.
//!
//! Connects to a Qdrant instance via the HTTP REST API.
//! Fails gracefully when Qdrant is unavailable: inserts are logged and
//! discarded; queries return empty results rather than panicking.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::store::{KnowledgeChunk, StoreError, StoredChunk, VectorBackend};

// ── Qdrant REST payload types ─────────────────────────────────────────────────

#[derive(Serialize)]
struct UpsertRequest {
    points: Vec<Point>,
}

#[derive(Serialize)]
struct Point {
    id: String,
    vector: Vec<f32>,
    payload: HashMap<String, Value>,
}

#[derive(Serialize)]
struct SearchRequest<'a> {
    vector: &'a [f32],
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<Filter>,
    limit: u32,
    with_payload: bool,
}

#[derive(Serialize)]
struct ScrollRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<Filter>,
    limit: u32,
    with_payload: bool,
    with_vector: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<&'a str>,
}

#[derive(Serialize, Clone)]
struct Filter {
    must: Vec<FieldCondition>,
}

#[derive(Serialize, Clone)]
struct FieldCondition {
    key: String,
    #[serde(rename = "match")]
    match_value: MatchValue,
}

#[derive(Serialize, Clone)]
struct MatchValue {
    value: String,
}

#[derive(Deserialize, Debug)]
struct SearchResponse {
    result: Vec<ScoredPoint>,
}

#[derive(Deserialize, Debug)]
struct ScrollResponse {
    result: ScrollResult,
}

#[derive(Deserialize, Debug)]
struct ScrollResult {
    points: Vec<ScrollPoint>,
}

#[derive(Deserialize, Debug)]
struct ScoredPoint {
    id: Value,
    payload: Option<HashMap<String, Value>>,
    #[serde(default)]
    #[allow(dead_code)]
    score: f32,
}

#[derive(Deserialize, Debug)]
struct ScrollPoint {
    id: Value,
    payload: Option<HashMap<String, Value>>,
}

// ── QdrantBackend ─────────────────────────────────────────────────────────────

/// Qdrant vector-database backend.
///
/// Uses the synchronous (`blocking`) reqwest client so the rest of the
/// codebase does not need to be async for v0.2.
pub struct QdrantBackend {
    url: String,
    collection_name: String,
    client: reqwest::blocking::Client,
    initialized: bool,
}

impl std::fmt::Debug for QdrantBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QdrantBackend")
            .field("url", &self.url)
            .field("collection_name", &self.collection_name)
            .field("initialized", &self.initialized)
            .finish()
    }
}

impl QdrantBackend {
    /// Create a new (uninitialised) backend pointing at `url`.
    ///
    /// Call [`initialize`][Self::initialize] before inserting points.
    #[must_use]
    pub fn new(url: &str, collection_name: &str) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            url: url.trim_end_matches('/').to_owned(),
            collection_name: collection_name.to_owned(),
            client,
            initialized: false,
        }
    }

    /// Create the collection if it does not already exist.
    ///
    /// Returns `Ok(())` if creation succeeds *or* if the collection
    /// already exists.  Returns `Err` only on genuine communication
    /// failures.
    pub fn initialize(&mut self, vector_size: u32) -> Result<(), StoreError> {
        let url = format!("{}/collections/{}", self.url, self.collection_name);
        let body = json!({
            "vectors": {
                "size": vector_size,
                "distance": "Cosine"
            }
        });

        let resp = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .map_err(|e| StoreError::Backend(format!("qdrant init request failed: {e}")))?;

        // 200 = created, 409 = already exists — both are fine.
        let status = resp.status();
        if status.is_success() || status.as_u16() == 409 {
            self.initialized = true;
            Ok(())
        } else {
            let text = resp.text().unwrap_or_default();
            Err(StoreError::Backend(format!(
                "qdrant init returned {status}: {text}"
            )))
        }
    }

    /// Return `true` if the Qdrant health endpoint responds successfully.
    #[must_use]
    pub fn health_check(&self) -> bool {
        let url = format!("{}/healthz", self.url);
        self.client
            .get(&url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Convert a Qdrant payload map into a [`KnowledgeChunk`].
    fn payload_to_chunk(id: &Value, payload: &HashMap<String, Value>) -> KnowledgeChunk {
        let get = |key: &str| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        };

        // Reconstruct metadata from remaining payload keys.
        let reserved = ["content", "domain", "source_section", "source_document"];
        let metadata: HashMap<String, String> = payload
            .iter()
            .filter(|(k, _)| !reserved.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_owned()))
            .collect();

        KnowledgeChunk {
            id: id.as_str().unwrap_or("").to_owned(),
            content: get("content"),
            domain: get("domain"),
            source_section: get("source_section"),
            source_document: get("source_document"),
            metadata,
        }
    }

    /// Build a domain filter, or `None` when `domain_filter` is empty.
    fn domain_filter(domain_filter: &str) -> Option<Filter> {
        if domain_filter.is_empty() {
            return None;
        }
        Some(Filter {
            must: vec![FieldCondition {
                key: "domain".to_owned(),
                match_value: MatchValue {
                    value: domain_filter.to_owned(),
                },
            }],
        })
    }
}

impl VectorBackend for QdrantBackend {
    fn insert(&mut self, chunk: StoredChunk) -> Result<(), StoreError> {
        // We need at least a placeholder vector if no embedding is stored.
        let vector = match chunk.embedding {
            Some(ref v) if !v.is_empty() => v.clone(),
            _ => {
                // Cannot insert into Qdrant without a vector — skip silently.
                tracing::warn!(
                    id = %chunk.id,
                    "QdrantBackend: chunk has no embedding, skipping insert"
                );
                return Ok(());
            }
        };

        let mut payload: HashMap<String, Value> = HashMap::new();
        payload.insert("content".to_owned(), json!(chunk.content));
        payload.insert("domain".to_owned(), json!(chunk.domain));
        payload.insert("source_section".to_owned(), json!(chunk.source_section));
        payload.insert("source_document".to_owned(), json!(chunk.source_document));
        for (k, v) in &chunk.metadata {
            payload.insert(k.clone(), json!(v));
        }

        let req = UpsertRequest {
            points: vec![Point {
                id: chunk.id.clone(),
                vector,
                payload,
            }],
        };

        let url = format!("{}/collections/{}/points", self.url, self.collection_name);
        let resp = self
            .client
            .put(&url)
            .json(&req)
            .send()
            .map_err(|e| StoreError::Backend(format!("qdrant upsert failed: {e}")))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            Err(StoreError::Backend(format!(
                "qdrant upsert returned {status}: {text}"
            )))
        }
    }

    fn query(&self, _query_text: &str, domain_filter: &str, max_results: u32) -> Vec<KnowledgeChunk> {
        // Text-only query: use scroll with a domain filter.
        let url = format!(
            "{}/collections/{}/points/scroll",
            self.url, self.collection_name
        );

        let req = ScrollRequest {
            filter: Self::domain_filter(domain_filter),
            limit: max_results,
            with_payload: true,
            with_vector: false,
            offset: None,
        };

        match self.client.post(&url).json(&req).send() {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<ScrollResponse>() {
                    Ok(sr) => sr
                        .result
                        .points
                        .iter()
                        .filter_map(|p| {
                            p.payload
                                .as_ref()
                                .map(|pay| Self::payload_to_chunk(&p.id, pay))
                        })
                        .collect(),
                    Err(e) => {
                        tracing::warn!("QdrantBackend: failed to parse scroll response: {e}");
                        Vec::new()
                    }
                }
            }
            Ok(resp) => {
                tracing::warn!(
                    status = %resp.status(),
                    "QdrantBackend: scroll returned non-success status"
                );
                Vec::new()
            }
            Err(e) => {
                tracing::warn!("QdrantBackend: scroll request failed: {e}");
                Vec::new()
            }
        }
    }

    fn query_by_embedding(
        &self,
        embedding: &[f32],
        domain_filter: &str,
        max_results: u32,
    ) -> Vec<KnowledgeChunk> {
        let url = format!(
            "{}/collections/{}/points/search",
            self.url, self.collection_name
        );

        let req = SearchRequest {
            vector: embedding,
            filter: Self::domain_filter(domain_filter),
            limit: max_results,
            with_payload: true,
        };

        match self.client.post(&url).json(&req).send() {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<SearchResponse>() {
                    Ok(sr) => sr
                        .result
                        .iter()
                        .filter_map(|p| {
                            p.payload
                                .as_ref()
                                .map(|pay| Self::payload_to_chunk(&p.id, pay))
                        })
                        .collect(),
                    Err(e) => {
                        tracing::warn!("QdrantBackend: failed to parse search response: {e}");
                        Vec::new()
                    }
                }
            }
            Ok(resp) => {
                tracing::warn!(
                    status = %resp.status(),
                    "QdrantBackend: search returned non-success status"
                );
                Vec::new()
            }
            Err(e) => {
                tracing::warn!("QdrantBackend: search request failed: {e}");
                Vec::new()
            }
        }
    }

    fn len(&self) -> usize {
        // We cannot cheaply count without an extra REST call; return 0.
        0
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk(id: &str, content: &str, domain: &str, embedding: Vec<f32>) -> StoredChunk {
        StoredChunk {
            id: id.to_owned(),
            content: content.to_owned(),
            domain: domain.to_owned(),
            source_section: "sec".to_owned(),
            source_document: "doc.md".to_owned(),
            metadata: HashMap::new(),
            embedding: Some(embedding),
        }
    }

    #[test]
    fn qdrant_health_check_unavailable() {
        // Point at a definitely-unreachable address.
        let backend = QdrantBackend::new("http://127.0.0.1:19999", "test");
        assert!(!backend.health_check(), "health_check must return false when unreachable");
    }

    #[test]
    fn qdrant_insert_unavailable_does_not_panic() {
        let mut backend = QdrantBackend::new("http://127.0.0.1:19999", "test");
        let chunk = make_chunk("id1", "content", "domain", vec![0.1, 0.2, 0.3]);
        // Should return an error but MUST NOT panic.
        let _ = backend.insert(chunk);
    }

    #[test]
    fn qdrant_query_unavailable_returns_empty() {
        let backend = QdrantBackend::new("http://127.0.0.1:19999", "test");
        let results = backend.query("hello", "", 10);
        assert!(results.is_empty(), "unavailable backend must return empty");
    }

    #[test]
    fn qdrant_query_by_embedding_unavailable_returns_empty() {
        let backend = QdrantBackend::new("http://127.0.0.1:19999", "test");
        let results = backend.query_by_embedding(&[0.1, 0.2, 0.3], "", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn qdrant_insert_no_embedding_is_ok() {
        let mut backend = QdrantBackend::new("http://127.0.0.1:19999", "test");
        let mut chunk = make_chunk("id2", "content", "domain", vec![]);
        chunk.embedding = None;
        // Should silently succeed (skips the upsert).
        let result = backend.insert(chunk);
        assert!(result.is_ok());
    }
}
