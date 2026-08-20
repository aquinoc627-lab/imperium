import type {
  EventKind,
  IntentStatus,
  JsonValue,
  SimulationResult,
} from "./types.ts";

export interface FoldedEvent {
  kind: string;
  payload: { [key: string]: JsonValue };
}

export interface FoldedToken {
  token_id: string;
  fingerprint: string;
  revoked: boolean;
}

export interface FoldedState {
  status: IntentStatus;
  output: string | null;
  simulation: SimulationResult | null;
  token: FoldedToken | null;
  proposer: string | null;
  canonical: string | null;
  fail_reason: string | null;
}

const STATUS_RANK: Record<IntentStatus, number> = {
  compiled: 0,
  simulated: 1,
  approved: 2,
  executed: 3,
  failed: 3,
};

export function foldEvents(events: FoldedEvent[]): FoldedState {
  const state: FoldedState = {
    status: "compiled",
    output: null,
    simulation: null,
    token: null,
    proposer: null,
    canonical: null,
    fail_reason: null,
  };

  for (const ev of events) {
    const p = ev.payload;
    switch (ev.kind as EventKind | "IntentReplayed") {
      case "IntentProposed":
        state.proposer = String(p.proposer ?? "");
        state.canonical = String(p.canonical ?? "");
        break;
      case "IntentCompiled":
        if (typeof p.proposer === "string") state.proposer = p.proposer;
        break;
      case "IntentSimulated": {
        state.status = "simulated";
        const notes = Array.isArray(p.notes)
          ? p.notes.map((n) => String(n))
          : [];
        state.simulation = {
          success_probability: Number(p.success_probability ?? 0),
          risk: Number(p.risk ?? 0),
          duration_ms: Number(p.duration_ms ?? 0),
          notes,
        };
        break;
      }
      case "IntentApproved":
        state.status = "approved";
        break;
      case "TokenIssued":
        state.token = {
          token_id: String(p.token_id ?? ""),
          fingerprint: String(p.fingerprint ?? ""),
          revoked: false,
        };
        break;
      case "TokenRevoked":
        if (state.token) state.token.revoked = true;
        break;
      case "TaskSucceeded":
        state.status = "executed";
        state.output = String(p.output ?? "");
        state.fail_reason = null;
        break;
      case "TaskFailed":
        state.status = "failed";
        state.fail_reason = String(p.reason ?? "failed");
        break;
      default:
        break;
    }
  }
  return state;
}

export function snapshotsMatch(
  folded: FoldedState,
  store: { status: IntentStatus; output: string | null },
): boolean {
  return folded.status === store.status && folded.output === store.output;
}

export function statusAtLeast(current: IntentStatus, min: IntentStatus): boolean {
  return STATUS_RANK[current] >= STATUS_RANK[min];
}
