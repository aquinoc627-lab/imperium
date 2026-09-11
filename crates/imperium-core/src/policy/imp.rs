//! The `.imp` policy language — Phase 9 "Semantic Firewall".
//!
//! A small, auditable, purpose-built policy engine (deliberately NOT
//! OPA/Rego; the Rego scaffold in this module belongs to specs 02–04).
//!
//! File format (`.imperium/policy.imp`), one rule per line:
//!
//! ```text
//! # comment
//! deny <verb> matching <glob>      # glob over the full resolved path (or
//!                                  # hostname, for fetch), * = any
//! deny containing <text>           # substring deny on the action text
//! require approval <verb>          # verb always needs explicit approval
//! allow <verb> under <prefix>      # explicit allow (first match wins)
//! allow fetch to <host>            # network allowlist (default-deny!)
//! ```
//!
//! Evaluation is first-match-wins in file order. Filesystem verbs fall
//! through to the grants layer on no match; **fetch falls through to DENY**
//! — nothing is fetchable without an explicit `allow fetch` rule.
//! Verbs: read, write, append, list, fetch.
//!
//! Layering (each layer can only deny more, never less):
//! 1. built-in host firewall (sensitive paths) — hard-coded
//! 2. built-in content denies (proposal-time ceiling) — hard-coded
//! 3. user policy (this file) — loaded fail-closed
//! 4. grants file — narrow-only permissions

use crate::v0::path_allowed;
use serde::{Deserialize, Serialize};

pub const POLICY_VERBS: &[&str] = &["read", "write", "append", "list", "fetch"];

/// Built-in content-deny ceiling (formerly the proposer BANNED list).
/// Applies at proposal time to every verb, substring + case-insensitive.
pub const BUILTIN_CONTENT_DENIES: &[&str] = &[
    "rm ",
    "sudo",
    "curl",
    "wget",
    "http:",
    "https:",
    "ftp:",
    "shell",
    "bash",
    "powershell",
    "eval",
    "network",
    "download",
    "install",
    "chmod",
    "chown",
    "drop table",
    "delete from",
];
pub const BUILTIN_CONTENT_DENIED_MESSAGE: &str =
    "Proposal rejected: banned verb or network reference.";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PolicyRule {
    PathDeny { verb: String, pattern: String },
    ContentDeny { substring: String },
    RequireApproval { verb: String },
    Allow { verb: String, prefix: String },
}

impl PolicyRule {
    /// Canonical one-line form; doubles as the deny reason.
    pub fn canonical(&self) -> String {
        match self {
            Self::PathDeny { verb, pattern } => format!("deny {verb} matching {pattern}"),
            Self::ContentDeny { substring } => format!("deny containing {substring}"),
            Self::RequireApproval { verb } => format!("require approval {verb}"),
            Self::Allow { verb, prefix } => {
                if verb == "fetch" {
                    format!("allow {verb} to {prefix}")
                } else {
                    format!("allow {verb} under {prefix}")
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow {
        /// Some(i): an explicit `allow` rule matched; None: fall-through.
        rule: Option<usize>,
    },
    RequireApproval {
        rule: usize,
    },
    Deny {
        rule: usize,
        /// Canonical text of the matched rule.
        reason: String,
    },
}

impl PolicyDecision {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Allow { .. } => "allow",
            Self::RequireApproval { .. } => "require_approval",
            Self::Deny { .. } => "deny",
        }
    }
}

/// Wildcard glob match over the full path; `*` matches any sequence
/// (including `/`). Case-insensitive.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark) = (usize::MAX, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '*' || p[pi] == t[ti]) {
            if p[pi] == '*' {
                star = pi;
                mark = ti;
                pi += 1;
            } else {
                pi += 1;
                ti += 1;
            }
        } else if star != usize::MAX {
            pi = star + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn is_policy_verb(verb: &str) -> bool {
    POLICY_VERBS.contains(&verb)
}

/// Parse with per-rule line tracking. Returns (rules, issues); issues are
/// non-fatal for lint but make enforcement fail closed.
pub fn parse_rules(text: &str) -> (Vec<(usize, PolicyRule)>, Vec<ParseIssue>) {
    let mut rules = vec![];
    let mut issues = vec![];
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let rule = match tokens.as_slice() {
            ["deny", verb, "matching", pattern] if tokens.len() == 4 => {
                if !is_policy_verb(verb) {
                    issues.push(ParseIssue {
                        line: line_no,
                        message: format!("unknown verb: {verb}"),
                    });
                    continue;
                }
                PolicyRule::PathDeny {
                    verb: verb.to_string(),
                    pattern: pattern.to_string(),
                }
            }
            ["require", "approval", verb] if tokens.len() == 3 => {
                if !is_policy_verb(verb) {
                    issues.push(ParseIssue {
                        line: line_no,
                        message: format!("unknown verb: {verb}"),
                    });
                    continue;
                }
                PolicyRule::RequireApproval {
                    verb: verb.to_string(),
                }
            }
            ["allow", verb, "under", prefix] if tokens.len() == 4 => {
                if !is_policy_verb(verb) {
                    issues.push(ParseIssue {
                        line: line_no,
                        message: format!("unknown verb: {verb}"),
                    });
                    continue;
                }
                PolicyRule::Allow {
                    verb: verb.to_string(),
                    prefix: prefix.to_string(),
                }
            }
            ["allow", "fetch", "to", host] if tokens.len() == 4 => PolicyRule::Allow {
                verb: "fetch".into(),
                prefix: host.to_string(),
            },
            _ if line.starts_with("deny containing ") => PolicyRule::ContentDeny {
                substring: line["deny containing ".len()..].trim().to_string(),
            },
            _ => {
                issues.push(ParseIssue {
                    line: line_no,
                    message: "unrecognized rule".into(),
                });
                continue;
            }
        };
        rules.push((line_no, rule));
    }
    (rules, issues)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseIssue {
    pub line: usize,
    pub message: String,
}

/// Parse a policy file. Lenient: invalid lines are skipped and reported.
pub fn parse_policy_lenient(text: &str) -> (Policy, Vec<ParseIssue>) {
    let (rules, issues) = parse_rules(text);
    (
        Policy {
            rules: rules.into_iter().map(|(_, r)| r).collect(),
        },
        issues,
    )
}

impl Policy {
    /// First-match-wins evaluation. `text` is the action's display text
    /// (echo → the text; write/append → path + contents; read/list → path).
    pub fn evaluate(&self, verb: &str, path: &str, text: &str) -> PolicyDecision {
        let lower_text = text.to_lowercase();
        for (i, rule) in self.rules.iter().enumerate() {
            match rule {
                PolicyRule::PathDeny { verb: v, pattern }
                    if v == verb && glob_match(pattern, path) =>
                {
                    return PolicyDecision::Deny {
                        rule: i,
                        reason: rule.canonical(),
                    };
                }
                PolicyRule::ContentDeny { substring }
                    if lower_text.contains(&substring.to_lowercase()) =>
                {
                    return PolicyDecision::Deny {
                        rule: i,
                        reason: rule.canonical(),
                    };
                }
                PolicyRule::RequireApproval { verb: v } if v == verb => {
                    return PolicyDecision::RequireApproval { rule: i };
                }
                PolicyRule::Allow { verb: v, prefix }
                    if v == verb
                        && (if v == "fetch" {
                            // Host allowlist: glob over the hostname
                            // (so `*.github.com` style entries work).
                            glob_match(prefix, path)
                        } else {
                            path_allowed(path, std::slice::from_ref(prefix))
                        }) =>
                {
                    return PolicyDecision::Allow { rule: Some(i) };
                }
                _ => {}
            }
        }
        PolicyDecision::Allow { rule: None }
    }

    /// Whether any rule forces explicit approval for this verb.
    pub fn requires_approval(&self, verb: &str) -> bool {
        self.rules
            .iter()
            .any(|r| matches!(r, PolicyRule::RequireApproval { verb: v } if v == verb))
    }

    /// Proposal-time content check against this policy's rules only.
    pub fn content_denied(&self, text: &str) -> Option<(usize, String)> {
        let lower = text.to_lowercase();
        for (i, rule) in self.rules.iter().enumerate() {
            if let PolicyRule::ContentDeny { substring } = rule {
                if lower.contains(&substring.to_lowercase()) {
                    return Some((i, rule.canonical()));
                }
            }
        }
        None
    }
}

/// Built-in proposal-time content ceiling (all verbs).
pub fn builtin_content_denied(text: &str) -> bool {
    let lower = text.to_lowercase();
    BUILTIN_CONTENT_DENIES.iter().any(|b| lower.contains(b))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LintIssue {
    pub line: usize,
    pub severity: Severity,
    pub message: String,
}

const HOST_FIREWALL_PATTERNS: &[&str] = &["*.key", "*secret*", ".env", ".env.*"];

/// Lint a policy source. Reports parse issues (Error) plus:
/// duplicates, allows shadowed by earlier denies, and denies that are
/// already enforced by the built-in host firewall.
pub fn lint_policy(text: &str) -> Vec<LintIssue> {
    let (rules, issues) = parse_rules(text);
    let mut out: Vec<LintIssue> = issues
        .into_iter()
        .map(|i| LintIssue {
            line: i.line,
            severity: Severity::Error,
            message: i.message,
        })
        .collect();
    for (j, (line_j, rule_j)) in rules.iter().enumerate() {
        // Duplicate of an earlier identical rule.
        if let Some((line_i, _)) = rules[..j].iter().find(|(_, r)| r == rule_j) {
            out.push(LintIssue {
                line: *line_j,
                severity: Severity::Warning,
                message: format!("duplicate of rule on line {line_i}"),
            });
            continue;
        }
        match rule_j {
            PolicyRule::Allow { verb, prefix } => {
                // Unreachable when an earlier deny for the same verb covers
                // every path this allow could match.
                let shadow = rules[..j].iter().find_map(|(li, r)| {
                    matches!(r,
                        PolicyRule::PathDeny { verb: v, pattern }
                        if v == verb && covers(pattern, prefix)
                    )
                    .then_some(*li)
                });
                if let Some(line_i) = shadow {
                    out.push(LintIssue {
                        line: *line_j,
                        severity: Severity::Warning,
                        message: format!("unreachable: deny on line {line_i} covers this allow"),
                    });
                }
            }
            PolicyRule::PathDeny { pattern, .. }
                if HOST_FIREWALL_PATTERNS.contains(&pattern.as_str()) =>
            {
                out.push(LintIssue {
                    line: *line_j,
                    severity: Severity::Warning,
                    message: "already enforced by the built-in host firewall".into(),
                });
            }
            _ => {}
        }
    }
    out
}

/// Does deny-glob `pattern` cover every path under `prefix`?
fn covers(pattern: &str, prefix: &str) -> bool {
    pattern == "*"
        || pattern == prefix
        || (pattern.ends_with("/*") && prefix.starts_with(&pattern[..pattern.len() - 1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches() {
        assert!(glob_match("*.key", "scratch/server.key"));
        assert!(glob_match("scratch/notes*", "scratch/notes.txt"));
        assert!(glob_match("*secret*", "scratch/my-secrets.txt"));
        assert!(glob_match("*", "anything/at/all"));
        assert!(!glob_match("*.key", "scratch/notes.txt"));
        assert!(!glob_match("scratch/notes*", "scratch/other.txt"));
        // Case-insensitive
        assert!(glob_match("*.KEY", "scratch/SERVER.key"));
    }

    #[test]
    fn parses_all_rule_forms() {
        let (policy, issues) = parse_policy_lenient(
            "# header\n\ndeny read matching *.key\ndeny containing curl\nrequire approval write\nallow read under scratch/\n",
        );
        assert!(issues.is_empty());
        assert_eq!(policy.rules.len(), 4);
        assert_eq!(policy.rules[0].canonical(), "deny read matching *.key");
        assert_eq!(policy.rules[1].canonical(), "deny containing curl");
        assert_eq!(policy.rules[2].canonical(), "require approval write");
        assert_eq!(policy.rules[3].canonical(), "allow read under scratch/");
    }

    #[test]
    fn parse_errors_are_reported_with_lines() {
        let (_, issues) = parse_policy_lenient("allow everything\ndeny rm matching *");
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].line, 1);
        assert_eq!(issues[0].message, "unrecognized rule");
        assert_eq!(issues[1].line, 2);
        assert_eq!(issues[1].message, "unknown verb: rm");
    }

    #[test]
    fn first_match_wins() {
        let (policy, _) =
            parse_policy_lenient("deny read matching scratch/notes*\nallow read under scratch/");
        assert_eq!(
            policy.evaluate("read", "scratch/notes.txt", "scratch/notes.txt"),
            PolicyDecision::Deny {
                rule: 0,
                reason: "deny read matching scratch/notes*".into()
            }
        );
        assert_eq!(
            policy.evaluate("read", "scratch/other.txt", "scratch/other.txt"),
            PolicyDecision::Allow { rule: Some(1) }
        );
        assert_eq!(
            policy.evaluate("write", "scratch/x.txt", "scratch/x.txt"),
            PolicyDecision::Allow { rule: None }
        );
    }

    #[test]
    fn content_deny_evaluation() {
        let (policy, _) = parse_policy_lenient("deny containing curl");
        assert!(policy.content_denied("get https://x via curl").is_some());
        assert!(policy.content_denied("hello world").is_none());
        assert!(builtin_content_denied("DROP TABLE users;"));
        assert!(!builtin_content_denied("hello world"));
    }

    #[test]
    fn require_approval_flag() {
        let (policy, _) = parse_policy_lenient("require approval append");
        assert!(policy.requires_approval("append"));
        assert!(!policy.requires_approval("read"));
        assert!(matches!(
            policy.evaluate("append", "scratch/log.txt", "scratch/log.txt"),
            PolicyDecision::RequireApproval { rule: 0 }
        ));
    }

    #[test]
    fn lint_finds_duplicates_and_shadows() {
        let issues = lint_policy("deny read matching *.key\ndeny read matching *.key");
        assert_eq!(issues.len(), 2); // host-firewall redundancy + duplicate
        assert_eq!(issues[1].message, "duplicate of rule on line 1");

        let issues = lint_policy("deny read matching scratch/*\nallow read under scratch/");
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].message,
            "unreachable: deny on line 1 covers this allow"
        );
    }
}
