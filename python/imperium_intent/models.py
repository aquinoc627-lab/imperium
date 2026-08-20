"""Intent IR models aligned with schemas/intent_ir.schema.json."""

from datetime import datetime
from enum import Enum
from typing import Any
from uuid import UUID

from pydantic import BaseModel, Field, field_validator


class GoalCategory(str, Enum):
    CODE_GENERATION = "CodeGeneration"
    REFACTORING = "Refactoring"
    MIGRATION = "Migration"
    DEPLOYMENT = "Deployment"
    INVESTIGATION = "Investigation"
    AUTOMATION = "Automation"
    ANALYSIS = "Analysis"
    CUSTOM = "Custom"


class Priority(str, Enum):
    LOW = "Low"
    NORMAL = "Normal"
    HIGH = "High"
    CRITICAL = "Critical"


class Goal(BaseModel):
    description: str
    category: GoalCategory
    priority: Priority = Priority.NORMAL


class ConstraintKind(str, Enum):
    ZERO_DOWNTIME = "ZeroDowntime"
    BUDGET_LIMIT = "BudgetLimit"
    COMPLIANCE = "Compliance"
    SECURITY_REVIEW = "SecurityReview"
    BACKWARD_COMPATIBILITY = "BackwardCompatibility"
    PERFORMANCE_TARGET = "PerformanceTarget"
    DATA_RESIDENCY = "DataResidency"
    CUSTOM = "Custom"


class ConstraintSeverity(str, Enum):
    HARD = "Hard"
    SOFT = "Soft"
    ADVISORY = "Advisory"


class Constraint(BaseModel):
    id: str
    kind: ConstraintKind
    parameters: dict[str, Any] = Field(default_factory=dict)
    severity: ConstraintSeverity = ConstraintSeverity.HARD


class Metric(str, Enum):
    TEST_PASS_RATE = "TestPassRate"
    LATENCY_P50 = "LatencyP50"
    LATENCY_P99 = "LatencyP99"
    ERROR_RATE = "ErrorRate"
    THROUGHPUT = "Throughput"
    COST_PER_HOUR = "CostPerHour"
    MEMORY_USAGE = "MemoryUsage"
    CPU_USAGE = "CPUUsage"
    SECURITY_SCORE = "SecurityScore"
    COMPLIANCE_SCORE = "ComplianceScore"
    CUSTOM = "Custom"


class ThresholdOperator(str, Enum):
    LESS_THAN = "LessThan"
    LESS_THAN_OR_EQUAL = "LessThanOrEqual"
    GREATER_THAN = "GreaterThan"
    GREATER_THAN_OR_EQUAL = "GreaterThanOrEqual"
    EQUAL = "Equal"
    NOT_EQUAL = "NotEqual"


class Threshold(BaseModel):
    operator: ThresholdOperator
    value: float
    unit: str


class SuccessCriterion(BaseModel):
    id: str
    metric: Metric
    threshold: Threshold
    weight: float = 1.0


class TaskKind(str, Enum):
    DISCOVERY = "Discovery"
    DESIGN = "Design"
    CODE_GENERATION = "CodeGeneration"
    TESTING = "Testing"
    SIMULATION = "Simulation"
    DEPLOYMENT = "Deployment"
    VERIFICATION = "Verification"
    ROLLBACK = "Rollback"
    NOTIFICATION = "Notification"
    CUSTOM = "Custom"


class RetryPolicy(BaseModel):
    max_attempts: int = 3
    backoff_ms: int = 1000
    backoff_multiplier: float = 2.0
    max_backoff_ms: int = 30000
    retryable_errors: list[str] = Field(default_factory=lambda: ["timeout", "unavailable"])


class CompensationType(str, Enum):
    UNDO = "Undo"
    REVERSE = "Reverse"
    COMPENSATE = "Compensate"
    NOTIFY = "Notify"


class CompensationAction(BaseModel):
    action_type: CompensationType
    target_task: UUID
    payload: dict[str, Any]


class Task(BaseModel):
    id: UUID
    name: str
    description: str
    kind: TaskKind
    capabilities: list[str] = Field(default_factory=list)
    dependencies: list[UUID] = Field(default_factory=list)
    estimated_duration_ms: int | None = None
    retry_policy: RetryPolicy = Field(default_factory=RetryPolicy)
    compensation: CompensationAction | None = None


class IntentIR(BaseModel):
    id: UUID
    name: str
    nl_source: str
    goal: Goal
    constraints: list[Constraint] = Field(default_factory=list)
    success_criteria: list[SuccessCriterion] = Field(default_factory=list)
    tasks: list[Task] = Field(default_factory=list)
    risk_score: float = 0.0
    requires_approval: bool = False
    version: int = 1
    compiled_at: datetime | None = None
    compiler_version: str | None = None

    @field_validator("risk_score")
    @classmethod
    def _risk_bounds(cls, value: float) -> float:
        if value < 0.0 or value > 1.0:
            raise ValueError("risk_score must be between 0.0 and 1.0")
        return value

    def validate_structure(self) -> None:
        if not self.tasks:
            raise ValueError("Intent has no tasks")
        if self.version != 1:
            raise ValueError(f"Version mismatch: expected 1, found {self.version}")
        ids = {task.id for task in self.tasks}
        for task in self.tasks:
            for dep in task.dependencies:
                if dep not in ids:
                    raise ValueError(f"Missing dependency: {dep}")
        visiting: set[UUID] = set()
        visited: set[UUID] = set()
        by_id = {task.id: task for task in self.tasks}

        def visit(task_id: UUID) -> None:
            visiting.add(task_id)
            for dep in by_id[task_id].dependencies:
                if dep in visiting:
                    raise ValueError(f"Cyclic dependency: {task_id} -> {dep}")
                if dep not in visited:
                    visit(dep)
            visiting.remove(task_id)
            visited.add(task_id)

        for task in self.tasks:
            if task.id not in visited:
                visit(task.id)
