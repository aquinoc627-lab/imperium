import assert from "node:assert/strict";
import test from "node:test";
import {
  issueToken,
  verifyToken,
  isPermissionSubset,
  pathAllowed,
  V0_GRANT,
  type CapabilityToken,
} from "./token.ts";

const SECRET = "test-secret-phase-2";
const NOW = 1_700_000_000_000;

async function echoToken(
  over: Partial<Parameters<typeof issueToken>[0]> = {},
): Promise<CapabilityToken> {
  return issueToken(
    {
      capability: "cap.echo",
      subject: "user-1",
      intent_id: "intent-1",
      permissions: { fs: [], net: [], env: [] },
      expires_at: NOW + 60_000,
      ...over,
    },
    SECRET,
    NOW,
  );
}

test("issue then verify succeeds", async () => {
  const token = await echoToken();
  const res = await verifyToken(token, { secret: SECRET, now: NOW });
  assert.equal(res.ok, true);
  assert.equal(res.reason, "ok");
});

test("empty signature fails closed", async () => {
  const token = await echoToken();
  token.signature = "";
  const res = await verifyToken(token, { secret: SECRET, now: NOW });
  assert.equal(res.ok, false);
  assert.equal(res.reason, "empty signature");
});

test("tampered payload fails verify", async () => {
  const token = await echoToken();
  token.capability = "cap.shell";
  const res = await verifyToken(token, { secret: SECRET, now: NOW });
  assert.equal(res.ok, false);
  assert.equal(res.reason, "invalid signature");
});

test("expired token fails", async () => {
  const token = await echoToken({ expires_at: NOW - 1 });
  const res = await verifyToken(token, { secret: SECRET, now: NOW });
  assert.equal(res.ok, false);
  assert.equal(res.reason, "expired");
});

test("nonce reuse fails", async () => {
  const token = await echoToken();
  const seen = new Set<string>();
  const first = await verifyToken(token, {
    secret: SECRET,
    now: NOW,
    seenNonces: seen,
  });
  assert.equal(first.ok, true);
  seen.add(token.nonce);
  const second = await verifyToken(token, {
    secret: SECRET,
    now: NOW,
    seenNonces: seen,
  });
  assert.equal(second.ok, false);
  assert.equal(second.reason, "nonce reused");
});

test("revoked token fails", async () => {
  const token = await echoToken();
  const res = await verifyToken(token, {
    secret: SECRET,
    now: NOW,
    revokedIds: new Set([token.id]),
  });
  assert.equal(res.ok, false);
  assert.equal(res.reason, "revoked");
});

test("unknown capability fails even with valid signature", async () => {
  const token = await issueToken(
    {
      capability: "cap.shell",
      subject: "user-1",
      intent_id: "intent-1",
      permissions: { fs: [], net: [], env: [] },
      expires_at: NOW + 60_000,
    },
    SECRET,
    NOW,
  );
  const res = await verifyToken(token, { secret: SECRET, now: NOW });
  assert.equal(res.ok, false);
  assert.equal(res.reason, "unknown capability");
});

test("network request is denied", async () => {
  const token = await issueToken(
    {
      capability: "cap.echo",
      subject: "user-1",
      intent_id: "intent-1",
      permissions: { fs: [], net: ["api.example.com"], env: [] },
      expires_at: NOW + 60_000,
    },
    SECRET,
    NOW,
  );
  const res = await verifyToken(token, { secret: SECRET, now: NOW });
  assert.equal(res.ok, false);
  assert.equal(res.reason, "network denied");
});

test("fs path outside grant is not a subset", () => {
  assert.equal(pathAllowed("/tmp/scratch/a.txt", ["/tmp/scratch"]), true);
  assert.equal(pathAllowed("/etc/passwd", ["/tmp/scratch"]), false);
  assert.equal(pathAllowed("/tmp/scratch", []), false);
  assert.equal(
    isPermissionSubset(
      { fs: ["/etc/passwd"], net: [], env: [] },
      { fs: ["/tmp/scratch"], net: [], env: [] },
    ),
    false,
  );
  assert.equal(
    isPermissionSubset(
      { fs: [], net: [], env: [] },
      V0_GRANT,
    ),
    true,
  );
});

test("permission not subset fails verify", async () => {
  const token = await issueToken(
    {
      capability: "cap.echo",
      subject: "user-1",
      intent_id: "intent-1",
      permissions: { fs: ["/workspace"], net: [], env: ["PATH"] },
      expires_at: NOW + 60_000,
    },
    SECRET,
    NOW,
  );
  const res = await verifyToken(token, {
    secret: SECRET,
    now: NOW,
    grant: V0_GRANT,
  });
  assert.equal(res.ok, false);
  assert.equal(res.reason, "permission not subset");
});
