import assert from "node:assert/strict";
import test from "node:test";
import { compileRules } from "./compiler.ts";
import { foldEvents, snapshotsMatch } from "./replay.ts";
import { simulateStatic } from "./simulate.ts";
import { grantForCapability, issueToken, verifyToken } from "./token.ts";
import { rightsFromToken, runGuest } from "./wasm-host.ts";

test("echo loop: compile → simulate → token → wasm → fold", async () => {
  const compiled = compileRules("Echo this message: ping");
  assert.equal(compiled.ok, true);
  if (!compiled.ok) return;

  const sim = simulateStatic(compiled.ir);
  assert.equal(sim.success_probability, 1);
  assert.equal(sim.risk, 0);

  const now = Date.now();
  const token = await issueToken(
    {
      capability: "cap.echo",
      subject: "tester",
      intent_id: compiled.ir.id,
      permissions: grantForCapability("cap.echo"),
      expires_at: now + 60_000,
    },
    "test-secret",
    now,
  );
  const check = await verifyToken(token, {
    secret: "test-secret",
    now,
    grant: grantForCapability("cap.echo"),
    expectedSubject: "tester",
    expectedIntentId: compiled.ir.id,
  });
  assert.equal(check.ok, true);

  const output = await runGuest(
    { kind: "echo", text: compiled.ir.tasks[0].description },
    rightsFromToken(token.capability, token.permissions.fs),
    { onEcho: () => {}, onWrite: () => {} },
  );
  assert.equal(output, "ping");

  const folded = foldEvents([
    { kind: "IntentCompiled", payload: {} },
    {
      kind: "IntentSimulated",
      payload: {
        success_probability: sim.success_probability,
        risk: sim.risk,
        duration_ms: sim.duration_ms,
        notes: sim.notes,
      },
    },
    { kind: "IntentApproved", payload: {} },
    { kind: "TokenIssued", payload: { token_id: token.id, fingerprint: token.signature.slice(0, 12) } },
    { kind: "TaskSucceeded", payload: { output } },
  ]);
  assert.equal(folded.status, "executed");
  assert.equal(folded.output, "ping");
  assert.equal(snapshotsMatch(folded, { status: "executed", output: "ping" }), true);
});

test("write path escape never compiles", () => {
  const compiled = compileRules("Write file ../secret with contents x");
  assert.equal(compiled.ok, false);
});
