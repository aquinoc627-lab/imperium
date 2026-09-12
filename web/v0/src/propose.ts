import { BUILTIN_CONTENT_DENIES, BUILTIN_CONTENT_DENIED_MESSAGE } from "./policy.ts";
import { MAX_NL_SOURCE_LEN, NL_TOO_LONG_ERROR } from "./compiler.ts";

export type Proposal =
  | { ok: true; canonical: string; proposer: "rules" | "local" | "model" }
  | { ok: false; error: string; proposer: "none" | "local" | "model" };

const EXACT_ECHO = /^echo this message:\s*(.+)$/is;
const EXACT_WRITE = /^write file\s+(.+?)\s+with contents\s+([\s\S]+)$/is;
const EXACT_READ = /^read file\s+(.+)$/is;
const EXACT_APPEND = /^append file\s+(.+?)\s+with contents\s+([\s\S]+)$/is;
const EXACT_LIST = /^list files under\s+(.+)$/is;

/** Built-in content ceiling: substring + case-insensitive (Phase 9 policy). */
function contentDenied(text: string): boolean {
  const lower = text.toLowerCase();
  return BUILTIN_CONTENT_DENIES.some((b) => lower.includes(b));
}

const ECHO_LOOSE =
  /^(?:please\s+)?(?:echo|say|print|repeat|tell me)\s*(?:this\s+message\s*)?[:\-]?\s+["']?(.+?)["']?$/is;

const WRITE_LOOSE =
  /^(?:please\s+)?(?:write|save|create|put)\s+(?:a\s+)?(?:file\s+)?(.+?)\s+(?:with(?:\s+contents)?|containing|as)\s+["']?([\s\S]+?)["']?$/is;

const READ_LOOSE =
  /^(?:please\s+)?(?:read|show|open|cat)\s+(?:a\s+)?(?:file\s+)?["']?(.+?)["']?$/is;

const APPEND_LOOSE =
  /^(?:please\s+)?append(?:\s+to)?\s+(?:a\s+)?(?:file\s+)?["']?(.+?)["']?\s+(?:with(?:\s+contents)?|containing|as)\s+["']?([\s\S]+?)["']?$/is;

const LIST_LOOSE =
  /^(?:please\s+)?(?:list|ls)\s+(?:files\s+under\s+|dir\s+)?(?:file\s+)?["']?(.+?)["']?$/is;

export function toCanonicalEcho(text: string): string {
  return `Echo this message: ${text.trim()}`;
}

export function toCanonicalWrite(path: string, contents: string): string {
  return `Write file ${path.trim()} with contents ${contents}`;
}

export function toCanonicalRead(path: string): string {
  return `Read file ${path.trim()}`;
}

export function toCanonicalAppend(path: string, contents: string): string {
  return `Append file ${path.trim()} with contents ${contents}`;
}

export function toCanonicalList(path: string): string {
  return `List files under ${path.trim()}`;
}

export function localPropose(nl: string): Proposal {
  if (nl.length > MAX_NL_SOURCE_LEN) {
    return { ok: false, error: NL_TOO_LONG_ERROR, proposer: "none" };
  }
  const source = nl.trim();
  if (!source) return { ok: false, error: "Natural language source is empty.", proposer: "none" };
  if (contentDenied(source)) {
    return { ok: false, error: BUILTIN_CONTENT_DENIED_MESSAGE, proposer: "local" };
  }
  if (
    EXACT_ECHO.test(source) ||
    EXACT_WRITE.test(source) ||
    EXACT_READ.test(source) ||
    EXACT_APPEND.test(source) ||
    EXACT_LIST.test(source)
  ) {
    return { ok: true, canonical: source, proposer: "rules" };
  }
  const echo = source.match(ECHO_LOOSE);
  if (echo?.[1]?.trim()) {
    return { ok: true, canonical: toCanonicalEcho(echo[1]), proposer: "local" };
  }
  const write = source.match(WRITE_LOOSE);
  if (write?.[1]?.trim() && write[2] !== undefined && write[2].length > 0) {
    return { ok: true, canonical: toCanonicalWrite(write[1], write[2]), proposer: "local" };
  }
  const read = source.match(READ_LOOSE);
  if (read?.[1]?.trim()) {
    return { ok: true, canonical: toCanonicalRead(read[1]), proposer: "local" };
  }
  const append = source.match(APPEND_LOOSE);
  if (append?.[1]?.trim() && append[2] !== undefined && append[2].length > 0) {
    return { ok: true, canonical: toCanonicalAppend(append[1], append[2]), proposer: "local" };
  }
  const list = source.match(LIST_LOOSE);
  if (list?.[1]?.trim()) {
    return { ok: true, canonical: toCanonicalList(list[1]), proposer: "local" };
  }
  return {
    ok: false,
    error: "Local proposer could not map this to echo or write.",
    proposer: "local",
  };
}

export function proposalFromModelJson(raw: string): Proposal {
  let parsed: {
    form?: string;
    text?: string;
    path?: string;
    contents?: string;
    reason?: string;
  };
  try {
    const start = raw.indexOf("{");
    const end = raw.lastIndexOf("}");
    parsed = JSON.parse(start >= 0 ? raw.slice(start, end + 1) : raw) as typeof parsed;
  } catch {
    return { ok: false, error: "Model proposal was not valid JSON.", proposer: "model" };
  }
  if (parsed.form === "reject") {
    return {
      ok: false,
      error: parsed.reason?.trim() || "Model rejected the intent.",
      proposer: "model",
    };
  }
  if (parsed.form === "echo" && parsed.text?.trim()) {
    if (contentDenied(parsed.text)) {
      return { ok: false, error: "Model echo text failed policy.", proposer: "model" };
    }
    return { ok: true, canonical: toCanonicalEcho(parsed.text), proposer: "model" };
  }
  if (parsed.form === "write" && parsed.path?.trim() && parsed.contents != null) {
    const blob = `${parsed.path} ${parsed.contents}`;
    if (contentDenied(blob)) {
      return { ok: false, error: "Model write failed policy.", proposer: "model" };
    }
    return {
      ok: true,
      canonical: toCanonicalWrite(parsed.path, String(parsed.contents)),
      proposer: "model",
    };
  }
  if (parsed.form === "read" && parsed.path?.trim()) {
    if (contentDenied(parsed.path)) {
      return { ok: false, error: "Model read failed policy.", proposer: "model" };
    }
    return { ok: true, canonical: toCanonicalRead(parsed.path), proposer: "model" };
  }
  if (parsed.form === "append" && parsed.path?.trim() && parsed.contents != null) {
    const blob = `${parsed.path} ${parsed.contents}`;
    if (contentDenied(blob)) {
      return { ok: false, error: "Model append failed policy.", proposer: "model" };
    }
    return {
      ok: true,
      canonical: toCanonicalAppend(parsed.path, String(parsed.contents)),
      proposer: "model",
    };
  }
  if (parsed.form === "list" && parsed.path?.trim()) {
    if (contentDenied(parsed.path)) {
      return { ok: false, error: "Model list failed policy.", proposer: "model" };
    }
    return { ok: true, canonical: toCanonicalList(parsed.path), proposer: "model" };
  }
  return { ok: false, error: "Model proposed an unsupported form.", proposer: "model" };
}
