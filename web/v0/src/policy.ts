/**
 * The `.imp` policy language — TS reference mirror of
 * `crates/imperium-core/src/policy/imp.rs`. Both are validated against the
 * shared fixtures in `tests/contract/v0_kernel.json`.
 */

export const POLICY_VERBS = ["read", "write", "append", "list", "fetch"] as const;

/** Built-in proposal-time content ceiling (all verbs), substring + case-insensitive. */
export const BUILTIN_CONTENT_DENIES = [
  "rm ", "sudo", "curl", "wget", "http:", "https:", "ftp:", "shell", "bash",
  "powershell", "eval", "network", "download", "install", "chmod", "chown",
  "drop table", "delete from",
] as const;
export const BUILTIN_CONTENT_DENIED_MESSAGE =
  "Proposal rejected: banned verb or network reference.";

export type PolicyRule =
  | { kind: "path_deny"; verb: string; pattern: string }
  | { kind: "content_deny"; substring: string }
  | { kind: "require_approval"; verb: string }
  | { kind: "allow"; verb: string; prefix: string };

export interface Policy {
  rules: PolicyRule[];
}

export type PolicyDecision =
  | { kind: "allow"; rule: number | null }
  | { kind: "require_approval"; rule: number }
  | { kind: "deny"; rule: number; reason: string };

export interface ParseIssue {
  line: number;
  message: string;
}

export function canonicalRule(rule: PolicyRule): string {
  switch (rule.kind) {
    case "path_deny":
      return `deny ${rule.verb} matching ${rule.pattern}`;
    case "content_deny":
      return `deny containing ${rule.substring}`;
    case "require_approval":
      return `require approval ${rule.verb}`;
    case "allow":
      if (rule.verb === "fetch") {
        return `allow ${rule.verb} to ${rule.prefix}`;
      }
      return `allow ${rule.verb} under ${rule.prefix}`;
  }
}

/** Wildcard glob over the full path; `*` matches any sequence. Case-insensitive. */
export function globMatch(pattern: string, text: string): boolean {
  const p = pattern.toLowerCase();
  const t = text.toLowerCase();
  let pi = 0;
  let ti = 0;
  let star = -1;
  let mark = 0;
  while (ti < t.length) {
    if (pi < p.length && (p[pi] === "*" || p[pi] === t[ti])) {
      if (p[pi] === "*") {
        star = pi;
        mark = ti;
        pi++;
      } else {
        pi++;
        ti++;
      }
    } else if (star !== -1) {
      pi = star + 1;
      mark++;
      ti = mark;
    } else {
      return false;
    }
  }
  while (pi < p.length && p[pi] === "*") pi++;
  return pi === p.length;
}

/** Prefix-boundary check (same semantics as token.ts pathAllowed). */
function pathAllowed(path: string, prefixes: string[]): boolean {
  if (prefixes.length === 0) return false;
  return prefixes.some((prefix) => {
    const p = prefix.replace(/\\/g, "/");
    return path === p || path.startsWith(p.endsWith("/") ? p : `${p}/`);
  });
}

function isPolicyVerb(verb: string): boolean {
  return (POLICY_VERBS as readonly string[]).includes(verb);
}

export function parseRules(text: string): {
  rules: Array<{ line: number; rule: PolicyRule }>;
  issues: ParseIssue[];
} {
  const rules: Array<{ line: number; rule: PolicyRule }> = [];
  const issues: ParseIssue[] = [];
  text.split("\n").forEach((raw, idx) => {
    const line = idx + 1;
    const trimmed = raw.trim();
    if (!trimmed || trimmed.startsWith("#")) return;
    const tokens = trimmed.split(/\s+/);
    if (tokens.length === 4 && tokens[0] === "deny" && tokens[2] === "matching") {
      if (!isPolicyVerb(tokens[1])) {
        issues.push({ line, message: `unknown verb: ${tokens[1]}` });
        return;
      }
      rules.push({
        line,
        rule: { kind: "path_deny", verb: tokens[1], pattern: tokens[3] },
      });
      return;
    }
    if (tokens.length === 3 && tokens[0] === "require" && tokens[1] === "approval") {
      if (!isPolicyVerb(tokens[2])) {
        issues.push({ line, message: `unknown verb: ${tokens[2]}` });
        return;
      }
      rules.push({ line, rule: { kind: "require_approval", verb: tokens[2] } });
      return;
    }
    if (tokens.length === 4 && tokens[0] === "allow" && tokens[1] === "fetch" && tokens[2] === "to") {
      rules.push({
        line,
        rule: { kind: "allow", verb: "fetch", prefix: tokens[3] },
      });
      return;
    }
    if (tokens.length === 4 && tokens[0] === "allow" && tokens[2] === "under") {
      if (!isPolicyVerb(tokens[1])) {
        issues.push({ line, message: `unknown verb: ${tokens[1]}` });
        return;
      }
      rules.push({
        line,
        rule: { kind: "allow", verb: tokens[1], prefix: tokens[3] },
      });
      return;
    }
    if (trimmed.startsWith("deny containing ")) {
      const substring = trimmed.slice("deny containing ".length).trim();
      rules.push({ line, rule: { kind: "content_deny", substring } });
      return;
    }
    issues.push({ line, message: "unrecognized rule" });
  });
  return { rules, issues };
}

export function parsePolicyLenient(text: string): { policy: Policy; issues: ParseIssue[] } {
  const { rules, issues } = parseRules(text);
  return { policy: { rules: rules.map((r) => r.rule) }, issues };
}

export function evaluate(
  policy: Policy,
  verb: string,
  path: string,
  text: string,
): PolicyDecision {
  const lowerText = text.toLowerCase();
  for (let i = 0; i < policy.rules.length; i++) {
    const rule = policy.rules[i];
    if (
      rule.kind === "path_deny" &&
      rule.verb === verb &&
      globMatch(rule.pattern, path)
    ) {
      return { kind: "deny", rule: i, reason: canonicalRule(rule) };
    }
    if (
      rule.kind === "content_deny" &&
      lowerText.includes(rule.substring.toLowerCase())
    ) {
      return { kind: "deny", rule: i, reason: canonicalRule(rule) };
    }
    if (rule.kind === "require_approval" && rule.verb === verb) {
      return { kind: "require_approval", rule: i };
    }
    if (
      rule.kind === "allow" &&
      rule.verb === verb &&
      (verb === "fetch"
        ? globMatch(rule.prefix, path)
        : pathAllowed(path, [rule.prefix]))
    ) {
      return { kind: "allow", rule: i };
    }
  }
  return { kind: "allow", rule: null };
}

export function requiresApproval(policy: Policy, verb: string): boolean {
  return policy.rules.some(
    (r) => r.kind === "require_approval" && r.verb === verb,
  );
}

export function contentDenied(policy: Policy, text: string): { rule: number; reason: string } | null {
  const lower = text.toLowerCase();
  for (let i = 0; i < policy.rules.length; i++) {
    const rule = policy.rules[i];
    if (rule.kind === "content_deny" && lower.includes(rule.substring.toLowerCase())) {
      return { rule: i, reason: canonicalRule(rule) };
    }
  }
  return null;
}

export function builtinContentDenied(text: string): boolean {
  const lower = text.toLowerCase();
  return BUILTIN_CONTENT_DENIES.some((b) => lower.includes(b));
}

export type Severity = "error" | "warning";

export interface LintIssue {
  line: number;
  severity: Severity;
  message: string;
}

const HOST_FIREWALL_PATTERNS = ["*.key", "*secret*", ".env", ".env.*"];

/** Does deny-glob `pattern` cover every path under `prefix`? */
function covers(pattern: string, prefix: string): boolean {
  return (
    pattern === "*" ||
    pattern === prefix ||
    (pattern.endsWith("/*") && prefix.startsWith(pattern.slice(0, -1)))
  );
}

export function lintPolicy(text: string): LintIssue[] {
  const { rules, issues } = parseRules(text);
  const out: LintIssue[] = issues.map((i) => ({
    line: i.line,
    severity: "error",
    message: i.message,
  }));
  for (let j = 0; j < rules.length; j++) {
    const { line: lineJ, rule: ruleJ } = rules[j];
    const earlier = rules.slice(0, j);
    const dup = earlier.find(
      (e) => JSON.stringify(e.rule) === JSON.stringify(ruleJ),
    );
    if (dup) {
      out.push({
        line: lineJ,
        severity: "warning",
        message: `duplicate of rule on line ${dup.line}`,
      });
      continue;
    }
    if (ruleJ.kind === "allow") {
      const shadow = earlier.find(
        (e) =>
          e.rule.kind === "path_deny" &&
          e.rule.verb === ruleJ.verb &&
          covers(e.rule.pattern, ruleJ.prefix),
      );
      if (shadow) {
        out.push({
          line: lineJ,
          severity: "warning",
          message: `unreachable: deny on line ${shadow.line} covers this allow`,
        });
      }
    }
    if (
      ruleJ.kind === "path_deny" &&
      (HOST_FIREWALL_PATTERNS as readonly string[]).includes(ruleJ.pattern)
    ) {
      out.push({
        line: lineJ,
        severity: "warning",
        message: "already enforced by the built-in host firewall",
      });
    }
  }
  return out;
}
