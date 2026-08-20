import { resolveScratchPath } from "./scratch.ts";
import type { IntentIR, Task } from "./types.ts";

const ECHO = /^echo this message:\s*(.+)$/is;
const WRITE = /^write file\s+(.+?)\s+with contents\s+([\s\S]+)$/is;

export const ECHO_CAPABILITY = "cap.echo";
export const WRITE_CAPABILITY = "cap.write";
/** @deprecated use ECHO_CAPABILITY */
export const ALLOWED_CAPABILITY = ECHO_CAPABILITY;
export const KNOWN_CAPABILITIES = [ECHO_CAPABILITY, WRITE_CAPABILITY] as const;
export const COMPILER_VERSION = "imperium-intent-rules-0.1.0";

function baseIr(
  name: string,
  source: string,
  goal: string,
  task: Task,
  risk: number,
): IntentIR {
  return {
    id: crypto.randomUUID(),
    name,
    nl_source: source,
    goal: {
      description: goal,
      category: "Automation",
      priority: "Normal",
    },
    constraints: [],
    success_criteria: [
      {
        id: "sc1",
        metric: "TestPassRate",
        threshold: { operator: "GreaterThanOrEqual", value: 1, unit: "ratio" },
        weight: 1,
      },
    ],
    tasks: [task],
    risk_score: risk,
    requires_approval: true,
    version: 1,
    compiled_at: new Date().toISOString(),
    compiler_version: COMPILER_VERSION,
  };
}

export function compileRules(nl: string):
  | { ok: true; ir: IntentIR }
  | { ok: false; error: string } {
  const source = nl.trim();
  if (!source) {
    return { ok: false, error: "Natural language source is empty." };
  }

  const echo = source.match(ECHO);
  if (echo) {
    const message = echo[1].trim();
    if (!message) return { ok: false, error: "Echo message is empty." };
    return {
      ok: true,
      ir: baseIr(`Echo ${message.slice(0, 40)}`, source, `Echo the text ${message}`, {
        id: crypto.randomUUID(),
        name: "Echo",
        description: message,
        kind: "Custom",
        capabilities: [ECHO_CAPABILITY],
        dependencies: [],
        estimated_duration_ms: 10,
        target_path: null,
      }, 0),
    };
  }

  const write = source.match(WRITE);
  if (write) {
    const resolved = resolveScratchPath(write[1]);
    if (!resolved.ok) return resolved;
    const contents = write[2];
    if (contents.length === 0) {
      return { ok: false, error: "Write contents are empty." };
    }
    return {
      ok: true,
      ir: baseIr(
        `Write ${resolved.path}`,
        source,
        `Write ${resolved.path}`,
        {
          id: crypto.randomUUID(),
          name: "Write",
          description: contents,
          kind: "Custom",
          capabilities: [WRITE_CAPABILITY],
          dependencies: [],
          estimated_duration_ms: 20,
          target_path: resolved.path,
        },
        0,
      ),
    };
  }

  return {
    ok: false,
    error:
      "v0 rules compiler only accepts: Echo this message: <text>  OR  Write file <path> with contents <text>",
  };
}

export function extractEchoText(ir: IntentIR): string {
  return ir.tasks[0]?.description ?? "";
}

export function extractWrite(ir: IntentIR): { path: string; contents: string } | null {
  const task = ir.tasks[0];
  if (!task?.target_path) return null;
  return { path: task.target_path, contents: task.description };
}
