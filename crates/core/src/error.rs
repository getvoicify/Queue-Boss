use thiserror::Error;

/// Typed backend failures. Adapters map raw driver errors onto these variants
/// with sanitized messages — a raw driver string must never reach the UI.
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("connection error: {0}")]
    Connection(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("internal error: {0}")]
    Internal(String),
}
