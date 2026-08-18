//! Error Types
//!
//! Unified error handling for IMPERIUM.

use thiserror::Error;

/// Core error type
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Intent error: {0}")]
    Intent(#[from] IntentError),

    #[error("Event error: {0}")]
    Event(#[from] EventError),

    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),

    #[error("Policy error: {0}")]
    Policy(#[from] PolicyError),

    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),

    #[error("Sync error: {0}")]
    Sync(#[from] SyncError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] SerializationError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias
pub type CoreResult<T> = Result<T, CoreError>;

/// Intent errors
#[derive(Debug, Error)]
pub enum IntentError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Compilation failed: {0}")]
    Compilation(String),

    #[error("Intent not found: {0}")]
    NotFound(crate::intent::IntentId),

    #[error("Intent already exists: {0}")]
    AlreadyExists(crate::intent::IntentId),

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Approval required but not given")]
    ApprovalRequired,

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

/// Event errors
#[derive(Debug, Error)]
pub enum EventError {
    #[error("Event not found: {0}")]
    NotFound(crate::event::EventId),

    #[error("Event store error: {0}")]
    Store(String),

    #[error("Serialization failed: {0}")]
    Serialization(String),

    #[error("Integrity check failed for event {0}")]
    IntegrityCheckFailed(crate::event::EventId),

    #[error("Causal ordering violation")]
    CausalOrderingViolation,
}

/// Capability errors
#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("Capability not found: {0}")]
    NotFound(crate::capability::CapabilityId),

    #[error("Capability already registered: {0}")]
    AlreadyRegistered(crate::capability::CapabilityId),

    #[error("Manifest validation failed: {0}")]
    ManifestValidation(String),

    #[error("WASM validation failed: {0}")]
    WasmValidation(String),

    #[error("Capability not available: {0}")]
    NotAvailable(crate::capability::CapabilityId),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    #[error("Token error: {0}")]
    TokenError(String),
}

/// Policy errors
#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("Policy not found: {0}")]
    NotFound(crate::policy::PolicyId),

    #[error("Policy compilation failed: {0}")]
    CompilationFailed(String),

    #[error("Policy evaluation failed: {0}")]
    EvaluationFailed(String),

    #[error("Policy violation: {0}")]
    Violation(String),

    #[error("Invalid policy set: {0}")]
    InvalidPolicySet(String),
}

/// Crypto errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),

    #[error("Hash computation failed: {0}")]
    HashFailed(String),

    #[error("Invalid hash length (expected 32 bytes)")]
    InvalidHashLength,

    #[error("Password hashing failed: {0}")]
    PasswordHashFailed(String),

    #[error("Password verification failed")]
    PasswordVerificationFailed,

    #[error("Random generation failed: {0}")]
    RandomGenerationFailed(String),
}

/// Storage errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Redb error: {0}")]
    Redb(#[from] redb::Error),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    #[error("Connection pool exhausted")]
    PoolExhausted,

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
}

/// Runtime errors
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("WASM instantiation failed: {0}")]
    WasmInstantiation(String),

    #[error("WASM execution failed: {0}")]
    WasmExecution(String),

    #[error("Capability host function error: {0}")]
    HostFunction(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Component not found: {0}")]
    ComponentNotFound(String),
}

/// Sync errors
#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Conflict resolution failed: {0}")]
    ConflictResolutionFailed(String),

    #[error("CRDT merge failed: {0}")]
    CrdtMergeFailed(String),
}

/// Serialization errors
#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("Postcard serialization failed: {0}")]
    Postcard(#[from] postcard::Error),

    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("CBOR serialization failed: {0}")]
    Cbor(#[from] serde_cbor::Error),
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required config: {0}")]
    Missing(String),

    #[error("Invalid config value: {key} = {value}")]
    InvalidValue { key: String, value: String },

    #[error("Config file not found: {0}")]
    FileNotFound(String),

    #[error("Config parse failed: {0}")]
    ParseFailed(String),
}