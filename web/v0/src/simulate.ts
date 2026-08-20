import { KNOWN_CAPABILITIES } from "./compiler.ts";
import type { IntentIR, SimulationResult } from "./types.ts";

export function simulateStatic(ir: IntentIR): SimulationResult {
  const caps = ir.tasks.flatMap((t) => t.capabilities);
  const allKnown =
    caps.length > 0 &&
    caps.every((c) => (KNOWN_CAPABILITIES as readonly string[]).includes(c));
  const duration_ms = ir.tasks.reduce(
    (sum, t) => sum + (t.estimated_duration_ms ?? 1000),
    0,
  );
  return {
    success_probability: allKnown ? 1 : 0,
    risk: allKnown ? 0 : 1,
    duration_ms,
    notes: allKnown
      ? caps.map((c) => `Capability ${c} allowed.`)
      : ["Unknown or missing capability. Execution would be denied."],
  };
}
