import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  compileRules,
  localPropose,
  simulateStatic,
  foldEvents,
  parsePolicyLenient,
  lintPolicy,
  FETCH_NO_ALLOW_RULE,
} from "./browser-api.ts";

describe("workbench binds to web/v0 exports", () => {
  it("compiles the six canonical sentences the instrument lists", () => {
    assert.equal(compileRules("Echo this message: ping").ok, true);
    assert.equal(
      compileRules("Write file notes.txt with contents hello").ok,
      true,
    );
    assert.equal(compileRules("Read file notes.txt").ok, true);
    assert.equal(
      compileRules("Append file notes.txt with contents more").ok,
      true,
    );
    assert.equal(compileRules("List files under notes").ok, true);
    assert.equal(compileRules("Fetch https://api.example.com").ok, true);
  });

  it("proposes a loose echo onto a canonical sentence", () => {
    const p = localPropose("say hello");
    assert.equal(p.ok, true);
    if (p.ok) assert.match(p.canonical, /^Echo this message:/);
  });

  it("dry-run previews write and default-denies fetch without allow rule", () => {
    const write = compileRules("Write file notes.txt with contents hello");
    assert.equal(write.ok, true);
    if (!write.ok) return;
    const preview = simulateStatic(write.ir);
    assert.equal(preview.effects_preview?.[0]?.kind, "write");

    const fetch = compileRules("Fetch https://api.example.com");
    assert.equal(fetch.ok, true);
    if (!fetch.ok) return;
    const denied = simulateStatic(fetch.ir);
    assert.equal(denied.effects_preview?.[0]?.kind, "denied");
    assert.equal(denied.effects_preview?.[0]?.reason, FETCH_NO_ALLOW_RULE);
  });

  it("folds compiled → simulated from the same helper the UI uses", () => {
    const folded = foldEvents([
      { kind: "IntentCompiled", payload: {} },
      { kind: "IntentSimulated", payload: { notes: [] } },
    ]);
    assert.equal(folded.status, "simulated");
  });

  it("lints an empty policy as usable", () => {
    const { policy, issues } = parsePolicyLenient("");
    assert.equal(issues.length, 0);
    assert.equal(policy.rules.length, 0);
    assert.equal(lintPolicy("").length, 0);
  });
});
