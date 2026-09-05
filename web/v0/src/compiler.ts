import { resolveScratchPath } from "./scratch.ts";
import type { Effect, IntentIR, Task } from "./types.ts";

const ECHO = /^echo this message:\s*(.+)$/is;
const WRITE = /^write file\s+(.+?)\s+with contents\s+([\s\S]+)$/is;
const READ = /^read file\s+(.+)$/is;
const APPEND = /^append file\s+(.+?)\s+with contents\s+([\s\S]+)$/is;
const LIST = /^list files under\s+(.+)$/is;
const FETCH = /^fetch\s+(\S+)$/is;

export const ECHO_CAPABILITY = "cap.echo";
export const WRITE_CAPABILITY = "cap.write";
export const READ_CAPABILITY = "cap.read";
export const APPEND_CAPABILITY = "cap.append";
export const LIST_CAPABILITY = "cap.list";
export const HTTP_CAPABILITY = "cap.http";
/** @deprecated use ECHO_CAPABILITY */
export const ALLOWED_CAPABILITY = ECHO_CAPABILITY;
export const KNOWN_CAPABILITIES = [
  ECHO_CAPABILITY,
  WRITE_CAPABILITY,
  READ_CAPABILITY,
  APPEND_CAPABILITY,
  LIST_CAPABILITY,
  HTTP_CAPABILITY,
] as const;
export const COMPILER_VERSION = "imperium-intent-rules-0.4.0";
export const IR_VERSION = 2 as const;
export const APPROVAL_THRESHOLD = 0.4;
export const COMPILE_USAGE_ERROR =
  "v0 rules compiler only accepts: Echo this message: <text>  OR  Write file <path> with contents <text>  OR  Read file <path>  OR  Append file <path> with contents <text>  OR  List files under <path>  OR  Fetch <https-url>";

/** Risk weight per verb (Phase 8): drives requires_approval. Unknown = 1. */
export function riskForCapability(capability: string): number {
  switch (capability) {
    case ECHO_CAPABILITY:
      return 0.0;
    case LIST_CAPABILITY:
      return 0.1;
    case READ_CAPABILITY:
      return 0.2;
    case APPEND_CAPABILITY:
      return 0.4;
    case WRITE_CAPABILITY:
      return 0.5;
    case HTTP_CAPABILITY:
      return 0.6;
    default:
      return 1.0;
  }
}

// --- Fetch URL validation (mirror of v0.rs fetch_url_host) ---
export const FETCH_HTTPS_REQUIRED = "https required";
export const FETCH_PORT_DENIED = "port not allowed";
export const FETCH_IP_DENIED = "ip addresses denied";
export const FETCH_LOCAL_DENIED = "local address denied";
export const FETCH_URL_INVALID = "invalid url";
export const FETCH_NO_ALLOW_RULE = "no allow fetch rule matched";

export function fetchUrlHost(url: string): { ok: true; host: string } | { ok: false; error: string } {
  const trimmed = url.trim();
  const lower = trimmed.toLowerCase();
  if (!lower || /\s/.test(trimmed)) return { ok: false, error: FETCH_URL_INVALID };
  if (!lower.startsWith("https://")) return { ok: false, error: FETCH_HTTPS_REQUIRED };
  const rest = lower.slice("https://".length);
  const authority = rest.split(/[/?#]/, 1)[0];
  if (!authority) return { ok: false, error: FETCH_URL_INVALID };
  if (authority.includes("@")) return { ok: false, error: FETCH_URL_INVALID };
  if (authority.includes(":")) return { ok: false, error: FETCH_PORT_DENIED };
  const host = authority.replace(/\.+$/, "");
  if (!host) return { ok: false, error: FETCH_URL_INVALID };
  const parts = host.split(".");
  const looksIpv4 =
    parts.length === 4 && parts.every((seg) => /^\d+$/.test(seg));
  if (looksIpv4) return { ok: false, error: FETCH_IP_DENIED };
  if (
    host === "localhost" ||
    host.endsWith(".localhost") ||
    host.endsWith(".local") ||
    host.endsWith(".internal")
  ) {
    return { ok: false, error: FETCH_LOCAL_DENIED };
  }
  if (!host.includes(".")) return { ok: false, error: FETCH_URL_INVALID };
  return { ok: true, host };
}

function baseIr(
  name: string,
  source: string,
  goal: string,
  task: Task,
): IntentIR {
  const cap = task.capabilities[0] ?? "";
  const risk = riskForCapability(cap);
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
    requires_approval: risk >= APPROVAL_THRESHOLD,
    version: IR_VERSION,
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
        effects: [{ type: "echo", text: message }] satisfies Effect[],
      }),
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
          effects: [
            { type: "write", path: resolved.path, size_bytes: contents.length },
          ] satisfies Effect[],
        },
      ),
    };
  }

  const read = source.match(READ);
  if (read) {
    const resolved = resolveScratchPath(read[1]);
    if (!resolved.ok) return resolved;
    return {
      ok: true,
      ir: baseIr(
        `Read ${resolved.path}`,
        source,
        `Read ${resolved.path}`,
        {
          id: crypto.randomUUID(),
          name: "Read",
          description: resolved.path,
          kind: "Custom",
          capabilities: [READ_CAPABILITY],
          dependencies: [],
          estimated_duration_ms: 15,
          target_path: resolved.path,
          effects: [{ type: "read", path: resolved.path }] satisfies Effect[],
        },
      ),
    };
  }

  const append = source.match(APPEND);
  if (append) {
    const resolved = resolveScratchPath(append[1]);
    if (!resolved.ok) return resolved;
    const contents = append[2];
    if (contents.length === 0) {
      return { ok: false, error: "Append contents are empty." };
    }
    return {
      ok: true,
      ir: baseIr(
        `Append ${resolved.path}`,
        source,
        `Append ${resolved.path}`,
        {
          id: crypto.randomUUID(),
          name: "Append",
          description: contents,
          kind: "Custom",
          capabilities: [APPEND_CAPABILITY],
          dependencies: [],
          estimated_duration_ms: 20,
          target_path: resolved.path,
          effects: [
            { type: "append", path: resolved.path, size_bytes: contents.length },
          ] satisfies Effect[],
        },
      ),
    };
  }

  const list = source.match(LIST);
  if (list) {
    const resolved = resolveScratchPath(list[1]);
    if (!resolved.ok) return resolved;
    return {
      ok: true,
      ir: baseIr(
        `List ${resolved.path}`,
        source,
        `List ${resolved.path}`,
        {
          id: crypto.randomUUID(),
          name: "List",
          description: resolved.path,
          kind: "Custom",
          capabilities: [LIST_CAPABILITY],
          dependencies: [],
          estimated_duration_ms: 15,
          target_path: resolved.path,
          effects: [{ type: "list", path: resolved.path }] satisfies Effect[],
        },
      ),
    };
  }

  const fetched = source.match(FETCH);
  if (fetched) {
    const url = fetched[1];
    const host = fetchUrlHost(url);
    if (!host.ok) return { ok: false, error: host.error };
    return {
      ok: true,
      ir: baseIr(
        `Fetch ${url}`,
        source,
        `Fetch ${url}`,
        {
          id: crypto.randomUUID(),
          name: "Fetch",
          description: url,
          kind: "Custom",
          capabilities: [HTTP_CAPABILITY],
          dependencies: [],
          estimated_duration_ms: 500,
          target_path: url,
          effects: [{ type: "fetch", url }] satisfies Effect[],
        },
      ),
    };
  }

  return { ok: false, error: COMPILE_USAGE_ERROR };
}

export function extractEchoText(ir: IntentIR): string {
  return ir.tasks[0]?.description ?? "";
}

export function extractWrite(ir: IntentIR): { path: string; contents: string } | null {
  const task = ir.tasks[0];
  if (!task?.target_path) return null;
  return { path: task.target_path, contents: task.description };
}
