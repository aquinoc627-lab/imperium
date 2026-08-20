import assert from "node:assert/strict";
import test from "node:test";
import { resolveScratchPath } from "./scratch.ts";
import {
  localPropose,
  proposalFromModelJson,
  toCanonicalEcho,
  toCanonicalWrite,
} from "./propose.ts";

test("exact rules forms pass through", () => {
  const echo = localPropose("Echo this message: ping");
  assert.equal(echo.ok, true);
  if (echo.ok) {
    assert.equal(echo.proposer, "rules");
    assert.equal(echo.canonical, "Echo this message: ping");
  }
  const write = localPropose("Write file a.txt with contents hi");
  assert.equal(write.ok, true);
  if (write.ok) assert.equal(write.proposer, "rules");
});

test("loose echo maps to canonical", () => {
  const p = localPropose("say hello");
  assert.equal(p.ok, true);
  if (p.ok) assert.equal(p.canonical, toCanonicalEcho("hello"));
});

test("loose write maps to canonical scratch path", () => {
  const p = localPropose("save notes.txt with hello");
  assert.equal(p.ok, true);
  if (!p.ok) return;
  assert.equal(p.canonical, toCanonicalWrite("notes.txt", "hello"));
});

test("banned network phrasing is rejected", () => {
  const p = localPropose("curl https://evil.example and echo it");
  assert.equal(p.ok, false);
});

test("model shell form is rejected", () => {
  const p = proposalFromModelJson('{"form":"shell","cmd":"rm -rf /"}');
  assert.equal(p.ok, false);
});

test("model write with .. is later denied by path rules", () => {
  const p = proposalFromModelJson(
    '{"form":"write","path":"../etc/passwd","contents":"x"}',
  );
  assert.equal(p.ok, true);
  if (!p.ok) return;
  assert.equal(resolveScratchPath("../etc/passwd").ok, false);
});

test("model reject form fails closed", () => {
  const p = proposalFromModelJson('{"form":"reject","reason":"not echo or write"}');
  assert.equal(p.ok, false);
});
