//! Knowledge ingestion pipeline.
//!
//! The [`IngestionPipeline`] walks one or more filesystem sources, reads
//! every matching file, and inserts the resulting chunks into a
//! [`VectorStore`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::embeddings::EmbeddingProvider;
use crate::ingest::DocumentIngester;
use crate::store::{StoreError, VectorStore};

// ── IngestionSource ───────────────────────────────────────────────────────────

/// Configuration for a document source to ingest.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestionSource {
    /// Path to file or directory.
    pub path: String,
    /// Domain tag for all ingested chunks.
    pub domain: String,
    /// Document type (framework, project, reference, session_history).
    pub doc_type: String,
    /// File extensions to include (e.g., `["md", "rs", "txt"]`). Empty = all.
    pub extensions: Vec<String>,
    /// Whether to recurse into subdirectories.
    pub recursive: bool,
}

// ── IngestionResult ───────────────────────────────────────────────────────────

/// Result of an ingestion run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestionResult {
    pub files_processed: usize,
    pub chunks_ingested: usize,
    pub errors: Vec<String>,
    pub domains: HashMap<String, usize>,
}

impl Default for IngestionResult {
    fn default() -> Self {
        Self {
            files_processed: 0,
            chunks_ingested: 0,
            errors: Vec::new(),
            domains: HashMap::new(),
        }
    }
}

// ── TOML config structures ────────────────────────────────────────────────────

/// Top-level structure for parsing `ingestion.toml`.
#[derive(Debug, serde::Deserialize)]
struct IngestionConfig {
    #[serde(rename = "source")]
    sources: Option<Vec<IngestionSource>>,
}

// ── IngestionPipeline ─────────────────────────────────────────────────────────

/// The ingestion pipeline processes multiple sources into a [`VectorStore`].
pub struct IngestionPipeline {
    sources: Vec<IngestionSource>,
    embedder: Option<Box<dyn EmbeddingProvider>>,
}

impl Default for IngestionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl IngestionPipeline {
    /// Create an empty pipeline with no sources.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            embedder: None,
        }
    }

    /// Add a source to the pipeline.
    pub fn add_source(&mut self, source: IngestionSource) {
        self.sources.push(source);
    }

    /// Attach an embedding provider used when ingesting chunks.
    #[must_use]
    pub fn with_embedder(mut self, embedder: Box<dyn EmbeddingProvider>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Run the pipeline, ingesting all sources into `store`.
    pub fn run(&self, store: &mut VectorStore) -> IngestionResult {
        let mut result = IngestionResult::default();

        for source in &self.sources {
            let path = Path::new(&source.path);
            let files = match collect_files(path, &source.extensions, source.recursive) {
                Ok(f) => f,
                Err(e) => {
                    result.errors.push(format!(
                        "failed to enumerate source '{}': {e}",
                        source.path
                    ));
                    continue;
                }
            };

            for file_path in files {
                let content = match std::fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        result.errors.push(format!(
                            "failed to read '{}': {e}",
                            file_path.display()
                        ));
                        continue;
                    }
                };

                let document_path = file_path.to_string_lossy().into_owned();
                let chunks = DocumentIngester::ingest_with_embeddings(
                    &content,
                    &document_path,
                    &source.domain,
                    &source.doc_type,
                    self.embedder.as_deref(),
                );

                let chunk_count = chunks.len();
                for chunk in chunks {
                    if let Err(StoreError::Backend(e)) = store.insert(chunk) {
                        result
                            .errors
                            .push(format!("insert error for '{document_path}': {e}"));
                    }
                }

                result.files_processed += 1;
                result.chunks_ingested += chunk_count;
                *result.domains.entry(source.domain.clone()).or_insert(0) += chunk_count;
            }
        }

        result
    }

    /// Create a pipeline from a TOML config file.
    ///
    /// The file should contain one or more `[[source]]` tables.
    pub fn from_config(path: &Path) -> Result<Self, String> {
        let raw =
            std::fs::read_to_string(path).map_err(|e| format!("cannot read config: {e}"))?;
        let config: IngestionConfig =
            toml::from_str(&raw).map_err(|e| format!("cannot parse TOML: {e}"))?;

        let mut pipeline = Self::new();
        for source in config.sources.unwrap_or_default() {
            pipeline.add_source(source);
        }
        Ok(pipeline)
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Return all files under `root` that match the given extension filter.
///
/// * `extensions` — list of extensions *without* the leading `.`  (e.g. `"md"`).
///   Empty list means accept every file.
/// * `recursive`  — when `true`, descend into subdirectories.
fn collect_files(
    root: &Path,
    extensions: &[String],
    recursive: bool,
) -> Result<Vec<PathBuf>, String> {
    if !root.exists() {
        return Err(format!("path does not exist: {}", root.display()));
    }

    if root.is_file() {
        if extension_matches(root, extensions) {
            return Ok(vec![root.to_path_buf()]);
        }
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_dir(root, extensions, recursive, &mut files)?;
    Ok(files)
}

fn collect_dir(
    dir: &Path,
    extensions: &[String],
    recursive: bool,
    out: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read directory '{}': {e}", dir.display()))?;

    let mut entries: Vec<_> = entries
        .filter_map(|e| e.ok())
        .collect();

    // Sort for deterministic ordering in tests.
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                collect_dir(&path, extensions, recursive, out)?;
            }
        } else if path.is_file() && extension_matches(&path, extensions) {
            out.push(path);
        }
    }

    Ok(())
}

/// Return `true` when `path`'s extension is in `extensions`, or `extensions` is empty.
fn extension_matches(path: &Path, extensions: &[String]) -> bool {
    if extensions.is_empty() {
        return true;
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => extensions.iter().any(|allowed| allowed == ext),
        None => false,
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    const SAMPLE_MD: &str = "# Test Document\n\nPreamble text.\n\n## Section One\n\nContent of section one.\n\n## Section Two\n\nContent of section two.\n";

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    // ── single file ──────────────────────────────────────────────────────────

    #[test]
    fn pipeline_ingests_single_file() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "doc.md", SAMPLE_MD);

        let file_path = dir.path().join("doc.md");
        let mut pipeline = IngestionPipeline::new();
        pipeline.add_source(IngestionSource {
            path: file_path.to_string_lossy().into_owned(),
            domain: "test".to_string(),
            doc_type: "reference".to_string(),
            extensions: vec!["md".to_string()],
            recursive: false,
        });

        let mut store = VectorStore::in_memory();
        let result = pipeline.run(&mut store);

        assert_eq!(result.files_processed, 1, "should process 1 file");
        assert!(result.chunks_ingested > 0, "should produce chunks");
        assert!(result.errors.is_empty(), "no errors expected");
    }

    // ── directory (non-recursive) ─────────────────────────────────────────────

    #[test]
    fn pipeline_ingests_flat_directory() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "a.md", SAMPLE_MD);
        write_file(dir.path(), "b.md", SAMPLE_MD);
        write_file(dir.path(), "c.txt", "ignored");

        let mut pipeline = IngestionPipeline::new();
        pipeline.add_source(IngestionSource {
            path: dir.path().to_string_lossy().into_owned(),
            domain: "docs".to_string(),
            doc_type: "framework".to_string(),
            extensions: vec!["md".to_string()],
            recursive: false,
        });

        let mut store = VectorStore::in_memory();
        let result = pipeline.run(&mut store);

        assert_eq!(result.files_processed, 2, "should process 2 md files");
        assert!(result.errors.is_empty());
    }

    // ── recursive directory ───────────────────────────────────────────────────

    #[test]
    fn pipeline_ingests_directory_recursively() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "top.md", SAMPLE_MD);
        write_file(dir.path(), "sub/nested.md", SAMPLE_MD);

        let mut pipeline = IngestionPipeline::new();
        pipeline.add_source(IngestionSource {
            path: dir.path().to_string_lossy().into_owned(),
            domain: "recursive".to_string(),
            doc_type: "project".to_string(),
            extensions: vec!["md".to_string()],
            recursive: true,
        });

        let mut store = VectorStore::in_memory();
        let result = pipeline.run(&mut store);

        assert_eq!(result.files_processed, 2, "should find files in subdirectory");
        assert!(result.errors.is_empty());
    }

    // ── extension filter ─────────────────────────────────────────────────────

    #[test]
    fn pipeline_extension_filter_excludes_non_matching() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "keep.md", SAMPLE_MD);
        write_file(dir.path(), "skip.rs", "fn main() {}");
        write_file(dir.path(), "skip.txt", "plain text");

        let mut pipeline = IngestionPipeline::new();
        pipeline.add_source(IngestionSource {
            path: dir.path().to_string_lossy().into_owned(),
            domain: "filtered".to_string(),
            doc_type: "docs".to_string(),
            extensions: vec!["md".to_string()],
            recursive: false,
        });

        let mut store = VectorStore::in_memory();
        let result = pipeline.run(&mut store);

        assert_eq!(result.files_processed, 1, "only .md file should be processed");
    }

    // ── empty directory ───────────────────────────────────────────────────────

    #[test]
    fn pipeline_empty_directory_returns_zero_chunks() {
        let dir = tempfile::tempdir().unwrap();

        let mut pipeline = IngestionPipeline::new();
        pipeline.add_source(IngestionSource {
            path: dir.path().to_string_lossy().into_owned(),
            domain: "empty".to_string(),
            doc_type: "docs".to_string(),
            extensions: vec!["md".to_string()],
            recursive: false,
        });

        let mut store = VectorStore::in_memory();
        let result = pipeline.run(&mut store);

        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_ingested, 0);
        assert!(result.errors.is_empty());
    }

    // ── stats tracking ────────────────────────────────────────────────────────

    #[test]
    fn pipeline_stats_track_domain_chunk_counts() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "doc.md", SAMPLE_MD);

        let mut pipeline = IngestionPipeline::new();
        pipeline.add_source(IngestionSource {
            path: dir.path().to_string_lossy().into_owned(),
            domain: "music".to_string(),
            doc_type: "reference".to_string(),
            extensions: vec!["md".to_string()],
            recursive: false,
        });

        let mut store = VectorStore::in_memory();
        let result = pipeline.run(&mut store);

        // SAMPLE_MD has 3 H2 sections + preamble = 3 chunks.
        assert!(result.chunks_ingested >= 1);
        let music_count = result.domains.get("music").copied().unwrap_or(0);
        assert_eq!(music_count, result.chunks_ingested, "domain count should match total");
    }

    // ── extension filter: empty = accept all ─────────────────────────────────

    #[test]
    fn pipeline_empty_extensions_accepts_all_files() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "a.md", "# A\n\nContent.");
        write_file(dir.path(), "b.rs", "// Rust file");
        write_file(dir.path(), "c.txt", "Text file");

        let mut pipeline = IngestionPipeline::new();
        pipeline.add_source(IngestionSource {
            path: dir.path().to_string_lossy().into_owned(),
            domain: "all".to_string(),
            doc_type: "misc".to_string(),
            extensions: vec![],
            recursive: false,
        });

        let mut store = VectorStore::in_memory();
        let result = pipeline.run(&mut store);

        assert_eq!(result.files_processed, 3, "all 3 files should be ingested");
    }

    // ── from_config ───────────────────────────────────────────────────────────

    #[test]
    fn pipeline_from_config_parses_toml() {
        let dir = tempfile::tempdir().unwrap();
        let toml_content = format!(
            r#"
[[source]]
path = "{}"
domain = "system"
doc_type = "reference"
extensions = ["md"]
recursive = false
"#,
            dir.path().to_string_lossy().replace('\\', "/")
        );

        let config_path = dir.path().join("ingestion.toml");
        std::fs::write(&config_path, toml_content).unwrap();

        let pipeline = IngestionPipeline::from_config(&config_path).expect("should parse");
        assert_eq!(pipeline.sources.len(), 1);
        assert_eq!(pipeline.sources[0].domain, "system");
    }
}
