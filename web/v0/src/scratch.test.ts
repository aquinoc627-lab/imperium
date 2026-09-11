import assert from "node:assert/strict";
import test from "node:test";
import { resolveScratchPath } from "./scratch.ts";
import { grantForCapability, isPermissionSubset, WRITE_GRANT } from "./token.ts";

test("hello.txt lands under scratch/", () => {
  const r = resolveScratchPath("hello.txt");
  assert.equal(r.ok, true);
  if (r.ok) assert.equal(r.path, "scratch/hello.txt");
});

test("scratch/foo.txt is unchanged prefix", () => {
  const r = resolveScratchPath("scratch/foo.txt");
  assert.equal(r.ok, true);
  if (r.ok) assert.equal(r.path, "scratch/foo.txt");
});

test("../ escape is denied", () => {
  const r = resolveScratchPath("../etc/passwd");
  assert.equal(r.ok, false);
});

test("scratch/../etc/passwd is denied", () => {
  const r = resolveScratchPath("scratch/../etc/passwd");
  assert.equal(r.ok, false);
});

test("absolute path is denied", () => {
  const r = resolveScratchPath("/etc/passwd");
  assert.equal(r.ok, false);
});

test("write grant is a subset of scratch prefix and has no net", () => {
  const g = grantForCapability("cap.write");
  assert.deepEqual(g, WRITE_GRANT);
  assert.equal(isPermissionSubset({ fs: ["scratch/a"], net: [], env: [] }, g), true);
  assert.equal(isPermissionSubset({ fs: ["/etc"], net: [], env: [] }, g), false);
  assert.equal(isPermissionSubset({ fs: ["scratch"], net: ["x.com"], env: [] }, g), false);
});
