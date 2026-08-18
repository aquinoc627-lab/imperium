//! Intent Intermediate Representation (IR)
//!
//! The executable specification of user intent.
//! Compiled from natural language, validated, simulated, then executed.

use crate::event::EventId;
use crate::crypto::Hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for an intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct IntentId(pub Uuid);

impl IntentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for IntentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for IntentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a task within an intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// The executable Intent IR - compiled from NL, ready for simulation/execution
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct IntentIR {
    /// Unique identifier
    pub id: IntentId,
    /// Human-readable name
    pub name: String,
    /// Natural language source
    pub nl_source: String,
    /// The goal statement
    pub goal: Goal,
    /// Constraints that must be satisfied
    pub constraints: Vec<Constraint>,
    /// Success criteria (measurable)
    pub success_criteria: Vec<SuccessCriterion>,
    /// Decomposed tasks (DAG)
    pub tasks: Vec<Task>,
    /// Risk score [0.0, 1.0]
    pub risk_score: f32,
    /// Whether human approval required before execution
    pub requires_approval: bool,
    /// Protocol version
    pub version: u32,
    /// Compilation timestamp
    pub compiled_at: chrono::DateTime<chrono::Utc>,
    /// Compiler version
    pub compiler_version: String,
    /// Hash of the IR for integrity
    #[serde(skip)]
    pub hash: Option<Hash>,
}

impl IntentIR {
    /// Validate the IR structure
    pub fn validate(&self) -> Result<(), IntentValidationError> {
        if self.tasks.is_empty() {
            return Err(IntentValidationError::NoTasks);
        }
        if self.risk_score < 0.0 || self.risk_score > 1.0 {
            return Err(IntentValidationError::InvalidRiskScore(self.risk_score));
        }
        if self.version != crate::PROTOCOL_VERSION {
            return Err(IntentValidationError::VersionMismatch {
                expected: crate::PROTOCOL_VERSION,
                found: self.version,
            });
        }
        // Check task DAG for cycles
        self.validate_dag()?;
        Ok(())
    }

    fn validate_dag(&self) -> Result<(), IntentValidationError> {
        use std::collections::HashSet;
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        fn visit(
            task_id: TaskId,
            tasks: &[Task],
            visited: &mut HashSet<TaskId>,
            rec_stack: &mut HashSet<TaskId>,
        ) -> Result<(), IntentValidationError> {
            visited.insert(task_id);
            rec_stack.insert(task_id);

            if let Some(task) = tasks.iter().find(|t| t.id == task_id) {
                for dep in &task.dependencies {
                    if !visited.contains(dep) {
                        visit(*dep, tasks, visited, rec_stack)?;
                    } else if rec_stack.contains(dep) {
                        return Err(IntentValidationError::CyclicDependency {
                            task: task_id,
                            dependency: *dep,
                        });
                    }
                }
            }

            rec_stack.remove(&task_id);
            Ok(())
        }

        for task in &self.tasks {
            if !visited.contains(&task.id) {
                visit(task.id, &self.tasks, &mut visited, &mut rec_stack)?;
            }
        }
        Ok(())
    }

    /// Compute hash of the IR
    pub fn compute_hash(&mut self) -> Hash {
        let bytes = postcard::to_stdvec(self).expect("IR serializable");
        self.hash = Some(Hash::blake3(&bytes));
        self.hash.unwrap()
    }
}

/// High-level goal statement
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct Goal {
    pub description: String,
    pub category: GoalCategory,
    pub priority: Priority,
}

/// Goal categories for routing and prioritization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum GoalCategory {
    CodeGeneration,
    Refactoring,
    Migration,
    Deployment,
    Investigation,
    Automation,
    Analysis,
    Custom(String),
}

/// Priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Constraint that must be satisfied
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct Constraint {
    pub id: String,
    pub kind: ConstraintKind,
    pub parameters: HashMap<String, serde_json::Value>,
    pub severity: ConstraintSeverity,
}

/// Types of constraints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum ConstraintKind {
    ZeroDowntime,
    BudgetLimit,
    Compliance,
    SecurityReview,
    BackwardCompatibility,
    PerformanceTarget,
    DataResidency,
    Custom,
}

/// Constraint severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum ConstraintSeverity {
    Hard,    // Must satisfy
    Soft,    // Should satisfy, penalty if not
    Advisory, // Informational
}

/// Measurable success criterion
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct SuccessCriterion {
    pub id: String,
    pub metric: Metric,
    pub threshold: Threshold,
    pub weight: f32, // For composite scoring
}

/// Metrics that can be measured
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum Metric {
    TestPassRate,
    LatencyP50,
    LatencyP99,
    ErrorRate,
    Throughput,
    CostPerHour,
    MemoryUsage,
    CPUUsage,
    SecurityScore,
    ComplianceScore,
    Custom(String),
}

/// Threshold for a metric
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct Threshold {
    pub operator: ThresholdOperator,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum ThresholdOperator {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
    NotEqual,
}

/// A single task in the intent DAG
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub description: String,
    pub kind: TaskKind,
    pub capabilities: Vec<String>, // Required capability names
    pub dependencies: Vec<TaskId>, // Must complete before this task
    pub estimated_duration_ms: Option<u64>,
    pub retry_policy: RetryPolicy,
    pub compensation: Option<CompensationAction>, // For saga pattern
}

/// Types of tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum TaskKind {
    Discovery,
    Design,
    CodeGeneration,
    Testing,
    Simulation,
    Deployment,
    Verification,
    Rollback,
    Notification,
    Custom,
}

/// Retry policy for task execution
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff_ms: u64,
    pub backoff_multiplier: f32,
    pub max_backoff_ms: u64,
    pub retryable_errors: Vec<String>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_ms: 1000,
            backoff_multiplier: 2.0,
            max_backoff_ms: 30000,
            retryable_errors: vec!["timeout".into(), "unavailable".into()],
        }
    }
}

/// Compensation action for saga rollback
#[derive(Debug, Clone, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub struct CompensationAction {
    pub action_type: CompensationType,
    pub target_task: TaskId,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, postcard::Serialize, postcard::Deserialize)]
pub enum CompensationType {
    Undo,
    Reverse,
    Compensate,
    Notify,
}

/// Validation errors for Intent IR
#[derive(Debug, Clone, thiserror::Error)]
pub enum IntentValidationError {
    #[error("Intent has no tasks")]
    NoTasks,
    #[error("Invalid risk score: {0} (must be 0.0-1.0)")]
    InvalidRiskScore(f32),
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },
    #[error("Cyclic dependency detected: task {task} depends on {dependency}")]
    CyclicDependency { task: TaskId, dependency: TaskId },
    #[error("Missing dependency: {0}")]
    MissingDependency(TaskId),
    #[error("Invalid task reference: {0}")]
    InvalidTaskReference(String),
}