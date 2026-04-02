# `nstn-ruvector`

**Path:** `crates/ruvector`  
**Package:** `nstn-ruvector` v0.5.0  
**Lines of code:** 2 573  
**Dependencies on other nstn-\* crates:** `nstn-common`

> **See also:** [Architecture Overview](../architecture/overview.md) · [Memory System](../architecture/memory.md) · [nstn-ruflo](./nstn-ruflo.md)

---

## Purpose

`nstn-ruvector` is the **knowledge tier** of Nanosistant. It provides a pluggable vector store with two storage backends (in-memory and Qdrant), a deterministic hash-based embedding provider for offline use, a markdown-aware document ingestion pipeline, an MCP server that exposes the store as four JSON-RPC tools, and a gRPC service stub.

Domain agents query the knowledge store via gRPC when they need domain-specific context (`KnowledgeQuery` events). The store is populated at startup by running the `IngestionPipeline` over configured document sources.

---

## Modules

```
nstn_ruvector
├── store       — VectorBackend trait, InMemoryBackend, VectorStore, KnowledgeChunk, StoredChunk
├── qdrant      — QdrantBackend (HTTP REST)
├── embeddings  — EmbeddingProvider trait, HashEmbedding, cosine_similarity
├── ingest      — DocumentIngester
├── pipeline    — IngestionPipeline, IngestionSource, IngestionResult
├── mcp_server  — McpServer (4 tools: query, ingest, domains, stats)
└── grpc_server — RuVectorGrpcService
```

---

## Module: `store`

**File:** `crates/ruvector/src/store.rs`

### Error Types

```rust
pub enum StoreError {
    Backend(String),
    Embedding(String),
}
```

### Data Types

```rust
/// A chunk returned from a query (public-facing type).
pub struct KnowledgeChunk {
    pub id: String,
    pub content: String,
    pub domain: String,
    pub source_section: String,
    pub source_document: String,
    pub metadata: HashMap<String, String>,
}

/// A chunk as stored internally (includes optional embedding).
pub struct StoredChunk {
    pub id: String,
    pub content: String,
    pub domain: String,
    pub source_section: String,
    pub source_document: String,
    pub metadata: HashMap<String, String>,
    pub embedding: Option<Vec<f32>>,   // None when no embedder is attached
}
```

### `VectorBackend` Trait

```rust
pub trait VectorBackend: Send + Sync {
    fn insert(&mut self, chunk: StoredChunk) -> Result<(), StoreError>;

    /// Keyword-based text query (TF-IDF-style token matching).
    fn query(&self, query_text: &str, domain_filter: &str, max_results: u32) -> Vec<KnowledgeChunk>;

    /// Semantic query using a pre-computed dense embedding.
    fn query_by_embedding(
        &self,
        embedding: &[f32],
        domain_filter: &str,
        max_results: u32,
    ) -> Vec<KnowledgeChunk>;

    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn domain_counts(&self) -> HashMap<String, usize> { HashMap::new() }
}
```

Implementations: [`InMemoryBackend`](#inmemorybackend) and [`QdrantBackend`](#qdrantbackend).

### `InMemoryBackend`

```rust
#[derive(Debug, Default)]
pub struct InMemoryBackend {
    chunks: Vec<StoredChunk>,
}

impl InMemoryBackend {
    pub fn new() -> Self;
}

impl VectorBackend for InMemoryBackend { /* … */ }
```

Keyword scoring uses a TF-IDF-style heuristic: counts query tokens that appear in the chunk content (case-insensitive), normalised by query length. Semantic search uses `cosine_similarity` over stored `embedding` vectors. Chunks without embeddings are excluded from semantic results.

### `VectorStore`

```rust
pub struct VectorStore {
    backend: Box<dyn VectorBackend>,
}

impl VectorStore {
    pub fn in_memory() -> Self;
    pub fn with_qdrant(qdrant: QdrantBackend) -> Self;
    pub fn with_backend(backend: Box<dyn VectorBackend>) -> Self;

    pub fn insert(&mut self, chunk: StoredChunk) -> Result<(), StoreError>;
    pub fn query(&self, query_text: &str, domain_filter: &str, max_results: u32) -> Vec<KnowledgeChunk>;
    pub fn query_by_embedding(&self, embedding: &[f32], domain_filter: &str,
                              max_results: u32) -> Vec<KnowledgeChunk>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn domain_counts(&self) -> HashMap<String, usize>;
}
```

---

## Module: `qdrant`

**File:** `crates/ruvector/src/qdrant.rs`

Qdrant HTTP REST backend. Connects to a Qdrant instance and fails gracefully when unavailable: inserts are logged and discarded; queries return empty results.

```rust
pub struct QdrantBackend { /* private */ }

impl QdrantBackend {
    /// Create a backend targeting `base_url` and `collection_name`.
    /// Uses a blocking `reqwest` client.
    pub fn new(base_url: &str, collection_name: &str) -> Self;

    /// Check whether Qdrant is reachable.
    pub fn is_healthy(&self) -> bool;
}

impl VectorBackend for QdrantBackend {
    fn insert(&mut self, chunk: StoredChunk) -> Result<(), StoreError>;
    fn query(&self, query_text: &str, domain_filter: &str, max_results: u32) -> Vec<KnowledgeChunk>;
    fn query_by_embedding(&self, embedding: &[f32], domain_filter: &str,
                          max_results: u32) -> Vec<KnowledgeChunk>;
    fn len(&self) -> usize;
    fn domain_counts(&self) -> HashMap<String, usize>;
}
```

**Wire format:** Uses Qdrant's REST API (`/collections/{name}/points`, `/collections/{name}/points/search`). Payloads are serialised as JSON. Domain filtering is implemented as a `must` filter on the `domain` payload field. `query_text` searches use scroll with payload filtering (not semantic — Qdrant keyword search is a future addition).

---

## Module: `embeddings`

**File:** `crates/ruvector/src/embeddings.rs`

### Error Types

```rust
pub enum EmbeddingError {
    Provider(String),
}
```

### `EmbeddingProvider` Trait

```rust
pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
    fn dimension(&self) -> usize;
}
```

### `HashEmbedding`

Deterministic hash-based embedding. Requires no external model or API key. Not semantically meaningful — for testing and offline fallback only.

```rust
pub struct HashEmbedding {
    dimension: usize,
}

impl HashEmbedding {
    pub fn new(dimension: usize) -> Self;
}

impl EmbeddingProvider for HashEmbedding { /* … */ }
```

**Algorithm:** Each word is hashed (Rust `DefaultHasher`) to a bucket index in `[0, dimension)`. A secondary hash determines the sign/magnitude (+1 or −1). The resulting vector is L2-normalised so cosine similarity equals dot product. The dimension is configurable; 256 is a reasonable offline default.

### `cosine_similarity`

```rust
/// Cosine similarity between two equal-length vectors.
/// Returns a value in [-1.0, 1.0]. Returns 0.0 for zero-magnitude vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32;
```

---

## Module: `ingest`

**File:** `crates/ruvector/src/ingest.rs`

Stateless markdown document chunker. Splits on `## ` (H2) headers and tags each chunk with domain metadata.

```rust
pub struct DocumentIngester;

impl DocumentIngester {
    /// Ingest content without embeddings.
    pub fn ingest(
        content: &str,
        document_path: &str,
        domain: &str,
        doc_type: &str,
    ) -> Vec<StoredChunk>;

    /// Ingest content and optionally generate embeddings.
    /// Embedding errors are logged as warnings; the chunk is stored
    /// with `embedding: None` rather than aborting ingestion.
    pub fn ingest_with_embeddings(
        content: &str,
        document_path: &str,
        domain: &str,
        doc_type: &str,
        embedder: Option<&dyn EmbeddingProvider>,
    ) -> Vec<StoredChunk>;
}
```

**Chunking logic:** The document is split on `\n## ` (the H2 delimiter with preceding newline). Each section becomes one chunk. The first segment (before the first `## `) is included as-is. Empty segments are skipped. Each chunk receives a UUID v4 as its `id`, and the section heading (first line) is stored in `source_section`.

---

## Module: `pipeline`

**File:** `crates/ruvector/src/pipeline.rs`

Orchestrates ingestion of multiple document sources into a `VectorStore`.

### `IngestionSource`

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestionSource {
    pub path: String,           // file or directory path
    pub domain: String,         // semantic domain tag
    pub doc_type: String,       // "framework" | "project" | "reference" | "session_history"
    pub extensions: Vec<String>, // e.g. ["md", "rs", "txt"]. Empty = all files
    pub recursive: bool,
}
```

### `IngestionResult`

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestionResult {
    pub files_processed: usize,
    pub chunks_ingested: usize,
    pub errors: Vec<String>,
    pub domains: HashMap<String, usize>,  // domain → chunk count
}
```

### `IngestionPipeline`

```rust
pub struct IngestionPipeline { /* private */ }

impl IngestionPipeline {
    pub fn new() -> Self;

    pub fn add_source(&mut self, source: IngestionSource);

    /// Attach an embedding provider for semantic embeddings.
    pub fn with_embedder(self, embedder: Box<dyn EmbeddingProvider>) -> Self;

    /// Load sources from an `ingestion.toml` file.
    pub fn from_toml(path: impl AsRef<Path>) -> Result<Self, std::io::Error>;

    /// Run the pipeline, inserting all chunks into `store`.
    pub fn run(&self, store: &mut VectorStore) -> IngestionResult;
}
```

**TOML config format:**

```toml
[[source]]
path = "docs/framework"
domain = "framework"
doc_type = "reference"
extensions = ["md"]
recursive = true

[[source]]
path = "crates/common/src"
domain = "development"
doc_type = "source"
extensions = ["rs"]
recursive = false
```

---

## Module: `mcp_server`

**File:** `crates/ruvector/src/mcp_server.rs`

JSON-RPC 2.0 MCP server that exposes the vector store as four tools over stdio (newline-delimited JSON). Implements the Model Context Protocol interface for integration with Claude and other MCP-compatible hosts.

```rust
pub struct McpServer {
    store: VectorStore,
}

impl McpServer {
    pub fn new(store: VectorStore) -> Self;

    /// Run the MCP server loop, reading from `stdin` and writing to `stdout`.
    /// Blocks until EOF on stdin.
    pub fn run(&mut self);
}
```

### Exposed MCP Tools

| Tool name | Parameters | Description |
|---|---|---|
| `ruvector_query` | `query: String`, `domain?: String`, `max_results?: u32` | Keyword search over the knowledge store. Returns up to `max_results` (default 5) chunks matching `query`, optionally filtered by `domain`. |
| `ruvector_ingest` | `content: String`, `document_path: String`, `domain: String`, `doc_type: String` | Ingest a markdown document into the store. Splits on `## ` headers. Returns chunk count. |
| `ruvector_domains` | _(none)_ | List all domains with their chunk counts. Returns a JSON object. |
| `ruvector_stats` | _(none)_ | Get overall store statistics: total chunks, domain count, last-updated timestamp. |

**Protocol:** Standard MCP over stdio. Requests are newline-delimited JSON objects with `jsonrpc`, `id`, `method`, and `params` fields. Responses are `result` or `error`.

---

## Module: `grpc_server`

**File:** `crates/ruvector/src/grpc_server.rs`

gRPC service stub that exposes knowledge queries from domain agents.

```rust
pub struct RuVectorGrpcService {
    store: Arc<Mutex<VectorStore>>,
}

impl RuVectorGrpcService {
    pub fn new(store: Arc<Mutex<VectorStore>>) -> Self;
}

// Implements the generated KnowledgeService gRPC trait
```

---

## Usage Example

```rust
use nstn_ruvector::{
    VectorStore, InMemoryBackend, EmbeddingProvider, HashEmbedding,
    DocumentIngester, IngestionPipeline, IngestionSource, McpServer,
    cosine_similarity,
};

// In-memory store with hash embeddings (no API key needed)
let mut store = VectorStore::in_memory();
let embedder: Box<dyn EmbeddingProvider> = Box::new(HashEmbedding::new(256));

// Ingest a markdown document
let content = "# My Document\n\n## Music Theory\nChords and scales...\n\n## Production\nMixing tips...";
let chunks = DocumentIngester::ingest_with_embeddings(
    content,
    "docs/music-theory.md",
    "music",
    "reference",
    Some(embedder.as_ref()),
);
for chunk in chunks {
    store.insert(chunk).unwrap();
}

// Keyword query
let results = store.query("chords scales", "music", 5);
for r in &results {
    println!("{}: {}", r.source_section, &r.content[..50.min(r.content.len())]);
}

// Semantic query
let query_vec = embedder.embed("music theory chords").unwrap();
let semantic_results = store.query_by_embedding(&query_vec, "music", 3);

// Pipeline ingestion from a TOML config
let mut pipeline = IngestionPipeline::new();
pipeline.add_source(IngestionSource {
    path: "docs/framework".into(),
    domain: "framework".into(),
    doc_type: "reference".into(),
    extensions: vec!["md".into()],
    recursive: true,
});
let result = pipeline.run(&mut store);
println!("Ingested {} chunks from {} files", result.chunks_ingested, result.files_processed);
```
