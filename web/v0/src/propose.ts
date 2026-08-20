export type Proposal =
  | { ok: true; canonical: string; proposer: "rules" | "local" | "model" }
  | { ok: false; error: string; proposer: "none" | "local" | "model" };

const EXACT_ECHO = /^echo this message:\s*(.+)$/is;
const EXACT_WRITE = /^write file\s+(.+?)\s+with contents\s+([\s\S]+)$/is;

const BANNED =
  /\b(rm\s|sudo|curl|wget|http:|https:|ftp:|shell|bash|powershell|eval|network|download|install|chmod|chown|drop table|delete from)\b/i;

const ECHO_LOOSE =
  /^(?:please\s+)?(?:echo|say|print|repeat|tell me)\s*(?:this\s+message\s*)?[:\-]?\s+["']?(.+?)["']?$/is;

const WRITE_LOOSE =
  /^(?:please\s+)?(?:write|save|create|put)\s+(?:a\s+)?(?:file\s+)?(.+?)\s+(?:with(?:\s+contents)?|containing|as)\s+["']?([\s\S]+?)["']?$/is;

export function toCanonicalEcho(text: string): string {
  return `Echo this message: ${text.trim()}`;
}

export function toCanonicalWrite(path: string, contents: string): string {
  return `Write file ${path.trim()} with contents ${contents}`;
}

export function localPropose(nl: string): Proposal {
  const source = nl.trim();
  if (!source) return { ok: false, error: "Natural language source is empty.", proposer: "none" };
  if (BANNED.test(source)) {
    return { ok: false, error: "Proposal rejected: banned verb or network reference.", proposer: "local" };
  }
  if (EXACT_ECHO.test(source) || EXACT_WRITE.test(source)) {
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
    if (BANNED.test(parsed.text)) {
      return { ok: false, error: "Model echo text failed policy.", proposer: "model" };
    }
    return { ok: true, canonical: toCanonicalEcho(parsed.text), proposer: "model" };
  }
  if (parsed.form === "write" && parsed.path?.trim() && parsed.contents != null) {
    const blob = `${parsed.path} ${parsed.contents}`;
    if (BANNED.test(blob)) {
      return { ok: false, error: "Model write failed policy.", proposer: "model" };
    }
    return {
      ok: true,
      canonical: toCanonicalWrite(parsed.path, String(parsed.contents)),
      proposer: "model",
    };
  }
  return { ok: false, error: "Model proposed an unsupported form.", proposer: "model" };
}
