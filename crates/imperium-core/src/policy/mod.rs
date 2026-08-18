//! Policy Engine
//!
//! OPA/Rego embedded for policy evaluation.
//! All decisions are auditable and replayable.

use crate::crypto::Hash;
use crate::event::ActorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique policy identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct PolicyId(pub Uuid);

impl PolicyId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PolicyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PolicyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Policy bundle (Rego + data + metadata)
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct PolicyBundle {
    pub id: PolicyId,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    /// Rego source code
    pub rego: String,
    /// Static data (JSON)
    pub data: serde_json::Value,
    /// Entry point (package.rule)
    pub entry_point: String,
    /// Dependencies on other policies
    pub dependencies: Vec<PolicyId>,
    /// Tags
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub hash: Hash,
}

/// Policy evaluation request
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct EvaluationRequest {
    pub policy_id: PolicyId,
    pub input: serde_json::Value,
    pub context: EvaluationContext,
}

/// Context for policy evaluation
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct EvaluationContext {
    pub actor: ActorId,
    pub intent_id: Option<crate::intent::IntentId>,
    pub capability_id: Option<crate::capability::CapabilityId>,
    pub resource: Option<String>,
    pub action: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Policy decision
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct Decision {
    pub policy_id: PolicyId,
    pub decision: DecisionType,
    pub reason: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub evaluated_at: chrono::DateTime<chrono::Utc>,
    pub evaluation_duration_ms: u64,
    /// Full trace for debugging
    pub trace: Option<Vec<TraceEntry>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum DecisionType {
    Allow,
    Deny,
    Undefined,
    Error,
}

/// Trace entry for debugging
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct TraceEntry {
    pub rule: String,
    pub result: serde_json::Value,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub row: u32,
    pub col: u32,
}

/// Policy set (multiple policies evaluated together)
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct PolicySet {
    pub id: PolicySetId,
    pub name: String,
    pub version: String,
    pub policies: Vec<PolicyId>,
    pub evaluation_strategy: EvaluationStrategy,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct PolicySetId(pub Uuid);

impl PolicySetId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PolicySetId {
    fn default() -> Self {
        Self::new()
    }
}

/// How to combine multiple policy decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum EvaluationStrategy {
    /// First deny wins (default)
    DenyOverrides,
    /// First allow wins
    AllowOverrides,
    /// All must allow
    Unanimous,
    /// Majority allow
    Majority,
    /// Custom (Rego rule)
    Custom(String),
}

/// Built-in policy IDs
pub mod builtin {
    use super::PolicyId;
    use uuid::uuid;

    /// Air-gap enforcement: no cloud routing without explicit approval
    pub const AIR_GAP: PolicyId = PolicyId(uuid!("00000000-0000-0000-0000-000000000001"));

    /// PII redaction: all outbound data must pass through privacy proxy
    pub const PII_REDACTION: PolicyId = PolicyId(uuid!("00000000-0000-0000-0000-000000000002"));

    /// Capability capability: only granted capabilities can be invoked
    pub const CAPABILITY_AUTHZ: PolicyId = PolicyId(uuid!("00000000-0000-0000-0000-000000000003"));

    /// Resource limits: enforce declared resource limits
    pub const RESOURCE_LIMITS: PolicyId = PolicyId(uuid!("00000000-0000-0000-0000-000000000004"));

    /// Intent approval: high-risk intents require human approval
    pub const INTENT_APPROVAL: PolicyId = PolicyId(uuid!("00000000-0000-0000-0000-000000000005"));

    /// Data residency: enforce data locality constraints
    pub const DATA_RESIDENCY: PolicyId = PolicyId(uuid!("00000000-0000-0000-0000-000000000006"));

    /// Audit logging: all decisions must be logged
    pub const AUDIT_LOGGING: PolicyId = PolicyId(uuid!("00000000-0000-0000-0000-000000000007"));
}

/// Default policy set ID
pub const DEFAULT_POLICY_SET: PolicySetId = PolicySetId(uuid::uuid!("00000000-0000-0000-0000-000000000000"));