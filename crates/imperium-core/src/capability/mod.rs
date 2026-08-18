//! Capability System
//!
//! Capabilities are the atomic units of execution in IMPERIUM.
//! Each capability is a WASM component with declared permissions.

use crate::crypto::Hash;
use crate::policy::PolicyId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique capability identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct CapabilityId(pub Uuid);

impl CapabilityId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CapabilityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Capability manifest - declares what the capability can do
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct CapabilityManifest {
    /// Unique identifier
    pub id: CapabilityId,
    /// Human-readable name
    pub name: String,
    /// Version (semver)
    pub version: String,
    /// Description
    pub description: String,
    /// Author
    pub author: String,
    /// Repository URL
    pub repository: Option<String>,
    /// License
    pub license: String,
    /// Declared capabilities (what this component can do)
    pub capabilities: DeclaredCapabilities,
    /// WASM component hash
    pub wasm_hash: Hash,
    /// WASM component size (bytes)
    pub wasm_size: u64,
    /// Entry point (function name)
    pub entry_point: String,
    /// Interface definition (WIT)
    pub wit: String,
    /// Dependencies on other capabilities
    pub dependencies: Vec<CapabilityDependency>,
    /// Minimum IMPERIUM protocol version
    pub min_protocol_version: u32,
    /// Tags for discovery
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Signature (sigstore)
    pub signature: Option<String>,
}

/// What capabilities this component declares it needs
#[derive(Debug, Clone, Default, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct DeclaredCapabilities {
    /// Network access (host:port patterns)
    pub network: Vec<NetworkCapability>,
    /// Vault access (path patterns with permissions)
    pub vault: Vec<VaultCapability>,
    /// Shell command access
    pub shell: Vec<ShellCapability>,
    /// Secret access (by name)
    pub secrets: Vec<SecretCapability>,
    /// Resource limits
    pub resources: ResourceLimits,
    /// Filesystem access
    pub filesystem: Vec<FilesystemCapability>,
    /// Custom capabilities
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct NetworkCapability {
    pub host_pattern: String,  // e.g., "api.github.com", "*.example.com"
    pub port: Option<u16>,     // None = any
    pub protocol: NetworkProtocol,
    pub tls_required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum NetworkProtocol {
    TCP,
    UDP,
    HTTP,
    HTTPS,
    WebSocket,
    GRPC,
}

#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct VaultCapability {
    pub path_pattern: String,  // e.g., "notes/*", "projects/secret/*"
    pub permissions: Vec<VaultPermission>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum VaultPermission {
    Read,
    Write,
    Delete,
    List,
    Search,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct ShellCapability {
    pub command: String,        // e.g., "git", "gh", "kubectl"
    pub args_pattern: Vec<String>, // Patterns like ["commit", "-m", "*"]
    pub working_dir: Option<String>,
    pub timeout_ms: u64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct SecretCapability {
    pub name: String,           // e.g., "GITHUB_TOKEN", "AWS_SECRET_KEY"
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct FilesystemCapability {
    pub path: String,
    pub permissions: Vec<FilesystemPermission>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum FilesystemPermission {
    Read,
    Write,
    Execute,
    Create,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_ms_per_invocation: u64,
    pub max_memory_bytes: u64,
    pub max_execution_time_ms: u64,
    pub max_concurrent_invocations: u32,
    pub max_network_bytes_per_sec: u64,
    pub max_filesystem_bytes_per_sec: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_ms_per_invocation: 5000,
            max_memory_bytes: 128 * 1024 * 1024, // 128 MB
            max_execution_time_ms: 30000,
            max_concurrent_invocations: 10,
            max_network_bytes_per_sec: 10 * 1024 * 1024, // 10 MB/s
            max_filesystem_bytes_per_sec: 50 * 1024 * 1024, // 50 MB/s
        }
    }
}

/// Dependency on another capability
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct CapabilityDependency {
    pub capability_id: CapabilityId,
    pub version_range: String, // semver range
    pub optional: bool,
}

/// Unforgeable capability token issued at runtime
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct CapabilityToken {
    /// Token ID
    pub id: TokenId,
    /// Capability this token grants access to
    pub capability_id: CapabilityId,
    /// Specific permissions granted (subset of manifest)
    pub permissions: GrantedPermissions,
    /// Issued to (actor)
    pub subject: crate::event::ActorId,
    /// Issued by (policy engine)
    pub issuer: crate::event::ActorId,
    /// Issued at
    pub issued_at: chrono::DateTime<chrono::Utc>,
    /// Expires at
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Nonce for replay protection
    pub nonce: [u8; 32],
    /// Signature
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct TokenId(pub Uuid);

impl TokenId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TokenId {
    fn default() -> Self {
        Self::new()
    }
}

/// Granted permissions (validated subset of declared)
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct GrantedPermissions {
    pub network: Vec<NetworkCapability>,
    pub vault: Vec<VaultCapability>,
    pub shell: Vec<ShellCapability>,
    pub secrets: Vec<SecretCapability>,
    pub filesystem: Vec<FilesystemCapability>,
    pub resources: ResourceLimits,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Capability registry entry
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct RegistryEntry {
    pub manifest: CapabilityManifest,
    pub wasm_bytes: Vec<u8>, // Stored separately in production
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub installed_by: crate::event::ActorId,
    pub status: CapabilityStatus,
    pub verification: VerificationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum CapabilityStatus {
    Available,
    Deprecated,
    Disabled,
    Quarantined,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum VerificationStatus {
    Unverified,
    Verified,
    Failed(String),
    InProgress,
}