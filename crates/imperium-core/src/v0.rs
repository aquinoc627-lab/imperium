//! Working v0 slice. Same contract as `web/v0`.

use crate::intent::{
    Goal, GoalCategory, IntentIR, IntentId, Priority, SuccessCriterion, Task, TaskId,
    TaskKind, Threshold, ThresholdOperator,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashSet;

type HmacSha256 = Hmac<Sha256>;

pub const ECHO_CAP: &str = "cap.echo";
pub const WRITE_CAP: &str = "cap.write";
pub const COMPILER_VERSION: &str = "imperium-intent-rules-0.1.0";
pub const SCRATCH_PREFIX: &str = "scratch";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Permissions {
    pub fs: Vec<String>,
    pub net: Vec<String>,
    pub env: Vec<String>,
}

impl Permissions {
    pub fn empty() -> Self {
        Self {
            fs: vec![],
            net: vec![],
            env: vec![],
        }
    }
}

pub fn grant_for_capability(capability: &str) -> Permissions {
    if capability == WRITE_CAP {
        Permissions {
            fs: vec![SCRATCH_PREFIX.to_string()],
            net: vec![],
            env: vec![],
        }
    } else {
        Permissions::empty()
    }
}

pub fn resolve_scratch_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Path is empty.".into());
    }
    if trimmed.contains('\0') {
        return Err("Path contains NUL.".into());
    }
    let norm = trimmed.replace('\\', "/");
    if norm.starts_with('/') {
        return Err("Absolute paths are denied.".into());
    }
    if norm.chars().nth(1) == Some(':') {
        return Err("Drive-letter paths are denied.".into());
    }
    let parts: Vec<&str> = norm.split('/').filter(|p| !p.is_empty()).collect();
    if parts.iter().any(|p| *p == ".." || *p == ".") {
        return Err("Path escape (..) is denied.".into());
    }
    let rest: Vec<&str> = if parts.first() == Some(&SCRATCH_PREFIX) {
        parts[1..].to_vec()
    } else {
        parts
    };
    if rest.is_empty() {
        return Err("Path must name a file under scratch/.".into());
    }
    if rest.iter().any(|p| *p == ".." || p.contains("..")) {
        return Err("Path escape is denied.".into());
    }
    Ok(format!("{SCRATCH_PREFIX}/{}", rest.join("/")))
}

pub fn path_allowed(requested: &str, granted: &[String]) -> bool {
    if granted.is_empty() {
        return false;
    }
    let norm = requested.replace('\\', "/");
    granted.iter().any(|prefix| {
        let p = prefix.replace('\\', "/");
        let with_slash = if p.ends_with('/') {
            p.clone()
        } else {
            format!("{p}/")
        };
        norm == p || norm.starts_with(&with_slash)
    })
}

pub fn is_permission_subset(requested: &Permissions, granted: &Permissions) -> bool {
    let fs_ok = requested.fs.iter().all(|p| path_allowed(p, &granted.fs));
    let net_ok = requested.net.iter().all(|h| granted.net.iter().any(|g| g == h));
    let env_ok = requested.env.iter().all(|k| granted.env.iter().any(|g| g == k));
    fs_ok && net_ok && env_ok
}

fn starts_ci(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}

pub fn compile_rules(nl: &str) -> Result<IntentIR, String> {
    let source = nl.trim();
    if source.is_empty() {
        return Err("Natural language source is empty.".into());
    }
    if starts_ci(source, "echo this message:") {
        let message = source["echo this message:".len()..].trim();
        if message.is_empty() {
            return Err("Echo message is empty.".into());
        }
        return Ok(echo_ir(source, message));
    }
    if starts_ci(source, "write file ") {
        let rest = &source["write file ".len()..];
        let lower = rest.to_ascii_lowercase();
        let Some(idx) = lower.find(" with contents ") else {
            return Err("v0 rules compiler only accepts: Echo this message: <text>  OR  Write file <path> with contents <text>".into());
        };
        let path = resolve_scratch_path(&rest[..idx])?;
        let contents = &rest[idx + " with contents ".len()..];
        if contents.is_empty() {
            return Err("Write contents are empty.".into());
        }
        return Ok(write_ir(source, &path, contents));
    }
    Err(
        "v0 rules compiler only accepts: Echo this message: <text>  OR  Write file <path> with contents <text>"
            .into(),
    )
}

const BANNED: &[&str] = &[
    "rm ", "sudo", "curl", "wget", "http:", "https:", "ftp:", "shell", "bash",
    "powershell", "eval", "network", "download", "install", "chmod", "chown",
];

pub fn local_propose(nl: &str) -> Result<(String, &'static str), String> {
    let source = nl.trim();
    if source.is_empty() {
        return Err("Natural language source is empty.".into());
    }
    let lower = source.to_ascii_lowercase();
    if BANNED.iter().any(|b| lower.contains(b)) {
        return Err("Proposal rejected: banned verb or network reference.".into());
    }
    if compile_rules(source).is_ok() {
        return Ok((source.to_string(), "rules"));
    }
    for verb in ["say ", "print ", "repeat ", "echo ", "tell me "] {
        if starts_ci(source, verb) {
            let text = source[verb.len()..].trim().trim_matches(|c| c == '"' || c == '\'');
            if !text.is_empty() {
                return Ok((format!("Echo this message: {text}"), "local"));
            }
        }
    }
    for verb in ["save ", "write ", "create ", "put "] {
        if let Some(rest) = source.get(verb.len()..) {
            let lower_rest = rest.to_ascii_lowercase();
            for sep in [" with ", " containing ", " as "] {
                if let Some(idx) = lower_rest.find(sep) {
                    let mut path = rest[..idx].trim();
                    if let Some(stripped) = path.strip_prefix("file ") {
                        path = stripped;
                    }
                    if let Some(stripped) = path.strip_prefix("a file ") {
                        path = stripped;
                    }
                    let contents = rest[idx + sep.len()..].trim().trim_matches(|c| c == '"' || c == '\'');
                    if !path.is_empty() && !contents.is_empty() {
                        return Ok((format!("Write file {path} with contents {contents}"), "local"));
                    }
                }
            }
        }
    }
    Err("Local proposer could not map this to echo or write.".into())
}

fn echo_ir(source: &str, message: &str) -> IntentIR {
    let name = format!("Echo {}", truncate(message, 40));
    base_ir(
        name,
        source,
        &format!("Echo the text {message}"),
        Task {
            id: TaskId::new(),
            name: "Echo".into(),
            description: message.into(),
            kind: TaskKind::Custom,
            capabilities: vec![ECHO_CAP.into()],
            dependencies: vec![],
            estimated_duration_ms: Some(10),
            target_path: None,
            retry_policy: Default::default(),
            compensation: None,
        },
    )
}

fn write_ir(source: &str, path: &str, contents: &str) -> IntentIR {
    base_ir(
        format!("Write {path}"),
        source,
        &format!("Write {path}"),
        Task {
            id: TaskId::new(),
            name: "Write".into(),
            description: contents.into(),
            kind: TaskKind::Custom,
            capabilities: vec![WRITE_CAP.into()],
            dependencies: vec![],
            estimated_duration_ms: Some(20),
            target_path: Some(path.into()),
            retry_policy: Default::default(),
            compensation: None,
        },
    )
}

fn base_ir(name: String, source: &str, goal: &str, task: Task) -> IntentIR {
    IntentIR {
        id: IntentId::new(),
        name,
        nl_source: source.into(),
        goal: Goal {
            description: goal.into(),
            category: GoalCategory::Automation,
            priority: Priority::Normal,
        },
        constraints: vec![],
        success_criteria: vec![SuccessCriterion {
            id: "sc1".into(),
            metric: crate::intent::Metric::TestPassRate,
            threshold: Threshold {
                operator: ThresholdOperator::GreaterThanOrEqual,
                value: 1.0,
                unit: "ratio".into(),
            },
            weight: 1.0,
        }],
        tasks: vec![task],
        risk_score: 0.0,
        requires_approval: true,
        version: 1,
        compiled_at: Some(chrono::Utc::now()),
        compiler_version: Some(COMPILER_VERSION.into()),
        hash: None,
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        s[..n].to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationResult {
    pub success_probability: f32,
    pub risk: f32,
    pub duration_ms: u64,
    pub notes: Vec<String>,
}

pub fn simulate_static(ir: &IntentIR) -> SimulationResult {
    let caps: Vec<&str> = ir
        .tasks
        .iter()
        .flat_map(|t| t.capabilities.iter().map(|c| c.as_str()))
        .collect();
    let known = !caps.is_empty()
        && caps
            .iter()
            .all(|c| *c == ECHO_CAP || *c == WRITE_CAP);
    let duration_ms = ir
        .tasks
        .iter()
        .map(|t| t.estimated_duration_ms.unwrap_or(1000))
        .sum();
    SimulationResult {
        success_probability: if known { 1.0 } else { 0.0 },
        risk: if known { 0.0 } else { 1.0 },
        duration_ms,
        notes: if known {
            caps.into_iter()
                .map(|c| format!("Capability {c} allowed."))
                .collect()
        } else {
            vec!["Unknown or missing capability. Execution would be denied.".into()]
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub id: String,
    pub capability: String,
    pub subject: String,
    pub intent_id: String,
    pub permissions: Permissions,
    pub nonce: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub signature: String,
}

fn canonical_payload(t: &CapabilityToken) -> String {
    let mut env = t.permissions.env.clone();
    let mut fs = t.permissions.fs.clone();
    let mut net = t.permissions.net.clone();
    env.sort();
    fs.sort();
    net.sort();
    serde_json::json!({
        "capability": t.capability,
        "expires_at": t.expires_at,
        "id": t.id,
        "intent_id": t.intent_id,
        "issued_at": t.issued_at,
        "nonce": t.nonce,
        "permissions": { "env": env, "fs": fs, "net": net },
        "subject": t.subject,
    })
    .to_string()
}

pub fn sign_token(token: &CapabilityToken, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(canonical_payload(token).as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn issue_token(
    capability: &str,
    subject: &str,
    intent_id: &str,
    permissions: Permissions,
    now_ms: i64,
    ttl_ms: i64,
    secret: &str,
) -> CapabilityToken {
    let mut token = CapabilityToken {
        id: uuid::Uuid::new_v4().to_string(),
        capability: capability.into(),
        subject: subject.into(),
        intent_id: intent_id.into(),
        permissions,
        nonce: uuid::Uuid::new_v4().to_string(),
        issued_at: now_ms,
        expires_at: now_ms + ttl_ms,
        signature: String::new(),
    };
    token.signature = sign_token(&token, secret);
    token
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyReason {
    Ok,
    EmptySignature,
    InvalidSignature,
    Expired,
    NonceReused,
    Revoked,
    UnknownCapability,
    NetworkDenied,
    PermissionNotSubset,
    SubjectMismatch,
    IntentMismatch,
}

impl VerifyReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::EmptySignature => "empty signature",
            Self::InvalidSignature => "invalid signature",
            Self::Expired => "expired",
            Self::NonceReused => "nonce reused",
            Self::Revoked => "revoked",
            Self::UnknownCapability => "unknown capability",
            Self::NetworkDenied => "network denied",
            Self::PermissionNotSubset => "permission not subset",
            Self::SubjectMismatch => "subject mismatch",
            Self::IntentMismatch => "intent mismatch",
        }
    }
}

pub fn verify_token(
    token: &CapabilityToken,
    secret: &str,
    now_ms: i64,
    seen_nonces: &HashSet<String>,
    revoked_ids: &HashSet<String>,
    grant: &Permissions,
    expected_subject: Option<&str>,
    expected_intent: Option<&str>,
) -> VerifyReason {
    if token.signature.is_empty() {
        return VerifyReason::EmptySignature;
    }
    if sign_token(token, secret) != token.signature {
        return VerifyReason::InvalidSignature;
    }
    if now_ms >= token.expires_at {
        return VerifyReason::Expired;
    }
    if revoked_ids.contains(&token.id) {
        return VerifyReason::Revoked;
    }
    if seen_nonces.contains(&token.nonce) {
        return VerifyReason::NonceReused;
    }
    if token.capability != ECHO_CAP && token.capability != WRITE_CAP {
        return VerifyReason::UnknownCapability;
    }
    if !token.permissions.net.is_empty() {
        return VerifyReason::NetworkDenied;
    }
    if !is_permission_subset(&token.permissions, grant) {
        return VerifyReason::PermissionNotSubset;
    }
    if expected_subject.is_some_and(|s| s != token.subject) {
        return VerifyReason::SubjectMismatch;
    }
    if expected_intent.is_some_and(|s| s != token.intent_id) {
        return VerifyReason::IntentMismatch;
    }
    VerifyReason::Ok
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    Compiled,
    Simulated,
    Approved,
    Executed,
    Failed,
}

impl std::fmt::Display for IntentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Compiled => "compiled",
            Self::Simulated => "simulated",
            Self::Approved => "approved",
            Self::Executed => "executed",
            Self::Failed => "failed",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V0Event {
    pub kind: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct FoldedState {
    pub status: Option<IntentStatus>,
    pub output: Option<String>,
    pub fail_reason: Option<String>,
}

pub fn fold_events(events: &[V0Event]) -> FoldedState {
    let mut state = FoldedState {
        status: Some(IntentStatus::Compiled),
        ..Default::default()
    };
    for ev in events {
        match ev.kind.as_str() {
            "IntentSimulated" => state.status = Some(IntentStatus::Simulated),
            "IntentApproved" => state.status = Some(IntentStatus::Approved),
            "TaskSucceeded" => {
                state.status = Some(IntentStatus::Executed);
                state.output = ev
                    .payload
                    .get("output")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                state.fail_reason = None;
            }
            "TaskFailed" => {
                state.status = Some(IntentStatus::Failed);
                state.fail_reason = ev
                    .payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            _ => {}
        }
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_compiles_and_simulates() {
        let ir = compile_rules("Echo this message: ping").unwrap();
        assert_eq!(ir.tasks[0].capabilities, vec![ECHO_CAP]);
        let sim = simulate_static(&ir);
        assert_eq!(sim.success_probability, 1.0);
        assert_eq!(sim.risk, 0.0);
    }

    #[test]
    fn write_escape_denied() {
        assert!(compile_rules("Write file ../secret with contents x").is_err());
        assert!(compile_rules("Write file /etc/passwd with contents x").is_err());
    }

    #[test]
    fn write_lands_under_scratch() {
        let ir = compile_rules("Write file notes.txt with contents hi").unwrap();
        assert_eq!(ir.tasks[0].target_path.as_deref(), Some("scratch/notes.txt"));
    }

    #[test]
    fn propose_say_hello() {
        let (c, src) = local_propose("say hello").unwrap();
        assert_eq!(src, "local");
        assert_eq!(c, "Echo this message: hello");
    }

    #[test]
    fn propose_bans_curl() {
        assert!(local_propose("curl https://evil.example").is_err());
    }

    #[test]
    fn token_roundtrip_and_fail_closed() {
        let secret = "s";
        let now = 1_000_000;
        let token = issue_token(
            ECHO_CAP,
            "cli",
            "intent-1",
            Permissions::empty(),
            now,
            60_000,
            secret,
        );
        assert_eq!(
            verify_token(
                &token,
                secret,
                now,
                &HashSet::new(),
                &HashSet::new(),
                &Permissions::empty(),
                Some("cli"),
                Some("intent-1"),
            ),
            VerifyReason::Ok
        );
        let mut bad = token.clone();
        bad.signature.clear();
        assert_eq!(
            verify_token(
                &bad,
                secret,
                now,
                &HashSet::new(),
                &HashSet::new(),
                &Permissions::empty(),
                None,
                None,
            ),
            VerifyReason::EmptySignature
        );
        assert_eq!(
            verify_token(
                &token,
                secret,
                now + 120_000,
                &HashSet::new(),
                &HashSet::new(),
                &Permissions::empty(),
                None,
                None,
            ),
            VerifyReason::Expired
        );
        let mut net = token.clone();
        net.permissions.net.push("evil".into());
        net.signature = sign_token(&net, secret);
        assert_eq!(
            verify_token(
                &net,
                secret,
                now,
                &HashSet::new(),
                &HashSet::new(),
                &Permissions::empty(),
                None,
                None,
            ),
            VerifyReason::NetworkDenied
        );
    }

    #[test]
    fn fold_happy_path() {
        let folded = fold_events(&[
            V0Event {
                kind: "IntentCompiled".into(),
                payload: serde_json::json!({}),
            },
            V0Event {
                kind: "IntentSimulated".into(),
                payload: serde_json::json!({}),
            },
            V0Event {
                kind: "IntentApproved".into(),
                payload: serde_json::json!({}),
            },
            V0Event {
                kind: "TaskSucceeded".into(),
                payload: serde_json::json!({"output": "ping"}),
            },
        ]);
        assert_eq!(folded.status, Some(IntentStatus::Executed));
        assert_eq!(folded.output.as_deref(), Some("ping"));
    }
}
