//! Error types for IMPERIUM core.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Intent error: {0}")]
    Intent(#[from] IntentError),
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] SerializationError),
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum IntentError {
    #[error("Validation failed: {0}")]
    Validation(String),
    #[error("Compilation failed: {0}")]
    Compilation(String),
    #[error("Intent not found: {0}")]
    NotFound(crate::intent::IntentId),
    #[error("Approval required but not given")]
    ApprovalRequired,
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Invalid hash length (expected 32 bytes)")]
    InvalidHashLength,
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
}

#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}
