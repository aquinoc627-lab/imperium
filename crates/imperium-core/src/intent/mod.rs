//! Intent Intermediate Representation (IR)
//!
//! Canonical types for protocol version 1. JSON contract:
//! `schemas/intent_ir.schema.json`.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Unique identifier for an intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Integrity hash of a canonical IR payload (BLAKE3, 32 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentHash(pub [u8; 32]);

/// The executable Intent IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentIR {
    pub id: IntentId,
    pub name: String,
    pub nl_source: String,
    pub goal: Goal,
    pub constraints: Vec<Constraint>,
    pub success_criteria: Vec<SuccessCriterion>,
    pub tasks: Vec<Task>,
    pub risk_score: f32,
    pub requires_approval: bool,
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiled_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiler_version: Option<String>,
    /// Not part of the JSON contract; computed after validation.
    #[serde(skip)]
    pub hash: Option<IntentHash>,
}

impl IntentIR {
    pub fn validate(&self) -> Result<(), IntentValidationError> {
        if self.tasks.is_empty() {
            return Err(IntentValidationError::NoTasks);
        }
        if !(0.0..=1.0).contains(&self.risk_score) {
            return Err(IntentValidationError::InvalidRiskScore(self.risk_score));
        }
        if self.version != crate::PROTOCOL_VERSION {
            return Err(IntentValidationError::VersionMismatch {
                expected: crate::PROTOCOL_VERSION,
                found: self.version,
            });
        }
        self.validate_dag()?;
        Ok(())
    }

    fn validate_dag(&self) -> Result<(), IntentValidationError> {
        let ids: HashSet<TaskId> = self.tasks.iter().map(|t| t.id).collect();
        for task in &self.tasks {
            for dep in &task.dependencies {
                if !ids.contains(dep) {
                    return Err(IntentValidationError::MissingDependency(*dep));
                }
            }
        }

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
                    if rec_stack.contains(dep) {
                        return Err(IntentValidationError::CyclicDependency {
                            task: task_id,
                            dependency: *dep,
                        });
                    }
                    if !visited.contains(dep) {
                        visit(*dep, tasks, visited, rec_stack)?;
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

    /// Hash the JSON form of the IR (hash field excluded by serde skip).
    pub fn compute_hash(&mut self) -> Result<IntentHash, IntentValidationError> {
        let bytes = serde_json::to_vec(self).map_err(|e| {
            IntentValidationError::InvalidTaskReference(e.to_string())
        })?;
        let hash = IntentHash(*blake3::hash(&bytes).as_bytes());
        self.hash = Some(hash);
        Ok(hash)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub description: String,
    pub category: GoalCategory,
    pub priority: Priority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalCategory {
    CodeGeneration,
    Refactoring,
    Migration,
    Deployment,
    Investigation,
    Automation,
    Analysis,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: String,
    pub kind: ConstraintKind,
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
    pub severity: ConstraintSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintSeverity {
    Hard,
    Soft,
    Advisory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriterion {
    pub id: String,
    pub metric: Metric,
    pub threshold: Threshold,
    #[serde(default = "default_weight")]
    pub weight: f32,
}

fn default_weight() -> f32 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threshold {
    pub operator: ThresholdOperator,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThresholdOperator {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub description: String,
    pub kind: TaskKind,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<TaskId>,
    #[serde(default)]
    pub estimated_duration_ms: Option<u64>,
    #[serde(default)]
    pub retry_policy: RetryPolicy,
    #[serde(default)]
    pub compensation: Option<CompensationAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationAction {
    pub action_type: CompensationType,
    pub target_task: TaskId,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompensationType {
    Undo,
    Reverse,
    Compensate,
    Notify,
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/contract/intent_ir")
    }

    fn load(name: &str) -> IntentIR {
        let path = fixture_dir().join(name);
        let data = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("failed to read {}: {e}", path.display())
        });
        serde_json::from_str(&data).expect("fixture deserializes")
    }

    #[test]
    fn echo_v0_fixture_validates() {
        let mut ir = load("echo_v0.json");
        ir.validate().expect("echo_v0 valid");
        ir.compute_hash().expect("hash");
        assert_eq!(ir.tasks.len(), 1);
        assert_eq!(ir.tasks[0].capabilities, vec!["cap.echo"]);
        assert!(ir.requires_approval);
    }

    #[test]
    fn simple_rest_api_fixture_validates() {
        let ir = load("simple_rest_api.json");
        ir.validate().expect("simple_rest_api valid");
        assert_eq!(ir.tasks.len(), 4);
    }

    #[test]
    fn missing_dependency_is_rejected() {
        let mut ir = load("echo_v0.json");
        ir.tasks[0].dependencies.push(TaskId(Uuid::nil()));
        match ir.validate() {
            Err(IntentValidationError::MissingDependency(_)) => {}
            other => panic!("expected MissingDependency, got {other:?}"),
        }
    }

    #[test]
    fn cyclic_dependency_is_rejected() {
        let mut ir = load("echo_v0.json");
        let a = TaskId::new();
        let b = TaskId::new();
        ir.tasks = vec![
            Task {
                id: a,
                name: "a".into(),
                description: "a".into(),
                kind: TaskKind::Custom,
                capabilities: vec!["cap.echo".into()],
                dependencies: vec![b],
                estimated_duration_ms: None,
                retry_policy: RetryPolicy::default(),
                compensation: None,
            },
            Task {
                id: b,
                name: "b".into(),
                description: "b".into(),
                kind: TaskKind::Custom,
                capabilities: vec!["cap.echo".into()],
                dependencies: vec![a],
                estimated_duration_ms: None,
                retry_policy: RetryPolicy::default(),
                compensation: None,
            },
        ];
        match ir.validate() {
            Err(IntentValidationError::CyclicDependency { .. }) => {}
            other => panic!("expected CyclicDependency, got {other:?}"),
        }
    }

    #[test]
    fn empty_tasks_rejected() {
        let mut ir = load("echo_v0.json");
        ir.tasks.clear();
        assert!(matches!(ir.validate(), Err(IntentValidationError::NoTasks)));
    }
}
