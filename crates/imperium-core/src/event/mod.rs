//! Event System
//!
//! Append-only event store with causal ordering.
//! Every state change in IMPERIUM is an event.

use crate::intent::{IntentId, TaskId};
use crate::crypto::Hash;
use crate::capability::CapabilityId;
use crate::policy::PolicyId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique event identifier (ULID for time-ordered uniqueness)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct EventId(pub ulid::Ulid);

impl EventId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Event envelope with metadata
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct Event {
    /// Unique event ID
    pub id: EventId,
    /// Event type for routing
    pub event_type: EventType,
    /// Payload
    pub payload: EventPayload,
    /// Timestamp (UTC)
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Causation ID (what caused this event)
    pub causation_id: Option<EventId>,
    /// Correlation ID (groups related events)
    pub correlation_id: Option<EventId>,
    /// Actor that initiated the event
    pub actor: Actor,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Hash of payload for integrity
    #[serde(skip)]
    pub payload_hash: Option<Hash>,
}

impl Event {
    pub fn new(event_type: EventType, payload: EventPayload, actor: Actor) -> Self {
        let mut event = Self {
            id: EventId::new(),
            event_type,
            payload,
            timestamp: chrono::Utc::now(),
            causation_id: None,
            correlation_id: None,
            actor,
            metadata: HashMap::new(),
            payload_hash: None,
        };
        event.compute_hash();
        event
    }

    pub fn with_causation(mut self, causation_id: EventId) -> Self {
        self.causation_id = Some(causation_id);
        self
    }

    pub fn with_correlation(mut self, correlation_id: EventId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn compute_hash(&mut self) {
        let payload_bytes = postcard::to_stdvec(&self.payload).expect("payload serializable");
        self.payload_hash = Some(Hash::blake3(&payload_bytes));
    }

    pub fn verify(&self) -> bool {
        if let Some(hash) = &self.payload_hash {
            let payload_bytes = postcard::to_stdvec(&self.payload).expect("payload serializable");
            *hash == Hash::blake3(&payload_bytes)
        } else {
            false
        }
    }
}

/// Event type for routing and filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum EventType {
    // Intent lifecycle
    IntentCreated,
    IntentCompiled,
    IntentValidated,
    IntentApproved,
    IntentRejected,
    IntentExecuted,
    IntentCompleted,
    IntentFailed,
    IntentRolledBack,

    // Task lifecycle
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    TaskRetried,
    TaskCompensated,

    // Simulation
    SimulationStarted,
    SimulationCompleted,
    SimulationFailed,
    CounterfactualQueried,
    CounterfactualResolved,

    // Capability synthesis
    CapabilitySynthesized,
    CapabilityTested,
    CapabilityRegistered,
    CapabilityUpdated,
    CapabilityRemoved,

    // Evolution
    FrictionDetected,
    PatchGenerated,
    ShadowDeployed,
    PatchValidated,
    PatchPromoted,
    PatchDiscarded,

    // World model
    WorldModelUpdated,
    CausalLinkDiscovered,
    AnomalyDetected,

    // Sync
    SyncStarted,
    SyncCompleted,
    SyncConflict,
    SyncResolved,

    // Policy
    PolicyLoaded,
    PolicyEvaluated,
    PolicyViolated,
    PolicyUpdated,

    // System
    DaemonStarted,
    DaemonStopped,
    HealthCheck,
    ErrorOccurred,
}

/// Event payloads (discriminated union)
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EventPayload {
    IntentCreated { intent_id: IntentId, name: String, nl_source: String },
    IntentCompiled { intent_id: IntentId, ir_hash: Hash },
    IntentValidated { intent_id: IntentId, valid: bool, errors: Vec<String> },
    IntentApproved { intent_id: IntentId, approver: Actor },
    IntentRejected { intent_id: IntentId, reason: String },
    IntentExecuted { intent_id: IntentId, branch: String },
    IntentCompleted { intent_id: IntentId, duration_ms: u64 },
    IntentFailed { intent_id: IntentId, error: String, failed_task: Option<TaskId> },
    IntentRolledBack { intent_id: IntentId, reason: String, completed_tasks: Vec<TaskId> },

    TaskStarted { intent_id: IntentId, task_id: TaskId },
    TaskCompleted { intent_id: IntentId, task_id: TaskId, outputs: serde_json::Value },
    TaskFailed { intent_id: IntentId, task_id: TaskId, error: String, attempt: u32 },
    TaskRetried { intent_id: IntentId, task_id: TaskId, attempt: u32 },
    TaskCompensated { intent_id: IntentId, task_id: TaskId, compensation_type: String },

    SimulationStarted { intent_id: IntentId, rollouts: u32 },
    SimulationCompleted { intent_id: IntentId, success_probability: f32, duration_ms: u64 },
    SimulationFailed { intent_id: IntentId, error: String },
    CounterfactualQueried { intent_id: IntentId, query: String },
    CounterfactualResolved { intent_id: IntentId, query: String, result: serde_json::Value },

    CapabilitySynthesized { capability_id: CapabilityId, spec_hash: Hash },
    CapabilityTested { capability_id: CapabilityId, passed: bool, results: serde_json::Value },
    CapabilityRegistered { capability_id: CapabilityId, manifest_hash: Hash },
    CapabilityUpdated { capability_id: CapabilityId, version: String },
    CapabilityRemoved { capability_id: CapabilityId, reason: String },

    FrictionDetected { pattern: String, frequency: u32, impact: f32 },
    PatchGenerated { patch_id: Uuid, target: String, changes: serde_json::Value },
    ShadowDeployed { patch_id: Uuid, deployment_id: Uuid },
    PatchValidated { patch_id: Uuid, metrics: serde_json::Value },
    PatchPromoted { patch_id: Uuid },
    PatchDiscarded { patch_id: Uuid, reason: String },

    WorldModelUpdated { entity: String, changes: serde_json::Value },
    CausalLinkDiscovered { from: String, to: String, strength: f32 },
    AnomalyDetected { metric: String, expected: f64, actual: f64, severity: String },

    SyncStarted { peer_id: String },
    SyncCompleted { peer_id: String, events_synced: u32 },
    SyncConflict { peer_id: String, event_ids: Vec<EventId> },
    SyncResolved { peer_id: String, resolution: String },

    PolicyLoaded { policy_id: PolicyId, version: String },
    PolicyEvaluated { policy_id: PolicyId, decision: String, reason: String },
    PolicyViolated { policy_id: PolicyId, violation: String },
    PolicyUpdated { policy_id: PolicyId, version: String },

    DaemonStarted { version: String, config_hash: Hash },
    DaemonStopped { reason: String },
    HealthCheck { status: String, details: serde_json::Value },
    ErrorOccurred { error: String, context: serde_json::Value },
}

/// Actor that initiates events
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct Actor {
    pub id: ActorId,
    pub kind: ActorKind,
    pub name: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum ActorKind {
    User,
    System,
    Daemon,
    Plugin,
    Scheduler,
    Synthesizer,
    Simulator,
    EvolutionLoop,
    External,
}

/// Actor identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct ActorId(pub Uuid);

impl ActorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ActorId {
    fn default() -> Self {
        Self::new()
    }
}

/// Event stream for a specific aggregate (intent, capability, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct EventStream {
    pub aggregate_id: String,
    pub aggregate_type: AggregateType,
    pub events: Vec<Event>,
    pub last_event_id: Option<EventId>,
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum AggregateType {
    Intent,
    Capability,
    WorldModel,
    Policy,
    Sync,
    System,
}