# Intent Intermediate Representation (IR) Specification

## Overview

The Intent IR is the executable specification of user intent. It is:
- **Compiled** from natural language
- **Validated** against policy and constraints
- **Simulated** before execution
- **Versioned** and signed
- **Executed** by the DAG scheduler

## Structure

```json
{
  "id": "IntentId",
  "name": "string",
  "nl_source": "string",
  "goal": "Goal",
  "constraints": ["Constraint"],
  "success_criteria": ["SuccessCriterion"],
  "tasks": ["Task"],
  "risk_score": "f32 [0.0, 1.0]",
  "requires_approval": "bool",
  "version": "u32",
  "compiled_at": "DateTime<Utc>",
  "compiler_version": "string",
  "hash": "Hash (computed)"
}
```

## Types

### Goal
```json
{
  "description": "string",
  "category": "CodeGeneration | Refactoring | Migration | Deployment | Investigation | Automation | Analysis | Custom",
  "priority": "Low | Normal | High | Critical"
}
```

### Constraint
```json
{
  "id": "string",
  "kind": "ZeroDowntime | BudgetLimit | Compliance | SecurityReview | BackwardCompatibility | PerformanceTarget | DataResidency | Custom",
  "parameters": "Map<string, JSON>",
  "severity": "Hard | Soft | Advisory"
}
```

### SuccessCriterion
```json
{
  "id": "string",
  "metric": "TestPassRate | LatencyP50 | LatencyP99 | ErrorRate | Throughput | CostPerHour | MemoryUsage | CPUUsage | SecurityScore | ComplianceScore | Custom",
  "threshold": {
    "operator": "LessThan | LessThanOrEqual | GreaterThan | GreaterThanOrEqual | Equal | NotEqual",
    "value": "f64",
    "unit": "string"
  },
  "weight": "f32"
}
```

### Task
```json
{
  "id": "TaskId",
  "name": "string",
  "description": "string",
  "kind": "Discovery | Design | CodeGeneration | Testing | Simulation | Deployment | Verification | Rollback | Notification | Custom",
  "capabilities": ["string"],
  "dependencies": ["TaskId"],
  "estimated_duration_ms": "u64?",
  "retry_policy": "RetryPolicy",
  "compensation": "CompensationAction?"
}
```

### RetryPolicy
```json
{
  "max_attempts": "u32",
  "backoff_ms": "u64",
  "backoff_multiplier": "f32",
  "max_backoff_ms": "u64",
  "retryable_errors": ["string"]
}
```

### CompensationAction
```json
{
  "action_type": "Undo | Reverse | Compensate | Notify",
  "target_task": "TaskId",
  "payload": "JSON"
}
```

## Validation Rules

1. **Non-empty tasks**: At least one task required
2. **Risk score bounds**: 0.0 ≤ risk_score ≤ 1.0
3. **Version match**: Must equal `PROTOCOL_VERSION`
4. **DAG acyclicity**: Task dependencies must form a DAG
5. **Dependency existence**: All referenced TaskIds must exist
6. **Capability resolution**: All declared capabilities must be available or synthesizable

## Compilation Pipeline

```
NL Input
    │
    ▼
┌─────────────────────┐
│ Context Gathering   │
│ - Codebase AST      │
│ - Infra topology    │
│ - Team ownership    │
│ - Historical patterns│
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ LLM Compilation     │
│ - Goal extraction   │
│ - Constraint mining │
│ - Task decomposition│
│ - Risk estimation   │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ Validation          │
│ - Schema validation │
│ - Policy check      │
│ - DAG verification  │
│ - Capability check  │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ Signing & Storage   │
│ - Compute hash      │
│ - Sign with device  │
│ - Store in event log│
└─────────────────────┘
```

## Serialization

- **Binary**: Postcard (compact, fast, schema-less)
- **Human**: JSON (debugging, API)
- **Storage**: Event store with Postcard payload

## Versioning

- `version` field tracks IR schema version
- Current: `PROTOCOL_VERSION = 1`
- Backward compatibility: Readers must handle older versions
- Migration: Automatic upcast on load

## Examples

### Simple: Create REST API
```json
{
  "id": "01HXK3JQ9V...",
  "name": "Create user REST API",
  "nl_source": "Create a REST API for user management with PostgreSQL",
  "goal": {
    "description": "Create user REST API with CRUD operations",
    "category": "CodeGeneration",
    "priority": "Normal"
  },
  "constraints": [
    {"id": "c1", "kind": "BudgetLimit", "parameters": {"max_hours": 2}, "severity": "Hard"}
  ],
  "success_criteria": [
    {"id": "sc1", "metric": "TestPassRate", "threshold": {"operator": "GreaterThanOrEqual", "value": 0.95, "unit": "ratio"}, "weight": 1.0}
  ],
  "tasks": [
    {"id": "t1", "name": "Design schema", "kind": "Design", "capabilities": ["postgres"], "dependencies": []},
    {"id": "t2", "name": "Generate models", "kind": "CodeGeneration", "capabilities": ["sqlalchemy"], "dependencies": ["t1"]},
    {"id": "t3", "name": "Generate routes", "kind": "CodeGeneration", "capabilities": ["fastapi"], "dependencies": ["t1"]},
    {"id": "t4", "name": "Write tests", "kind": "Testing", "capabilities": ["pytest"], "dependencies": ["t2", "t3"]},
    {"id": "t5", "name": "Run tests", "kind": "Testing", "capabilities": ["pytest"], "dependencies": ["t4"]}
  ],
  "risk_score": 0.2,
  "requires_approval": false,
  "version": 1,
  "compiled_at": "2024-01-15T10:30:00Z",
  "compiler_version": "imperium-intent-0.1.0-dev"
}
```

### Complex: Migration with Zero Downtime
```json
{
  "id": "01HXK3JQ9W...",
  "name": "Migrate auth to passkeys",
  "nl_source": "Migrate auth to passkeys, zero downtime, under 2hrs",
  "goal": {
    "description": "Migrate authentication from passwords to passkeys",
    "category": "Migration",
    "priority": "Critical"
  },
  "constraints": [
    {"id": "c1", "kind": "ZeroDowntime", "parameters": {}, "severity": "Hard"},
    {"id": "c2", "kind": "BackwardCompatibility", "parameters": {"days": 30}, "severity": "Hard"},
    {"id": "c3", "kind": "SecurityReview", "parameters": {"required": true}, "severity": "Hard"},
    {"id": "c4", "kind": "BudgetLimit", "parameters": {"max_hours": 2}, "severity": "Soft"}
  ],
  "success_criteria": [
    {"id": "sc1", "metric": "TestPassRate", "threshold": {"operator": "GreaterThanOrEqual", "value": 0.99, "unit": "ratio"}, "weight": 1.0},
    {"id": "sc2", "metric": "LatencyP99", "threshold": {"operator": "LessThan", "value": 200, "unit": "ms"}, "weight": 0.5},
    {"id": "sc3", "metric": "ErrorRate", "threshold": {"operator": "LessThan", "value": 0.001, "unit": "ratio"}, "weight": 0.5}
  ],
  "tasks": [
    {"id": "t1", "name": "Discover auth surface", "kind": "Discovery", "capabilities": ["code-graph", "git-history"], "dependencies": []},
    {"id": "t2", "name": "Design passkey schema", "kind": "Design", "capabilities": ["db-schema", "threat-model"], "dependencies": ["t1"]},
    {"id": "t3", "name": "Generate migration", "kind": "CodeGeneration", "capabilities": ["sql-migration", "test-gen"], "dependencies": ["t2"]},
    {"id": "t4", "name": "Simulate rollout", "kind": "Simulation", "capabilities": ["staging-sim", "chaos-test"], "dependencies": ["t3"]},
    {"id": "t5", "name": "Deploy to staging", "kind": "Deployment", "capabilities": ["k8s", "argo"], "dependencies": ["t4"]},
    {"id": "t6", "name": "Verify staging", "kind": "Verification", "capabilities": ["load-test", "security-scan"], "dependencies": ["t5"]},
    {"id": "t7", "name": "Canary production", "kind": "Deployment", "capabilities": ["k8s", "flagger"], "dependencies": ["t6"]},
    {"id": "t8", "name": "Full cutover", "kind": "Deployment", "capabilities": ["k8s"], "dependencies": ["t7"]},
    {"id": "t9", "name": "Create PR", "kind": "Notification", "capabilities": ["github"], "dependencies": ["t8"]}
  ],
  "risk_score": 0.7,
  "requires_approval": true,
  "version": 1,
  "compiled_at": "2024-01-15T10:30:00Z",
  "compiler_version": "imperium-intent-0.1.0-dev"
}
```