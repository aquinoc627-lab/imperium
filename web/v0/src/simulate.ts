import { KNOWN_CAPABILITIES, WRITE_CAPABILITY, READ_CAPABILITY, APPEND_CAPABILITY, LIST_CAPABILITY, HTTP_CAPABILITY, fetchUrlHost, FETCH_NO_ALLOW_RULE } from "./compiler.ts";
import type { Effect, EffectPreview, IntentIR, SimulationResult } from "./types.ts";
import { pathAllowed } from "./token.ts";
import { resolveScratchPath } from "./scratch.ts";
import { evaluate, type Policy, type PolicyDecision } from "./policy.ts";

/** Sensitive-path patterns. Matched per path segment, case-insensitive. */
export const SENSITIVE_PATTERNS = ["*.key", "*secret*", ".env", ".env.*"] as const;
export const SENSITIVE_DENIED_REASON = "sensitive path denied";

export function isSensitivePath(path: string): boolean {
  return path
    .replace(/\\/g, "/")
    .split("/")
    .some((seg) => {
      const l = seg.toLowerCase();
      return (
        l.endsWith(".key") || l.includes("secret") || l === ".env" || l.startsWith(".env.")
      );
    });
}

function hostOp(capability: string): string {
  switch (capability) {
    case WRITE_CAPABILITY:
      return "write";
    case READ_CAPABILITY:
      return "read";
    case APPEND_CAPABILITY:
      return "append";
    case LIST_CAPABILITY:
      return "list";
    case HTTP_CAPABILITY:
      return "fetch";
    default:
      return "effect";
  }
}

function grantPrefixes(capability: string): string[] {
  return capability === WRITE_CAPABILITY ||
    capability === READ_CAPABILITY ||
    capability === APPEND_CAPABILITY ||
    capability === LIST_CAPABILITY
    ? ["scratch"]
    : [];
}

function previewEffect(
  effect: Effect,
  capability: string,
  policy?: Policy,
): EffectPreview {
  if (effect.type === "echo") {
    // Echo is path-free: policy content rules may still deny it.
    if (policy) {
      const decision = evaluate(policy, "echo", "", effect.text);
      if (decision.kind === "deny") {
        return {
          kind: "denied",
          capability,
          path: "",
          reason: `policy: ${decision.reason}`,
        };
      }
    }
    return { kind: "echo", text: effect.text };
  }
  if (effect.type === "fetch") {
    // Fetch gate: URL checks → policy allowlist (default-deny).
    const deny = (reason: string): EffectPreview => ({
      kind: "denied",
      capability,
      path: effect.url,
      reason,
    });
    const host = fetchUrlHost(effect.url);
    if (!host.ok) return deny(host.error);
    if (!policy) return deny(FETCH_NO_ALLOW_RULE);
    const decision = evaluate(policy, "fetch", host.host, effect.url);
    if (decision.kind === "deny") {
      return deny(`policy: ${decision.reason}`);
    }
    if (decision.kind === "allow" && decision.rule !== null) {
      return { kind: "fetch", url: effect.url };
    }
    return deny(FETCH_NO_ALLOW_RULE);
  }
  const op = hostOp(capability);
  const deny = (path: string, reason: string): EffectPreview => ({
    kind: "denied",
    capability,
    path,
    reason,
  });
  const resolved = resolveScratchPath(effect.path);
  if (!resolved.ok) {
    return deny(effect.path, resolved.error);
  }
  if (!pathAllowed(resolved.path, grantPrefixes(capability))) {
    return deny(resolved.path, `host.${op} path denied`);
  }
  if (isSensitivePath(resolved.path)) {
    return deny(resolved.path, SENSITIVE_DENIED_REASON);
  }
  // User policy layer (can only deny more than the host firewall).
  if (policy) {
    const decision: PolicyDecision = evaluate(policy, op, resolved.path, resolved.path);
    if (decision.kind === "deny") {
      return deny(resolved.path, `policy: ${decision.reason}`);
    }
  }
  if (effect.type === "write") {
    return { kind: "write", path: resolved.path, bytes: effect.size_bytes ?? 0 };
  }
  if (effect.type === "append") {
    return { kind: "append", path: resolved.path, bytes: effect.size_bytes ?? 0 };
  }
  if (effect.type === "read") {
    return { kind: "read", path: resolved.path };
  }
  return { kind: "list", path: resolved.path };
}

export function simulateStatic(ir: IntentIR, policy?: Policy): SimulationResult {
  const caps = ir.tasks.flatMap((t) => t.capabilities);
  const allKnown =
    caps.length > 0 &&
    caps.every((c) => (KNOWN_CAPABILITIES as readonly string[]).includes(c));
  const duration_ms = ir.tasks.reduce(
    (sum, t) => sum + (t.estimated_duration_ms ?? 1000),
    0,
  );
  const notes = allKnown
    ? caps.map((c) => `Capability ${c} allowed.`)
    : ["Unknown or missing capability. Execution would be denied."];

  // Dry-run: fold each declared effect into an exact preview. No execution.
  const effects_preview: EffectPreview[] = [];
  let denied = false;
  for (const task of ir.tasks) {
    const effects = task.effects ?? [];
    if (effects.length === 0) {
      notes.push(`Task ${task.name} declares no effects; preview unavailable.`);
      continue;
    }
    for (const effect of effects) {
      const preview = previewEffect(effect, task.capabilities[0] ?? "", policy);
      if (preview.kind === "denied") denied = true;
      effects_preview.push(preview);
    }
  }
  if (denied) {
    notes.push("Dry-run denied one or more effects; approval is not possible.");
  }

  return {
    success_probability: denied || !allKnown ? 0 : 1,
    risk: denied || !allKnown ? 1 : 0,
    duration_ms,
    notes,
    effects_preview,
  };
}

export function knownWriteCapability(capability: string): boolean {
  return capability === WRITE_CAPABILITY;
}

// --- Phase 12: probabilistic dry-run (Monte Carlo) ---
// Mirrors crates/imperium-core/src/v0.rs: identical LCG, draw order, math.

export interface CapabilityStat {
  samples: number;
  successes: number;
  durations_ms: number[];
}

export type WorldStats = Record<string, CapabilityStat>;

export const MIN_DURATION_SAMPLES = 5;
export const APPROVAL_PROBABILITY_MILLI = 9000n;

export function mcGate(successes: number, trials: number): boolean {
  if (trials === 0) return false;
  return (
    BigInt(successes) * 10_000n >= APPROVAL_PROBABILITY_MILLI * BigInt(trials)
  );
}

/** FNV-1a 64 over the intent id (mirror of derive_seed). */
export function deriveSeed(id: string): bigint {
  let hash = 0xcbf29ce484222325n;
  for (const b of new TextEncoder().encode(id)) {
    hash ^= BigInt(b);
    hash = (hash * 0x100000001b3n) & 0xffffffffffffffffn;
  }
  return hash;
}

/** 64-bit LCG via BigInt (Rust u64 wrapping ops are exact here). */
export class MonteCarloRng {
  private s: bigint;
  constructor(seed: bigint) {
    this.s = seed & 0xffffffffffffffffn;
  }
  draw(): bigint {
    this.s =
      (this.s * 6364136223846793005n + 1442695040888963407n) &
      0xffffffffffffffffn;
    return this.s >> 33n;
  }
}

function laplaceMilli(stat: CapabilityStat): number {
  return Math.floor(((stat.successes + 1) * 10_000) / (stat.samples + 2));
}

function factorNote(cap: string, stat: CapabilityStat): string {
  const rate = (stat.successes + 1) / (stat.samples + 2);
  return stat.samples >= MIN_DURATION_SAMPLES
    ? `P(${cap}) = ${rate.toFixed(4)} (n=${stat.samples})`
    : `P(${cap}) = ${rate.toFixed(4)} (prior, n=${stat.samples})`;
}

const EMPTY_STAT: CapabilityStat = { samples: 0, successes: 0, durations_ms: [] };

export interface McOptions {
  stats: WorldStats;
  trials: number;
  seed: bigint;
}

export function dryRunMonteCarlo(
  ir: IntentIR,
  policy: Policy | undefined,
  mc: McOptions,
): SimulationResult {
  const base = simulateStatic(ir, policy);
  if (base.success_probability === 0) {
    // Hard denial: uncertainty never softens a denial. Mirror the Rust
    // serialized shape (all MC fields present, zeroed).
    return {
      ...base,
      probabilistic: false,
      trials: 0,
      seed: 0,
      p_success: 0,
      p50_ms: 0,
      p95_ms: 0,
      mc_successes: 0,
      factors: [],
    };
  }
  const rng = new MonteCarloRng(mc.seed);
  let successes = 0;
  const successDurations: number[] = [];
  const factors: string[] = [];
  for (const task of ir.tasks) {
    const cap = task.capabilities[0] ?? "";
    factors.push(factorNote(cap, mc.stats[cap] ?? EMPTY_STAT));
  }
  for (let t = 0; t < mc.trials; t++) {
    let trialOk = true;
    let totalMs = 0;
    for (const task of ir.tasks) {
      const cap = task.capabilities[0] ?? "";
      const stat = mc.stats[cap] ?? EMPTY_STAT;
      const pMilli = laplaceMilli(stat);
      // Mirror Rust's serde-default RetryPolicy (max_attempts 3, 1000ms ×2).
      const rp = task.retry_policy ?? {
        max_attempts: 3,
        backoff_ms: 1000,
        backoff_multiplier: 2.0,
      };
      const attempts = Math.max(1, rp.max_attempts);
      let done = false;
      for (let attempt = 0; attempt < attempts; attempt++) {
        const dur =
          stat.durations_ms.length >= MIN_DURATION_SAMPLES
            ? stat.durations_ms[Number(rng.draw() % BigInt(stat.durations_ms.length))]
            : (task.estimated_duration_ms ?? 1000);
        totalMs += dur;
        if (Number(rng.draw() % 10_000n) < pMilli) {
          done = true;
          break;
        }
        if (attempt + 1 < attempts) {
          const backoff = Math.floor(
            (rp.backoff_ms ?? 0) * Math.pow(rp.backoff_multiplier ?? 0, attempt),
          );
          totalMs += backoff;
        }
      }
      if (!done) {
        trialOk = false;
        break;
      }
    }
    if (trialOk) {
      successes++;
      successDurations.push(totalMs);
    }
  }
  let p50 = 0;
  let p95 = 0;
  if (successDurations.length > 0) {
    successDurations.sort((a, b) => a - b);
    const n = BigInt(successDurations.length);
    const i50 = Number((n * 50n) / 100n);
    const i95 = Number((n * 95n) / 100n);
    p50 = successDurations[Math.min(i50, successDurations.length - 1)];
    p95 = successDurations[Math.min(i95, successDurations.length - 1)];
  }
  return {
    ...base,
    probabilistic: true,
    trials: mc.trials,
    seed: Number(mc.seed & 0xffffffffffffffffn),
    p_success: successes / mc.trials,
    p50_ms: p50,
    p95_ms: p95,
    mc_successes: successes,
    factors,
  };
}
