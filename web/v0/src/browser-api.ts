/**
 * Public surface the browser workbench is allowed to call.
 * Do not add behavior here — re-export only.
 */
export {
  compileRules,
  COMPILE_USAGE_ERROR,
  COMPILER_VERSION,
  APPROVAL_THRESHOLD,
  riskForCapability,
  extractEchoText,
  extractWrite,
  ECHO_CAPABILITY,
  WRITE_CAPABILITY,
  READ_CAPABILITY,
  APPEND_CAPABILITY,
  LIST_CAPABILITY,
  HTTP_CAPABILITY,
  FETCH_NO_ALLOW_RULE,
} from "./compiler.ts";
export { localPropose } from "./propose.ts";
export { simulateStatic } from "./simulate.ts";
export {
  issueToken,
  verifyToken,
  fingerprint,
  grantForCapability,
} from "./token.ts";
export { foldEvents } from "./replay.ts";
export {
  parsePolicyLenient,
  lintPolicy,
  evaluate,
} from "./policy.ts";
export { runGuest, rightsFromToken } from "./wasm-host.ts";
export type { Policy } from "./policy.ts";
export type { IntentIR, SimulationResult, EffectPreview } from "./types.ts";
