//! Local deterministic execution layer.
//!
//! Delegates to `nstn_common::try_deterministic_resolution` so that
//! zero-cost queries are resolved without any network round-trip.

/// Stateless executor that checks the deterministic function library.
pub struct LocalExecutor;

impl LocalExecutor {
    /// Try to resolve `message` via deterministic (zero-token) functions.
    ///
    /// Returns `Some(answer)` when a deterministic rule matches, or `None`
    /// when the message must be forwarded to the `RuFlo` brain.
    #[must_use]
    pub fn try_resolve(message: &str) -> Option<String> {
        nstn_common::try_deterministic_resolution(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_deterministic_query_resolves() {
        // "c major scale" is handled by the deterministic library.
        let result = LocalExecutor::try_resolve("c major scale");
        assert!(result.is_some(), "expected a deterministic answer for 'c major scale'");
    }

    #[test]
    fn open_ended_query_returns_none() {
        let result = LocalExecutor::try_resolve("help me write a verse about love");
        assert!(result.is_none());
    }
}
