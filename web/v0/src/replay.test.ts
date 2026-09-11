import assert from "node:assert/strict";
import test from "node:test";
import { foldEvents, snapshotsMatch } from "./replay.ts";

test("compile-only fold stays compiled", () => {
  const s = foldEvents([{ kind: "IntentCompiled", payload: {} }]);
  assert.equal(s.status, "compiled");
  assert.equal(s.output, null);
  assert.equal(s.simulation, null);
});

test("full happy path reconstructs execute", () => {
  const s = foldEvents([
    { kind: "IntentProposed", payload: { proposer: "local", canonical: "Echo this message: ping" } },
    { kind: "IntentCompiled", payload: { proposer: "local" } },
    {
      kind: "IntentSimulated",
      payload: { success_probability: 1, risk: 0, duration_ms: 10, notes: ["Capability cap.echo allowed."] },
    },
    { kind: "IntentApproved", payload: {} },
    { kind: "TokenIssued", payload: { token_id: "t1", fingerprint: "abc" } },
    { kind: "TaskStarted", payload: { task_id: "x" } },
    { kind: "TaskSucceeded", payload: { output: "ping" } },
    { kind: "IntentReplayed", payload: { matches_store: true } },
  ]);
  assert.equal(s.status, "executed");
  assert.equal(s.output, "ping");
  assert.equal(s.proposer, "local");
  assert.equal(s.token?.fingerprint, "abc");
  assert.equal(s.token?.revoked, false);
  assert.equal(s.simulation?.success_probability, 1);
  assert.equal(snapshotsMatch(s, { status: "executed", output: "ping" }), true);
});

test("failure and revoke are visible in the fold", () => {
  const s = foldEvents([
    { kind: "IntentCompiled", payload: {} },
    { kind: "IntentSimulated", payload: { success_probability: 1, risk: 0, duration_ms: 10, notes: [] } },
    { kind: "IntentApproved", payload: {} },
    { kind: "TokenIssued", payload: { token_id: "t1", fingerprint: "abc" } },
    { kind: "TokenRevoked", payload: { token_id: "t1" } },
    { kind: "TaskFailed", payload: { reason: "revoked" } },
  ]);
  assert.equal(s.status, "failed");
  assert.equal(s.fail_reason, "revoked");
  assert.equal(s.token?.revoked, true);
  assert.equal(snapshotsMatch(s, { status: "failed", output: null }), true);
});
