import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { compileRules } from "./compiler.ts";
import { foldEvents } from "./replay.ts";
import { simulateStatic } from "./simulate.ts";
import { issueToken, signClaims } from "./token.ts";
import { localPropose } from "./propose.ts";
import {
  builtinContentDenied,
  lintPolicy,
  parsePolicyLenient,
  evaluate as evaluatePolicy,
} from "./policy.ts";
import { dryRunMonteCarlo } from "./simulate.ts";
import { buildTemplate, verifyShadow } from "./forms.ts";

const FIXTURE = new URL("../../../tests/contract/v0_kernel.json", import.meta.url);
const fx = JSON.parse(readFileSync(FIXTURE, "utf8"));

/** Every key in `expected` must match `actual`; extra actual keys are allowed. Numbers compare numerically. */
function subset(actual: unknown, expected: unknown): boolean {
  if (typeof actual === "number" && typeof expected === "number") {
    return Math.abs(actual - expected) < 1e-6;
  }
  if (
    actual !== null && typeof actual === "object" && !Array.isArray(actual) &&
    expected !== null && typeof expected === "object" && !Array.isArray(expected)
  ) {
    const a = actual as Record<string, unknown>;
    const e = expected as Record<string, unknown>;
    return Object.entries(e).every(([k, ev]) => k in a && subset(a[k], ev));
  }
  if (Array.isArray(actual) && Array.isArray(expected)) {
    return actual.length === expected.length &&
      actual.every((av, i) => subset(av, expected[i]));
  }
  return actual === expected;
}

function normalizeIr(ir: Record<string, unknown>) {
  const clone = structuredClone(ir) as Record<string, unknown>;
  clone.id = "<uuid>";
  clone.compiled_at = "<ts>";
  for (const t of clone.tasks as Array<Record<string, unknown>>) {
    t.id = "<task-uuid>";
  }
  return clone;
}

test("contract: compile cases match shared fixtures", () => {
  for (const c of fx.compile_cases) {
    const compiled = compileRules(c.input);
    assert.ok(compiled.ok, `case ${c.name}: compile failed`);
    if (!compiled.ok) continue;
    assert.ok(
      subset(normalizeIr(compiled.ir as unknown as Record<string, unknown>), c.expected_ir),
      `case ${c.name}: IR mismatch`,
    );
  }
});

test("contract: compile errors match shared fixtures", () => {
  for (const c of fx.compile_errors) {
    const compiled = compileRules(c.input);
    assert.ok(!compiled.ok, `case ${c.name}: should not compile`);
    if (compiled.ok) continue;
    assert.equal(compiled.error, c.error, `case ${c.name}: error mismatch`);
  }
});

test("contract: propose cases match shared fixtures", () => {
  for (const c of fx.propose_cases) {
    const p = localPropose(c.input);
    assert.ok(p.ok, `case ${c.name}: propose failed`);
    if (!p.ok) continue;
    assert.equal(p.canonical, c.canonical);
    assert.equal(p.proposer, c.proposer);
  }
  for (const c of fx.propose_errors) {
    const p = localPropose(c.input);
    assert.ok(!p.ok, `case ${c.name}: should not propose`);
    if (p.ok) continue;
    assert.equal(p.error, c.error);
  }
});

test("contract: token canonical signatures match shared fixtures", async () => {
  for (const c of fx.token_cases) {
    const direct = await signClaims(
      { ...c.claims, permissions: { ...c.claims.permissions } },
      c.secret,
    );
    assert.equal(direct, c.signature_hex, `case ${c.name}: direct sign mismatch`);
    const issued = await issueToken(
      { ...c.claims, permissions: { ...c.claims.permissions } },
      c.secret,
      c.claims.issued_at,
    );
    assert.equal(issued.signature, c.signature_hex, `case ${c.name}: issued mismatch`);
  }
});

test("contract: simulate cases match shared fixtures", () => {
  for (const c of fx.simulate_cases) {
    const policy = c.policy
      ? (() => {
          const { policy, issues } = parsePolicyLenient(c.policy);
          assert.equal(issues.length, 0, "fixture policy must parse");
          return policy;
        })()
      : undefined;
    const sim = simulateStatic(c.ir, policy);
    assert.ok(
      subset(sim as unknown as Record<string, unknown>, c.expected),
      `case ${c.name}: simulation mismatch ${JSON.stringify(sim)}`,
    );
  }
});

test("contract: fold cases match shared fixtures", () => {
  for (const c of fx.fold_cases) {
    const folded = foldEvents(c.events);
    assert.ok(
      subset(folded as unknown as Record<string, unknown>, c.expected),
      `case ${c.name}: fold mismatch ${JSON.stringify(folded)}`,
    );
  }
});

test("contract: policy cases match shared fixtures", () => {
  for (const c of fx.policy_cases) {
    const { policy, issues } = parsePolicyLenient(c.policy);
    assert.equal(issues.length, 0, `case ${c.name}: policy must parse`);
    const decision = evaluatePolicy(policy, c.verb, c.path, c.text);
    assert.ok(
      subset(decision as unknown as Record<string, unknown>, c.expected),
      `case ${c.name}: decision mismatch ${JSON.stringify(decision)}`,
    );
  }
});

test("contract: policy parse errors match shared fixtures", () => {
  for (const c of fx.policy_parse_errors) {
    const { issues } = parsePolicyLenient(c.policy);
    assert.ok(
      subset(issues as unknown as unknown[], c.issues),
      `case ${c.name}: parse issues mismatch ${JSON.stringify(issues)}`,
    );
  }
});

test("contract: policy lint cases match shared fixtures", () => {
  for (const c of fx.policy_lint_cases) {
    const issues = lintPolicy(c.policy);
    assert.ok(
      subset(issues as unknown as unknown[], c.issues),
      `case ${c.name}: lint issues mismatch ${JSON.stringify(issues)}`,
    );
  }
});

test("contract: builtin content cases match shared fixtures", () => {
  for (const c of fx.builtin_content_cases) {
    assert.equal(builtinContentDenied(c.text), c.expected, `case ${c.name}`);
  }
});

test("contract: monte carlo cases match shared fixtures", () => {
  for (const c of fx.monte_carlo_cases) {
    const sim = dryRunMonteCarlo(c.ir, undefined, {
      stats: c.stats,
      trials: c.trials,
      seed: BigInt(c.seed),
    });
    assert.ok(
      subset(sim as unknown as Record<string, unknown>, c.expected),
      `case ${c.name}: monte carlo mismatch ${JSON.stringify(sim)}`,
    );
  }
});

test("contract: form template cases match shared fixtures", () => {
  for (const c of fx.form_template_cases) {
    const result = buildTemplate(c.ir);
    const v =
      result.ok
        ? { template: result.template.template, slot: result.template.slot }
        : { error: result.error };
    assert.ok(
      subset(v, c.expected),
      `case ${c.name}: form template mismatch ${JSON.stringify(v)}`,
    );
  }
});

test("contract: shadow diff cases match shared fixtures", () => {
  for (const c of fx.shadow_diff_cases) {
    const diff = verifyShadow(c.predicted, c.actual);
    assert.ok(
      subset(diff as unknown as Record<string, unknown>, c.expected),
      `case ${c.name}: shadow diff mismatch ${JSON.stringify(diff)}`,
    );
  }
});
