export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export type IntentStatus =
  | "compiled"
  | "simulated"
  | "approved"
  | "executed"
  | "failed";

export type EventKind =
  | "IntentCompiled"
  | "IntentProposed"
  | "IntentSimulated"
  | "IntentApproved"
  | "TokenIssued"
  | "TokenRevoked"
  | "TaskStarted"
  | "TaskSucceeded"
  | "TaskFailed"
  | "IntentReplayed";

export type GoalCategory =
  | "CodeGeneration"
  | "Refactoring"
  | "Migration"
  | "Deployment"
  | "Investigation"
  | "Automation"
  | "Analysis"
  | "Custom";

export type TaskKind =
  | "Discovery"
  | "Design"
  | "CodeGeneration"
  | "Testing"
  | "Simulation"
  | "Deployment"
  | "Verification"
  | "Rollback"
  | "Notification"
  | "Custom";

export interface Task {
  id: string;
  name: string;
  description: string;
  kind: TaskKind;
  capabilities: string[];
  dependencies: string[];
  estimated_duration_ms: number | null;
  target_path: string | null;
}

export interface IntentIR {
  id: string;
  name: string;
  nl_source: string;
  goal: {
    description: string;
    category: GoalCategory;
    priority: "Low" | "Normal" | "High" | "Critical";
  };
  constraints: JsonValue[];
  success_criteria: Array<{
    id: string;
    metric: string;
    threshold: { operator: string; value: number; unit: string };
    weight: number;
  }>;
  tasks: Task[];
  risk_score: number;
  requires_approval: boolean;
  version: 1;
  compiled_at: string;
  compiler_version: string;
}

export interface SimulationResult {
  success_probability: number;
  risk: number;
  duration_ms: number;
  notes: string[];
}

export interface TokenSummary {
  id: string;
  capability: string;
  fingerprint: string;
  expires_at: string;
  revoked: boolean;
  used: boolean;
  permissions: {
    fs: string[];
    net: string[];
    env: string[];
  };
}

export interface IntentRecord {
  id: string;
  name: string;
  nl_source: string;
  status: IntentStatus;
  ir: IntentIR;
  simulation: SimulationResult | null;
  output: string | null;
  created_at: string;
  updated_at: string;
  token: TokenSummary | null;
}

export interface EventRecord {
  id: string;
  intent_id: string;
  kind: EventKind;
  payload: { [key: string]: JsonValue };
  created_at: string;
}
