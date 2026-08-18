"""Intent Compiler - NL to IR"""

from pydantic import BaseModel, Field
from typing import List, Optional, Dict, Any
from uuid import UUID
from enum import Enum

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
    parameters: Dict[str, Any] = {}
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
    retryable_errors: List[str] = ["timeout", "unavailable"]

class CompensationType(str, Enum):
    UNDO = "Undo"
    REVERSE = "Reverse"
    COMPENSATE = "Compensate"
    NOTIFY = "Notify"

class CompensationAction(BaseModel):
    action_type: CompensationType
    target_task: UUID
    payload: Dict[str, Any]

class Task(BaseModel):
    id: UUID
    name: str
    description: str
    kind: TaskKind
    capabilities: List[str] = []
    dependencies: List[UUID] = []
    estimated_duration_ms: Optional[int] = None
    retry_policy: RetryPolicy = RetryPolicy()
    compensation: Optional[CompensationAction] = None

class IntentIR(BaseModel):
    id: UUID
    name: str
    nl_source: str
    goal: Goal
    constraints: List[Constraint] = []
    success_criteria: List[SuccessCriterion] = []
    tasks: List[Task] = []
    risk_score: float = 0.0
    requires_approval: bool = False
    version: int = 1

class IntentCompiler:
    """Compiles natural language to Intent IR."""
    
    def __init__(self):
        pass
    
    async def compile(self, nl: str, context: Dict[str, Any]) -> IntentIR:
        """Compile natural language to Intent IR."""
        # TODO: Implement LLM-based compilation
        raise NotImplementedError("Intent compilation not yet implemented")