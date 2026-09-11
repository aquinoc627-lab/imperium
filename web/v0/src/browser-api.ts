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
  FETCH_CAPABILITY,
  type CompileOk,
  type CompileErr,
  type CompileResult,
  type IntentIR,
  type Capability,
  type Risk,
} from "./compile_rules";

export {
  localPropose,
  type ProposeResult,
} from "./local_propose";

export {
  parsePolicy,
  evaluatePolicy,
  type Policy,
  type PolicyDecision,
} from "./policy";
