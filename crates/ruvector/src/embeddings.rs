//! Embedding generation for semantic search.
//!
//! Provides the [`EmbeddingProvider`] trait and a built-in
//! [`HashEmbedding`] implementation that requires no external model.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("provider error: {0}")]
    Provider(String),
}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Trait for generating text embeddings.
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a single text string into a dense float vector.
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Embed a batch of texts.  Default impl calls [`embed`] in a loop.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Dimensionality of the embedding vectors produced by this provider.
    fn dimension(&self) -> usize;
}

// ── HashEmbedding ─────────────────────────────────────────────────────────────

/// Deterministic hash-based embedding (no model required).
///
/// Distributes each word's hash into `dimension` buckets and
/// accumulates a float value per bucket.  The resulting vector is
/// L2-normalised so cosine similarity is equivalent to dot product.
///
/// **Not** semantically meaningful — for testing and fallback only.
pub struct HashEmbedding {
    dimension: usize,
}

impl HashEmbedding {
    /// Create a new `HashEmbedding` with the specified vector dimension.
    #[must_use]
    pub fn new(dimension: usize) -> Self {
        assert!(dimension > 0, "dimension must be > 0");
        Self { dimension }
    }

    /// Hash a single word to a bucket index in `[0, dimension)`.
    fn bucket(&self, word: &str) -> usize {
        let mut h = DefaultHasher::new();
        word.hash(&mut h);
        (h.finish() as usize) % self.dimension
    }
}

impl EmbeddingProvider for HashEmbedding {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let mut vec = vec![0.0_f32; self.dimension];

        for word in text.split_whitespace() {
            let word_lower = word.to_lowercase();
            let idx = self.bucket(&word_lower);
            // Also use a secondary hash for the sign/magnitude so tokens
            // with different words that happen to collide partially cancel.
            let mut h2 = DefaultHasher::new();
            word_lower.hash(&mut h2);
            let h2_val = h2.finish();
            let contribution = if h2_val & 1 == 0 { 1.0_f32 } else { -1.0_f32 };
            vec[idx] += contribution;
        }

        // L2-normalise.
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-9 {
            for v in &mut vec {
                *v /= norm;
            }
        }

        Ok(vec)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

// ── Cosine similarity ─────────────────────────────────────────────────────────

/// Compute cosine similarity between two equal-length vectors.
///
/// Returns a value in `[-1.0, 1.0]`.  Returns `0.0` when either
/// vector has zero magnitude.
///
/// # Panics
///
/// Panics if `a.len() != b.len()`.
#[must_use]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vectors must have the same length");

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a < 1e-9 || norm_b < 1e-9 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_embedding_consistent() {
        let emb = HashEmbedding::new(64);
        let a = emb.embed("hello world").unwrap();
        let b = emb.embed("hello world").unwrap();
        assert_eq!(a, b, "same input must produce same embedding");
    }

    #[test]
    fn hash_embedding_dimension() {
        let emb = HashEmbedding::new(128);
        let v = emb.embed("some text here").unwrap();
        assert_eq!(v.len(), 128);
    }

    #[test]
    fn hash_embedding_normalised() {
        let emb = HashEmbedding::new(64);
        let v = emb.embed("test normalisation").unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "expected unit vector, got norm {norm}");
    }

    #[test]
    fn hash_embedding_empty_text() {
        let emb = HashEmbedding::new(32);
        let v = emb.embed("").unwrap();
        assert_eq!(v.len(), 32);
        // All zeros — norm is 0, stays zero
        assert!(v.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn hash_embedding_batch() {
        let emb = HashEmbedding::new(32);
        let vecs = emb.embed_batch(&["foo", "bar", "baz"]).unwrap();
        assert_eq!(vecs.len(), 3);
        for v in &vecs {
            assert_eq!(v.len(), 32);
        }
    }

    #[test]
    fn cosine_similarity_identical() {
        let v = vec![1.0_f32, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0_f32, 0.0, 0.0];
        let b = vec![0.0_f32, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_opposite() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![-1.0_f32, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let a = vec![0.0_f32, 0.0, 0.0];
        let b = vec![1.0_f32, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn cosine_similarity_known_values() {
        // [3, 4] has norm 5; [4, 3] has norm 5; dot = 12+12 = 24; sim = 24/25
        let a = vec![3.0_f32, 4.0];
        let b = vec![4.0_f32, 3.0];
        let expected = 24.0_f32 / 25.0;
        assert!((cosine_similarity(&a, &b) - expected).abs() < 1e-6);
    }
}
