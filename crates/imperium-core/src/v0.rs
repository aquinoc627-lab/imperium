//! Working v0 slice. Same contract as `web/v0`.

use crate::intent::{
    Effect, Goal, GoalCategory, IntentIR, IntentId, Priority, SuccessCriterion, Task, TaskId,
    TaskKind, Threshold, ThresholdOperator,
};
use crate::policy::imp::PolicyDecision;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashSet;

type HmacSha256 = Hmac<Sha256>;

pub const ECHO_CAP: &str = "cap.echo";
pub const WRITE_CAP: &str = "cap.write";
pub const READ_CAP: &str = "cap.read";
pub const APPEND_CAP: &str = "cap.append";
pub const LIST_CAP: &str = "cap.list";
pub const HTTP_CAP: &str = "cap.http";
pub const COMPILER_VERSION: &str = "imperium-intent-rules-0.4.0";
pub const SCRATCH_PREFIX: &str = "scratch";

/// Every capability the v0 kernel knows. Unknown capabilities simulate
/// as risk 1.0 and are denied at token verification.
pub const KNOWN_CAPABILITIES: &[&str] = &[
    ECHO_CAP, WRITE_CAP, READ_CAP, APPEND_CAP, LIST_CAP, HTTP_CAP,
];

/// Air-gap anchor: model-provider (cloud LLM) routing is denied *in code*,
/// never in config. `cap.http` is different: it is user-declared capability
/// traffic (policy-allowlisted hosts, approved at every use) — not model
/// routing. No model provider exists in the kernel, so this stays false.
pub const ALLOW_CLOUD_ROUTING: bool = false;

/// Risk weight per verb (Phase 8): drives `requires_approval` via
/// `APPROVAL_THRESHOLD`. Unknown verbs are maximum risk.
pub fn risk_for_capability(capability: &str) -> f32 {
    match capability {
        ECHO_CAP => 0.0,
        LIST_CAP => 0.1,
        READ_CAP => 0.2,
        APPEND_CAP => 0.4,
        WRITE_CAP => 0.5,
        HTTP_CAP => 0.6,
        _ => 1.0,
    }
}

/// Intents at or above this risk require explicit approval; below it,
/// execution may auto-approve (still emitting full audit events).
pub const APPROVAL_THRESHOLD: f32 = 0.4;

/// Sensitive-path patterns. Matched per path segment, case-insensitive.
/// Hard-coded at the host layer — policy (Phase 9) can only deny more,
/// never less.
pub const SENSITIVE_PATTERNS: &[&str] = &["*.key", "*secret*", ".env", ".env.*"];
pub const SENSITIVE_DENIED_REASON: &str = "sensitive path denied";

pub fn is_sensitive_path(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    norm.split('/').any(|seg| {
        let l = seg.to_ascii_lowercase();
        l.ends_with(".key") || l.contains("secret") || l == ".env" || l.starts_with(".env.")
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
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
    let fs = if capability == WRITE_CAP
        || capability == READ_CAP
        || capability == APPEND_CAP
        || capability == LIST_CAP
    {
        vec![SCRATCH_PREFIX.to_string()]
    } else {
        vec![]
    };
    let net = if capability == HTTP_CAP {
        // Ceiling only: the user may *declare* specific hosts. Nothing is
        // fetchable until grants.json + policy both allow a host.
        vec!["*".to_string()]
    } else {
        vec![]
    };
    Permissions {
        fs,
        net,
        env: vec![],
    }
}

/// Built-in ceiling for every capability. The grants file (CLI) may only
/// narrow these; anything wider is rejected fail-closed.
pub fn default_grants() -> std::collections::BTreeMap<String, Permissions> {
    KNOWN_CAPABILITIES
        .iter()
        .map(|c| (c.to_string(), grant_for_capability(c)))
        .collect()
}

/// What `init` writes: the ceiling with network closed. The user must
/// explicitly declare hosts in grants.json (within the `*` ceiling) and
/// allowlist them in policy.imp before any fetch executes.
pub fn initial_grants() -> std::collections::BTreeMap<String, Permissions> {
    let mut grants = default_grants();
    if let Some(p) = grants.get_mut(HTTP_CAP) {
        p.net = vec![];
    }
    grants
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
    let net_ok = requested
        .net
        .iter()
        .all(|h| granted.net.iter().any(|g| g == "*" || g == h));
    let env_ok = requested
        .env
        .iter()
        .all(|k| granted.env.iter().any(|g| g == k));
    fs_ok && net_ok && env_ok
}

/// URL checks for `Fetch <url>`: https only, no explicit port, hostname
/// must not be an IP literal or a local/internal name (SSRF hardening).
/// Returns the normalized lowercase host.
pub const FETCH_HTTPS_REQUIRED: &str = "https required";
pub const FETCH_PORT_DENIED: &str = "port not allowed";
pub const FETCH_IP_DENIED: &str = "ip addresses denied";
pub const FETCH_LOCAL_DENIED: &str = "local address denied";
pub const FETCH_URL_INVALID: &str = "invalid url";
pub const FETCH_NO_ALLOW_RULE: &str = "no allow fetch rule matched";

pub fn fetch_url_host(url: &str) -> Result<String, String> {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    if lower.is_empty() || url.contains(char::is_whitespace) {
        return Err(FETCH_URL_INVALID.into());
    }
    let Some(rest) = lower.strip_prefix("https://") else {
        return Err(FETCH_HTTPS_REQUIRED.into());
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() {
        return Err(FETCH_URL_INVALID.into());
    }
    if authority.contains('@') {
        return Err(FETCH_URL_INVALID.into());
    }
    if authority.contains(':') {
        return Err(FETCH_PORT_DENIED.into());
    }
    let host = authority.trim_end_matches('.');
    if host.is_empty() {
        return Err(FETCH_URL_INVALID.into());
    }
    // IPv4 literals (IPv6 needs colons, already rejected as ports above).
    let parts: Vec<&str> = host.split('.').collect();
    let looks_ipv4 = parts.len() == 4
        && parts
            .iter()
            .all(|seg| !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()));
    if looks_ipv4 {
        return Err(FETCH_IP_DENIED.into());
    }
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
    {
        return Err(FETCH_LOCAL_DENIED.into());
    }
    if !host.contains('.') {
        return Err(FETCH_URL_INVALID.into());
    }
    Ok(host.to_string())
}

fn starts_ci(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}

pub const COMPILE_USAGE_ERROR: &str = "v0 rules compiler only accepts: Echo this message: <text>  OR  Write file <path> with contents <text>  OR  Read file <path>  OR  Append file <path> with contents <text>  OR  List files under <path>  OR  Fetch <https-url>";

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
            return Err(COMPILE_USAGE_ERROR.into());
        };
        let path = resolve_scratch_path(&rest[..idx])?;
        let contents = &rest[idx + " with contents ".len()..];
        if contents.is_empty() {
            return Err("Write contents are empty.".into());
        }
        return Ok(write_ir(source, &path, contents));
    }
    if starts_ci(source, "read file ") {
        let path = resolve_scratch_path(&source["read file ".len()..])?;
        return Ok(read_ir(source, &path));
    }
    if starts_ci(source, "append file ") {
        let rest = &source["append file ".len()..];
        let lower = rest.to_ascii_lowercase();
        let Some(idx) = lower.find(" with contents ") else {
            return Err(COMPILE_USAGE_ERROR.into());
        };
        let path = resolve_scratch_path(&rest[..idx])?;
        let contents = &rest[idx + " with contents ".len()..];
        if contents.is_empty() {
            return Err("Append contents are empty.".into());
        }
        return Ok(append_ir(source, &path, contents));
    }
    if starts_ci(source, "list files under ") {
        let path = resolve_scratch_path(&source["list files under ".len()..])?;
        return Ok(list_ir(source, &path));
    }
    if starts_ci(source, "fetch ") {
        let url = source["fetch ".len()..].trim();
        // Canonical Fetch compiles only if the URL passes every host check;
        // the host allowlist itself is enforced by policy + grants later.
        fetch_url_host(url)?;
        return Ok(fetch_ir(source, url));
    }
    Err(COMPILE_USAGE_ERROR.into())
}

pub fn local_propose(nl: &str) -> Result<(String, &'static str), String> {
    let source = nl.trim();
    if source.is_empty() {
        return Err("Natural language source is empty.".into());
    }
    let lower = source.to_ascii_lowercase();
    if crate::policy::imp::builtin_content_denied(&lower) {
        return Err(crate::policy::imp::BUILTIN_CONTENT_DENIED_MESSAGE.into());
    }
    if compile_rules(source).is_ok() {
        return Ok((source.to_string(), "rules"));
    }
    for verb in ["say ", "print ", "repeat ", "echo ", "tell me "] {
        if starts_ci(source, verb) {
            let text = source[verb.len()..]
                .trim()
                .trim_matches(|c| c == '"' || c == '\'');
            if !text.is_empty() {
                return Ok((format!("Echo this message: {text}"), "local"));
            }
        }
    }
    for verb in ["save ", "write ", "create ", "put "] {
        if starts_ci(source, verb) {
            let rest = &source[verb.len()..];
            let lower_rest = rest.to_ascii_lowercase();
            for sep in [" with ", " containing ", " as "] {
                if let Some(idx) = lower_rest.find(sep) {
                    let path = strip_file_prefix(rest[..idx].trim());
                    let contents = rest[idx + sep.len()..]
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'');
                    if !path.is_empty() && !contents.is_empty() {
                        return Ok((
                            format!("Write file {path} with contents {contents}"),
                            "local",
                        ));
                    }
                }
            }
        }
    }
    // Read: `read [file] <path>`
    for verb in ["read ", "show ", "open ", "cat "] {
        if starts_ci(source, verb) {
            let path = strip_file_prefix(source[verb.len()..].trim());
            if !path.is_empty() {
                return Ok((format!("Read file {path}"), "local"));
            }
        }
    }
    // Append: `append [to] [file] <path> with <text>`
    for verb in ["append to ", "append "] {
        if starts_ci(source, verb) {
            let rest = &source[verb.len()..];
            let lower_rest = rest.to_ascii_lowercase();
            for sep in [" with ", " containing ", " as "] {
                if let Some(idx) = lower_rest.find(sep) {
                    let path = strip_file_prefix(rest[..idx].trim());
                    let contents = rest[idx + sep.len()..]
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'');
                    if !path.is_empty() && !contents.is_empty() {
                        return Ok((
                            format!("Append file {path} with contents {contents}"),
                            "local",
                        ));
                    }
                }
            }
        }
    }
    // List: `list [files under|dir] <path>`
    for verb in ["list ", "ls "] {
        if starts_ci(source, verb) {
            let trimmed = source[verb.len()..].trim();
            let path = trimmed
                .strip_prefix("files under ")
                .or_else(|| trimmed.strip_prefix("dir "))
                .unwrap_or(trimmed);
            let path = strip_file_prefix(path);
            if !path.is_empty() {
                return Ok((format!("List files under {path}"), "local"));
            }
        }
    }
    Err("Local proposer could not map this to echo or write.".into())
}

fn strip_file_prefix(path: &str) -> &str {
    path.strip_prefix("file ")
        .or_else(|| path.strip_prefix("a file "))
        .unwrap_or(path)
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
            effects: vec![Effect::Echo {
                text: message.into(),
            }],
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
            effects: vec![Effect::Write {
                path: path.into(),
                size_bytes: Some(contents.len() as u64),
            }],
            retry_policy: Default::default(),
            compensation: None,
        },
    )
}

fn read_ir(source: &str, path: &str) -> IntentIR {
    base_ir(
        format!("Read {path}"),
        source,
        &format!("Read {path}"),
        Task {
            id: TaskId::new(),
            name: "Read".into(),
            description: path.into(),
            kind: TaskKind::Custom,
            capabilities: vec![READ_CAP.into()],
            dependencies: vec![],
            estimated_duration_ms: Some(15),
            target_path: Some(path.into()),
            effects: vec![Effect::Read { path: path.into() }],
            retry_policy: Default::default(),
            compensation: None,
        },
    )
}

fn append_ir(source: &str, path: &str, contents: &str) -> IntentIR {
    base_ir(
        format!("Append {path}"),
        source,
        &format!("Append {path}"),
        Task {
            id: TaskId::new(),
            name: "Append".into(),
            description: contents.into(),
            kind: TaskKind::Custom,
            capabilities: vec![APPEND_CAP.into()],
            dependencies: vec![],
            estimated_duration_ms: Some(20),
            target_path: Some(path.into()),
            effects: vec![Effect::Append {
                path: path.into(),
                size_bytes: Some(contents.len() as u64),
            }],
            retry_policy: Default::default(),
            compensation: None,
        },
    )
}

fn list_ir(source: &str, path: &str) -> IntentIR {
    base_ir(
        format!("List {path}"),
        source,
        &format!("List {path}"),
        Task {
            id: TaskId::new(),
            name: "List".into(),
            description: path.into(),
            kind: TaskKind::Custom,
            capabilities: vec![LIST_CAP.into()],
            dependencies: vec![],
            estimated_duration_ms: Some(15),
            target_path: Some(path.into()),
            effects: vec![Effect::List { path: path.into() }],
            retry_policy: Default::default(),
            compensation: None,
        },
    )
}

fn fetch_ir(source: &str, url: &str) -> IntentIR {
    base_ir(
        format!("Fetch {url}"),
        source,
        &format!("Fetch {url}"),
        Task {
            id: TaskId::new(),
            name: "Fetch".into(),
            description: url.into(),
            kind: TaskKind::Custom,
            capabilities: vec![HTTP_CAP.into()],
            dependencies: vec![],
            estimated_duration_ms: Some(500),
            target_path: Some(url.into()),
            effects: vec![Effect::Fetch { url: url.into() }],
            retry_policy: Default::default(),
            compensation: None,
        },
    )
}

fn base_ir(name: String, source: &str, goal: &str, task: Task) -> IntentIR {
    let cap = task.capabilities.first().map(String::as_str).unwrap_or("");
    let risk = risk_for_capability(cap);
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
        risk_score: risk,
        requires_approval: risk >= APPROVAL_THRESHOLD,
        version: crate::PROTOCOL_VERSION,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SimulationResult {
    pub success_probability: f32,
    pub risk: f32,
    pub duration_ms: u64,
    pub notes: Vec<String>,
    #[serde(default)]
    pub effects_preview: Vec<EffectPreview>,
    // --- Phase 12: probabilistic dry-run (all defaults; static sims omit) ---
    #[serde(default)]
    pub probabilistic: bool,
    #[serde(default)]
    pub trials: u64,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub p_success: f32,
    #[serde(default)]
    pub p50_ms: u64,
    #[serde(default)]
    pub p95_ms: u64,
    #[serde(default)]
    pub mc_successes: u64,
    #[serde(default)]
    pub factors: Vec<String>,
}

/// One concrete effect an intent would produce if executed, derived by
/// the dry-run simulator without touching any durable state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectPreview {
    Echo {
        text: String,
    },
    Write {
        path: String,
        bytes: u64,
    },
    Read {
        path: String,
    },
    Append {
        path: String,
        bytes: u64,
    },
    List {
        path: String,
    },
    /// GET an allowlisted https URL.
    Fetch {
        url: String,
    },
    /// A denied effect: which capability, which path, why.
    Denied {
        capability: String,
        path: String,
        reason: String,
    },
}
type PreviewMake<'a> = Box<dyn FnOnce(String) -> EffectPreview + 'a>;
type PreviewText<'a> = Box<dyn FnOnce(String) -> String + 'a>;

/// Fetch preview gate: URL validation → policy allowlist (default-deny).
fn preview_fetch(
    url: &str,
    capability: &str,
    policy: Option<&crate::policy::imp::Policy>,
) -> EffectPreview {
    let deny = |reason: String| EffectPreview::Denied {
        capability: capability.into(),
        path: url.to_string(),
        reason,
    };
    let host = match fetch_url_host(url) {
        Ok(h) => h,
        Err(reason) => return deny(reason),
    };
    match policy {
        // No policy at all: network is default-deny.
        None => deny(FETCH_NO_ALLOW_RULE.into()),
        Some(p) => match p.evaluate("fetch", &host, url) {
            PolicyDecision::Deny { reason, .. } => deny(format!("policy: {reason}")),
            PolicyDecision::Allow { rule: Some(_) } => EffectPreview::Fetch {
                url: url.to_string(),
            },
            // Fall-through for fetch = deny (no allow rule covers this host).
            PolicyDecision::Allow { rule: None } => deny(FETCH_NO_ALLOW_RULE.into()),
            PolicyDecision::RequireApproval { .. } => {
                // Fetch is high-risk anyway (always explicit approval); the
                // preview still requires an explicit allow rule.
                deny(FETCH_NO_ALLOW_RULE.into())
            }
        },
    }
}

fn preview_effect(
    effect: &Effect,
    capability: &str,
    policy: Option<&crate::policy::imp::Policy>,
) -> EffectPreview {
    let (op, path, make, preview_text): (&str, &str, PreviewMake<'_>, PreviewText<'_>) =
        match effect {
            Effect::Echo { text } => {
                // Echo is path-free: policy content rules may still deny it.
                if let Some(p) = policy {
                    if let PolicyDecision::Deny { reason, .. } = p.evaluate("echo", "", text) {
                        return EffectPreview::Denied {
                            capability: capability.into(),
                            path: String::new(),
                            reason: format!("policy: {reason}"),
                        };
                    }
                }
                return EffectPreview::Echo { text: text.clone() };
            }
            Effect::Write { path, size_bytes } => (
                "write",
                path,
                Box::new(move |p| EffectPreview::Write {
                    path: p,
                    bytes: size_bytes.unwrap_or(0),
                }),
                Box::new(|p: String| p),
            ),
            Effect::Read { path } => (
                "read",
                path,
                Box::new(move |p| EffectPreview::Read { path: p }),
                Box::new(|p: String| p),
            ),
            Effect::Append { path, size_bytes } => (
                "append",
                path,
                Box::new(move |p| EffectPreview::Append {
                    path: p,
                    bytes: size_bytes.unwrap_or(0),
                }),
                Box::new(|p: String| p),
            ),
            Effect::List { path } => (
                "list",
                path,
                Box::new(move |p| EffectPreview::List { path: p }),
                Box::new(|p: String| p),
            ),
            Effect::Fetch { url } => {
                // Fetch is handled by its own gate: URL checks → policy
                // allowlist (default-deny) → grants. Never reaches the
                // scratch-path machinery below.
                return preview_fetch(url, capability, policy);
            }
        };
    let deny = |resolved: Option<String>, reason: String| EffectPreview::Denied {
        capability: capability.into(),
        path: resolved.unwrap_or_else(|| path.to_string()),
        reason,
    };
    let resolved = match resolve_scratch_path(path) {
        Ok(p) => p,
        Err(reason) => return deny(None, reason),
    };
    if !path_allowed(&resolved, &grant_for_capability(capability).fs) {
        return deny(Some(resolved), format!("host.{op} path denied"));
    }
    if is_sensitive_path(&resolved) {
        return deny(Some(resolved), SENSITIVE_DENIED_REASON.into());
    }
    // User policy layer (can only deny more than the host firewall).
    if let Some(p) = policy {
        let text = preview_text(resolved.clone());
        if let PolicyDecision::Deny { reason, .. } = p.evaluate(op, &resolved, &text) {
            return deny(Some(resolved), format!("policy: {reason}"));
        }
    }
    make(resolved)
}

/// Fold a hypothetical event stream (tagged `dry_run: true`, never persisted)
/// into the resulting simulation plus the stream itself.
pub fn dry_run_events(ir: &IntentIR) -> (SimulationResult, Vec<V0Event>) {
    dry_run_events_with_policy(ir, None)
}

pub fn dry_run_events_with_policy(
    ir: &IntentIR,
    policy: Option<&crate::policy::imp::Policy>,
) -> (SimulationResult, Vec<V0Event>) {
    let mut sim = simulate_static(ir);
    let mut previews: Vec<EffectPreview> = vec![];
    let mut events: Vec<V0Event> = vec![V0Event {
        kind: "IntentSimulated".into(),
        payload: serde_json::json!({
            "dry_run": true,
            "success_probability": sim.success_probability,
            "risk": sim.risk,
            "duration_ms": sim.duration_ms,
            "notes": sim.notes,
        }),
        at: None,
    }];
    for task in &ir.tasks {
        events.push(V0Event {
            kind: "TaskStarted".into(),
            payload: serde_json::json!({"dry_run": true, "task_id": task.id.to_string()}),
            at: None,
        });
        if task.effects.is_empty() {
            sim.notes.push(format!(
                "Task {} declares no effects; preview unavailable.",
                task.name
            ));
            continue;
        }
        for effect in &task.effects {
            let capability = task.capabilities.first().cloned().unwrap_or_default();
            let preview = preview_effect(effect, &capability, policy);
            let denied = matches!(preview, EffectPreview::Denied { .. });
            let payload = match &preview {
                EffectPreview::Echo { text } => serde_json::json!({
                    "dry_run": true,
                    "output": format!("echo {text}"),
                }),
                EffectPreview::Write { path, bytes } => serde_json::json!({
                    "dry_run": true,
                    "output": format!("wrote {path} ({bytes} bytes)"),
                }),
                EffectPreview::Read { path } => serde_json::json!({
                    "dry_run": true,
                    "output": format!("read {path}"),
                }),
                EffectPreview::Append { path, bytes } => serde_json::json!({
                    "dry_run": true,
                    "output": format!("appended {bytes} bytes to {path}"),
                }),
                EffectPreview::List { path } => serde_json::json!({
                    "dry_run": true,
                    "output": format!("list {path}"),
                }),
                EffectPreview::Fetch { url } => serde_json::json!({
                    "dry_run": true,
                    "output": format!("fetch {url}"),
                }),
                EffectPreview::Denied {
                    capability,
                    path,
                    reason,
                } => serde_json::json!({
                    "dry_run": true,
                    "reason": format!("{capability} denied: {path}: {reason}"),
                }),
            };
            events.push(V0Event {
                kind: if denied {
                    "TaskFailed"
                } else {
                    "TaskSucceeded"
                }
                .into(),
                payload,
                at: None,
            });
            previews.push(preview);
        }
    }
    let folded = fold_events(&events);
    if folded.status == Some(IntentStatus::Failed) {
        sim.notes
            .push("Dry-run denied one or more effects; approval is not possible.".into());
        sim.success_probability = 0.0;
        sim.risk = 1.0;
    }
    sim.effects_preview = previews;
    (sim, events)
}

/// Dry-run only: the resulting simulation (events are built and folded, then dropped).
pub fn dry_run(ir: &IntentIR) -> SimulationResult {
    dry_run_events(ir).0
}

/// Dry-run with a user policy layered on top of the host firewall.
pub fn dry_run_with_policy(
    ir: &IntentIR,
    policy: Option<&crate::policy::imp::Policy>,
) -> SimulationResult {
    dry_run_events_with_policy(ir, policy).0
}

pub fn simulate_static(ir: &IntentIR) -> SimulationResult {
    let caps: Vec<&str> = ir
        .tasks
        .iter()
        .flat_map(|t| t.capabilities.iter().map(|c| c.as_str()))
        .collect();
    let known = !caps.is_empty() && caps.iter().all(|c| KNOWN_CAPABILITIES.contains(c));
    let duration_ms = ir
        .tasks
        .iter()
        .map(|t| t.estimated_duration_ms.unwrap_or(1000))
        .sum();
    SimulationResult {
        success_probability: if known { 1.0 } else { 0.0 },
        risk: if known { 0.0 } else { 1.0 },
        duration_ms,
        effects_preview: vec![],
        notes: if known {
            caps.into_iter()
                .map(|c| format!("Capability {c} allowed."))
                .collect()
        } else {
            vec!["Unknown or missing capability. Execution would be denied.".into()]
        },
        ..Default::default()
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

/// Inputs to a token verification. Fail-closed: every check runs unless
/// the expectation is explicitly absent.
pub struct VerifyContext<'a> {
    pub secret: &'a str,
    pub now_ms: i64,
    pub seen_nonces: &'a HashSet<String>,
    pub revoked_ids: &'a HashSet<String>,
    pub grant: &'a Permissions,
    pub expected_subject: Option<&'a str>,
    pub expected_intent: Option<&'a str>,
}

pub fn verify_token(token: &CapabilityToken, ctx: &VerifyContext) -> VerifyReason {
    if token.signature.is_empty() {
        return VerifyReason::EmptySignature;
    }
    if sign_token(token, ctx.secret) != token.signature {
        return VerifyReason::InvalidSignature;
    }
    if ctx.now_ms >= token.expires_at {
        return VerifyReason::Expired;
    }
    if ctx.revoked_ids.contains(&token.id) {
        return VerifyReason::Revoked;
    }
    if ctx.seen_nonces.contains(&token.nonce) {
        return VerifyReason::NonceReused;
    }
    if !KNOWN_CAPABILITIES.contains(&token.capability.as_str()) {
        return VerifyReason::UnknownCapability;
    }
    // Network floor: only cap.http may ever carry net permissions (and even
    // then only hosts its declared grant allows — the subset check below).
    if !token.permissions.net.is_empty() && token.capability != HTTP_CAP {
        return VerifyReason::NetworkDenied;
    }
    if !is_permission_subset(&token.permissions, ctx.grant) {
        return VerifyReason::PermissionNotSubset;
    }
    if ctx.expected_subject.is_some_and(|s| s != token.subject) {
        return VerifyReason::SubjectMismatch;
    }
    if ctx.expected_intent.is_some_and(|s| s != token.intent_id) {
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
    /// Wall-clock epoch ms when the event was recorded. Absent on legacy
    /// events and hypothetical (dry-run) events; enables observed-duration
    /// facts (TaskStarted → TaskSucceeded deltas) in the world model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct FoldedToken {
    pub token_id: String,
    pub fingerprint: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FoldedState {
    pub status: Option<IntentStatus>,
    pub output: Option<String>,
    pub fail_reason: Option<String>,
    pub proposer: Option<String>,
    pub canonical: Option<String>,
    pub simulation: Option<SimulationResult>,
    pub token: Option<FoldedToken>,
}

pub fn fold_events(events: &[V0Event]) -> FoldedState {
    let mut state = FoldedState {
        status: Some(IntentStatus::Compiled),
        ..Default::default()
    };
    for ev in events {
        let payload = &ev.payload;
        match ev.kind.as_str() {
            "IntentProposed" => {
                state.proposer = payload
                    .get("proposer")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                state.canonical = payload
                    .get("canonical")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            "IntentCompiled" => {
                if let Some(p) = payload.get("proposer").and_then(|v| v.as_str()) {
                    state.proposer = Some(p.to_string());
                }
            }
            "IntentSimulated" => {
                state.status = Some(IntentStatus::Simulated);
                state.simulation =
                    Some(serde_json::from_value(payload.clone()).unwrap_or_default());
            }
            "IntentApproved" => state.status = Some(IntentStatus::Approved),
            "TokenIssued" => {
                state.token = Some(FoldedToken {
                    token_id: payload
                        .get("token_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .into(),
                    fingerprint: payload
                        .get("fingerprint")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .into(),
                    revoked: false,
                });
            }
            "TokenRevoked" => {
                if let Some(t) = state.token.as_mut() {
                    t.revoked = true;
                }
            }
            "TaskSucceeded" => {
                // Shadow executions are provenance, not state transitions:
                // the intent stays at its pre-shadow status.
                if payload.get("shadow") == Some(&serde_json::value::Value::Bool(true)) {
                    continue;
                }
                state.status = Some(IntentStatus::Executed);
                state.output = payload
                    .get("output")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                state.fail_reason = None;
            }
            "TaskFailed" => {
                if payload.get("shadow") == Some(&serde_json::value::Value::Bool(true)) {
                    continue;
                }
                state.status = Some(IntentStatus::Failed);
                state.fail_reason = payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            _ => {}
        }
    }
    state
}

// --- Phase 12: probabilistic dry-run (Monte Carlo) ---

/// Observed per-capability facts (a view over the ledger's event log —
/// never a store of its own).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CapabilityStat {
    pub samples: u64,
    pub successes: u64,
    #[serde(default)]
    pub durations_ms: Vec<u64>,
}

pub type WorldStats = std::collections::BTreeMap<String, CapabilityStat>;

/// Minimum observed durations before sampling replaces the static estimate.
pub const MIN_DURATION_SAMPLES: usize = 5;

/// Approval gate for probabilistic simulations: p_success >= 0.9, computed
/// in integer math (milli-units) so Rust f32 and JS f64 agree exactly.
pub const APPROVAL_PROBABILITY_MILLI: u64 = 9000;

pub fn mc_gate(successes: u64, trials: u64) -> bool {
    if trials == 0 {
        return false;
    }
    successes * 10_000 >= APPROVAL_PROBABILITY_MILLI * trials
}

/// Deterministic seed from an intent id (FNV-1a 64).
pub fn derive_seed(id: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in id.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

/// 64-bit LCG (identical constants + shift in the TS mirror; JS uses BigInt).
pub struct MonteCarloRng(u64);

impl MonteCarloRng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn draw(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

fn laplace_milli(stat: &CapabilityStat) -> u64 {
    ((stat.successes + 1) * 10_000) / (stat.samples + 2)
}

fn factor_note(cap: &str, stat: &CapabilityStat) -> String {
    let rate = (stat.successes + 1) as f64 / (stat.samples + 2) as f64;
    if stat.samples as usize >= MIN_DURATION_SAMPLES {
        format!("P({cap}) = {rate:.4} (n={})", stat.samples)
    } else {
        format!("P({cap}) = {rate:.4} (prior, n={})", stat.samples)
    }
}

/// Probabilistic dry-run: hard denials pass through unchanged (p stays 0,
/// `probabilistic` stays false); otherwise roll the task graph `trials`
/// times with the seeded LCG.
///
/// Rollout order (pinned — the TS mirror must draw in exactly this order):
/// per trial, per task, per attempt: duration draw first, then success
/// draw. Backoff (after a failed non-final attempt) adds
/// `backoff_ms * multiplier^attempt` in f64. Success = every task completed.
pub fn dry_run_monte_carlo(
    ir: &IntentIR,
    policy: Option<&crate::policy::imp::Policy>,
    stats: Option<&WorldStats>,
    trials: u64,
    seed: u64,
) -> SimulationResult {
    let base = dry_run_with_policy(ir, policy);
    if base.success_probability == 0.0 {
        // Hard denial: uncertainty never softens a denial.
        return base;
    }
    let mut rng = MonteCarloRng::new(seed);
    let mut successes: u64 = 0;
    let mut success_durations: Vec<u64> = vec![];
    let mut factors: Vec<String> = vec![];
    for task in &ir.tasks {
        let cap = task.capabilities.first().cloned().unwrap_or_default();
        let stat = stats.and_then(|s| s.get(&cap)).cloned().unwrap_or_default();
        factors.push(factor_note(&cap, &stat));
    }
    for _ in 0..trials {
        let mut trial_ok = true;
        let mut total_ms: u64 = 0;
        for task in &ir.tasks {
            let cap = task.capabilities.first().cloned().unwrap_or_default();
            let empty = CapabilityStat::default();
            let stat = stats.and_then(|s| s.get(&cap)).unwrap_or(&empty);
            let p_milli = laplace_milli(stat);
            let attempts = task.retry_policy.max_attempts.max(1) as u64;
            let mut done = false;
            for attempt in 0..attempts {
                let dur = if stat.durations_ms.len() >= MIN_DURATION_SAMPLES {
                    stat.durations_ms[(rng.draw() % stat.durations_ms.len() as u64) as usize]
                } else {
                    task.estimated_duration_ms.unwrap_or(1000)
                };
                total_ms += dur;
                if rng.draw() % 10_000 < p_milli {
                    done = true;
                    break;
                }
                if attempt + 1 < attempts {
                    let backoff = task.retry_policy.backoff_ms as f64
                        * (task.retry_policy.backoff_multiplier as f64).powi(attempt as i32);
                    total_ms += backoff.floor() as u64;
                }
            }
            if !done {
                trial_ok = false;
                break;
            }
        }
        if trial_ok {
            successes += 1;
            success_durations.push(total_ms);
        }
    }
    let mut p50 = 0;
    let mut p95 = 0;
    if !success_durations.is_empty() {
        success_durations.sort_unstable();
        let n = success_durations.len() as u64;
        let i50 = ((n * 50) / 100).min(n - 1) as usize;
        let i95 = ((n * 95) / 100).min(n - 1) as usize;
        p50 = success_durations[i50];
        p95 = success_durations[i95];
    }
    SimulationResult {
        probabilistic: true,
        trials,
        seed,
        p_success: successes as f32 / trials as f32,
        p50_ms: p50,
        p95_ms: p95,
        mc_successes: successes,
        factors,
        ..base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_compiles_and_simulates() {
        let ir = compile_rules("Echo this message: ping").unwrap();
        assert_eq!(ir.tasks[0].capabilities, vec![ECHO_CAP]);
        assert_eq!(ir.version, crate::PROTOCOL_VERSION);
        assert_eq!(
            ir.tasks[0].effects,
            vec![Effect::Echo {
                text: "ping".into()
            }]
        );
        let sim = simulate_static(&ir);
        assert_eq!(sim.success_probability, 1.0);
        assert_eq!(sim.risk, 0.0);
    }

    #[test]
    fn dry_run_previews_exact_write() {
        let ir = compile_rules("Write file notes.txt with contents hello").unwrap();
        let sim = dry_run(&ir);
        assert_eq!(
            sim.effects_preview,
            vec![EffectPreview::Write {
                path: "scratch/notes.txt".into(),
                bytes: 5,
            }]
        );
        assert_eq!(sim.success_probability, 1.0);
        assert_eq!(sim.risk, 0.0);
    }

    #[test]
    fn dry_run_denies_escape_at_simulation() {
        let mut ir = compile_rules("Echo this message: ping").unwrap();
        ir.tasks[0].capabilities = vec![WRITE_CAP.into()];
        ir.tasks[0].effects = vec![Effect::Write {
            path: "../secret".into(),
            size_bytes: Some(1),
        }];
        let sim = dry_run(&ir);
        assert!(
            matches!(
                &sim.effects_preview[0],
                EffectPreview::Denied { capability, path, reason }
                    if capability == WRITE_CAP && path == "../secret" && reason.contains("denied")
            ),
            "expected denial preview, got {:?}",
            sim.effects_preview
        );
        assert_eq!(sim.success_probability, 0.0);
        assert_eq!(sim.risk, 1.0);
    }

    #[test]
    fn dry_run_emits_tagged_events_never_persisted() {
        let ir = compile_rules("Write file notes.txt with contents hello").unwrap();
        let (_sim, events) = dry_run_events(&ir);
        assert!(events
            .iter()
            .all(|e| e.payload.get("dry_run") == Some(&serde_json::value::Value::Bool(true))));
        assert_eq!(events.last().unwrap().kind, "TaskSucceeded");
    }

    #[test]
    fn write_escape_denied() {
        assert!(compile_rules("Write file ../secret with contents x").is_err());
        assert!(compile_rules("Write file /etc/passwd with contents x").is_err());
    }

    #[test]
    fn write_lands_under_scratch() {
        let ir = compile_rules("Write file notes.txt with contents hi").unwrap();
        assert_eq!(
            ir.tasks[0].target_path.as_deref(),
            Some("scratch/notes.txt")
        );
    }

    #[test]
    fn new_verb_forms_compile() {
        let read = compile_rules("Read file notes.txt").unwrap();
        assert_eq!(read.tasks[0].capabilities, vec![READ_CAP]);
        assert_eq!(read.risk_score, 0.2);
        assert!(!read.requires_approval);
        assert_eq!(
            read.tasks[0].effects,
            vec![Effect::Read {
                path: "scratch/notes.txt".into()
            }]
        );

        let append = compile_rules("Append file log.txt with contents hi").unwrap();
        assert_eq!(append.tasks[0].capabilities, vec![APPEND_CAP]);
        assert_eq!(append.risk_score, 0.4);
        assert!(append.requires_approval);

        let list = compile_rules("List files under notes_dir").unwrap();
        assert_eq!(list.tasks[0].capabilities, vec![LIST_CAP]);
        assert!(!list.requires_approval);
    }

    #[test]
    fn write_intent_requires_approval_by_risk() {
        let ir = compile_rules("Write file notes.txt with contents hi").unwrap();
        assert_eq!(ir.risk_score, 0.5);
        assert!(ir.requires_approval);
    }

    #[test]
    fn propose_maps_new_verbs() {
        let (c, src) = local_propose("read notes.txt").unwrap();
        assert_eq!((c.as_str(), src), ("Read file notes.txt", "local"));
        let (c, src) = local_propose("append to log.txt with more").unwrap();
        assert_eq!(
            (c.as_str(), src),
            ("Append file log.txt with contents more", "local")
        );
        let (c, src) = local_propose("list files under notes_dir").unwrap();
        assert_eq!((c.as_str(), src), ("list files under notes_dir", "rules"));
        let (c, src) = local_propose("ls notes_dir").unwrap();
        assert_eq!((c.as_str(), src), ("List files under notes_dir", "local"));
    }

    #[test]
    fn sensitive_paths_detected() {
        assert!(is_sensitive_path("scratch/.env"));
        assert!(is_sensitive_path("scratch/server.key"));
        assert!(is_sensitive_path("scratch/my-secrets.txt"));
        assert!(is_sensitive_path("scratch/SECRETS"));
        assert!(is_sensitive_path("scratch/.env.local"));
        assert!(!is_sensitive_path("scratch/notes.txt"));
        assert!(!is_sensitive_path("scratch/environment"));
    }

    #[test]
    fn dry_run_denies_sensitive_read() {
        let ir = compile_rules("Read file .env").unwrap();
        let sim = dry_run(&ir);
        assert_eq!(sim.success_probability, 0.0);
        assert_eq!(sim.risk, 1.0);
        assert!(
            matches!(
                &sim.effects_preview[0],
                EffectPreview::Denied { capability, path, reason }
                    if capability == READ_CAP && path == "scratch/.env" && reason == SENSITIVE_DENIED_REASON
            ),
            "expected sensitive denial, got {:?}",
            sim.effects_preview
        );
    }

    #[test]
    fn fetch_url_validation() {
        assert_eq!(
            fetch_url_host("https://api.github.com/repos/x").unwrap(),
            "api.github.com"
        );
        assert_eq!(
            fetch_url_host("https://Api.GitHub.com./x").unwrap(),
            "api.github.com"
        );
        assert_eq!(
            fetch_url_host("http://api.github.com"),
            Err(FETCH_HTTPS_REQUIRED.into())
        );
        assert_eq!(
            fetch_url_host("https://api.github.com:8443"),
            Err(FETCH_PORT_DENIED.into())
        );
        assert_eq!(
            fetch_url_host("https://127.0.0.1/x"),
            Err(FETCH_IP_DENIED.into())
        );
        assert_eq!(
            fetch_url_host("https://localhost/x"),
            Err(FETCH_LOCAL_DENIED.into())
        );
        assert_eq!(
            fetch_url_host("https://db.internal/x"),
            Err(FETCH_LOCAL_DENIED.into())
        );
        assert_eq!(
            fetch_url_host("ftp://x.com"),
            Err(FETCH_HTTPS_REQUIRED.into())
        );
        assert_eq!(
            fetch_url_host("https://user@x.com"),
            Err(FETCH_URL_INVALID.into())
        );
        assert_eq!(
            fetch_url_host("https://nodot"),
            Err(FETCH_URL_INVALID.into())
        );
    }

    #[test]
    fn fetch_compiles_high_risk() {
        let ir = compile_rules("Fetch https://api.github.com/repos/x").unwrap();
        assert_eq!(ir.tasks[0].capabilities, vec![HTTP_CAP]);
        assert_eq!(ir.risk_score, 0.6);
        assert!(ir.requires_approval);
        assert_eq!(
            ir.tasks[0].effects,
            vec![Effect::Fetch {
                url: "https://api.github.com/repos/x".into()
            }]
        );
    }

    #[test]
    fn fetch_preview_default_deny_and_allow() {
        let ir = compile_rules("Fetch https://api.github.com/repos/x").unwrap();
        // No policy: default-deny.
        let sim = dry_run(&ir);
        assert!(matches!(
            &sim.effects_preview[0],
            EffectPreview::Denied { reason, .. } if reason == FETCH_NO_ALLOW_RULE
        ));
        // Explicit allow rule: permitted preview.
        let (policy, _) =
            crate::policy::imp::parse_policy_lenient("allow fetch to api.github.com\n");
        let sim = dry_run_with_policy(&ir, Some(&policy));
        assert!(matches!(
            &sim.effects_preview[0],
            EffectPreview::Fetch { url } if url == "https://api.github.com/repos/x"
        ));
        // Wildcard allow + glob deny interplay: deny wins (first match).
        let (policy_deny, _) = crate::policy::imp::parse_policy_lenient(
            "deny fetch matching *.github.com\nallow fetch to api.github.com\n",
        );
        let sim = dry_run_with_policy(&ir, Some(&policy_deny));
        assert!(matches!(
            &sim.effects_preview[0],
            EffectPreview::Denied { reason, .. } if reason.contains("policy: deny fetch matching *.github.com")
        ));
    }

    #[test]
    fn net_permissions_layering() {
        // The '*' ceiling covers any declared host; initial grants are closed.
        let ceiling = grant_for_capability(HTTP_CAP);
        assert_eq!(ceiling.net, vec!["*"]);
        assert!(initial_grants()[HTTP_CAP].net.is_empty());
        let declared = Permissions {
            fs: vec![],
            net: vec!["api.github.com".into()],
            env: vec![],
        };
        assert!(is_permission_subset(&declared, &ceiling));
        assert!(!is_permission_subset(
            &Permissions {
                fs: vec![],
                net: vec!["evil.com".into()],
                env: vec![],
            },
            &declared
        ));
        // Net floor: non-http capabilities can never carry net permissions.
        let token = issue_token(
            ECHO_CAP,
            "cli",
            "i",
            Permissions {
                fs: vec![],
                net: vec!["api.github.com".into()],
                env: vec![],
            },
            0,
            1_000,
            "s",
        );
        assert_eq!(
            verify_token(
                &token,
                &VerifyContext {
                    secret: "s",
                    now_ms: 0,
                    seen_nonces: &HashSet::new(),
                    revoked_ids: &HashSet::new(),
                    grant: &grant_for_capability(ECHO_CAP),
                    expected_subject: None,
                    expected_intent: None,
                }
            ),
            VerifyReason::NetworkDenied
        );
    }

    #[test]
    fn allow_cloud_routing_is_compile_time_false() {
        const { assert!(!ALLOW_CLOUD_ROUTING) }
    }

    // --- Phase 12: Monte Carlo ---

    fn write_ir_for_mc() -> IntentIR {
        compile_rules("Write file notes.txt with contents hi").unwrap()
    }

    #[test]
    fn monte_carlo_is_deterministic_per_seed() {
        let ir = write_ir_for_mc();
        let mut stats = WorldStats::new();
        stats.insert(
            WRITE_CAP.into(),
            CapabilityStat {
                samples: 20,
                successes: 19,
                durations_ms: vec![10, 12, 14, 16, 18, 20],
            },
        );
        let a = dry_run_monte_carlo(&ir, None, Some(&stats), 500, 42);
        let b = dry_run_monte_carlo(&ir, None, Some(&stats), 500, 42);
        assert_eq!(a, b);
        assert!(a.probabilistic);
        assert_eq!(a.mc_successes, b.mc_successes);
        assert_eq!(a.p50_ms, b.p50_ms);
        // Different seed → (almost surely) different draws.
        let c = dry_run_monte_carlo(&ir, None, Some(&stats), 500, 43);
        assert_ne!(a.seed, c.seed);
    }

    #[test]
    fn monte_carlo_denial_passes_through() {
        let mut ir = write_ir_for_mc();
        ir.tasks[0].capabilities = vec![WRITE_CAP.into()];
        ir.tasks[0].effects = vec![Effect::Write {
            path: "../secret".into(),
            size_bytes: Some(1),
        }];
        let sim = dry_run_monte_carlo(&ir, None, None, 100, 7);
        assert!(!sim.probabilistic);
        assert_eq!(sim.success_probability, 0.0);
        assert_eq!(sim.mc_successes, 0);
    }

    #[test]
    fn monte_carlo_retries_lift_success() {
        let ir = write_ir_for_mc();
        // Fresh capability: Laplace prior = 1/2 per attempt → 1-(1/2)^3 = 0.875.
        let no_stats = dry_run_monte_carlo(&ir, None, None, 2000, 9);
        assert!(
            (no_stats.p_success - 0.875).abs() < 0.03,
            "p={}",
            no_stats.p_success
        );
        // Perfect history: p=11/12 per attempt → ≈0.999 with 3 attempts.
        let mut stats = WorldStats::new();
        stats.insert(
            WRITE_CAP.into(),
            CapabilityStat {
                samples: 10,
                successes: 10,
                durations_ms: vec![],
            },
        );
        let good = dry_run_monte_carlo(&ir, None, Some(&stats), 2000, 9);
        assert!(good.p_success > 0.98, "p={}", good.p_success);
        assert!(good.p_success > no_stats.p_success);
    }

    #[test]
    fn monte_carlo_factors_and_percentiles() {
        let ir = write_ir_for_mc();
        let mut stats = WorldStats::new();
        stats.insert(
            WRITE_CAP.into(),
            CapabilityStat {
                samples: 9,
                successes: 9,
                durations_ms: vec![10, 20, 30, 40, 50, 60, 70, 80, 90],
            },
        );
        let sim = dry_run_monte_carlo(&ir, None, Some(&stats), 1000, 5);
        assert_eq!(sim.factors, vec!["P(cap.write) = 0.9091 (n=9)".to_string()]);
        assert!(sim.p50_ms >= 40 && sim.p50_ms <= 60, "p50={}", sim.p50_ms);
        assert!(sim.p95_ms >= 80, "p95={}", sim.p95_ms);
        // Small history → prior note.
        let mut small = WorldStats::new();
        small.insert(
            WRITE_CAP.into(),
            CapabilityStat {
                samples: 2,
                successes: 2,
                durations_ms: vec![],
            },
        );
        let sim = dry_run_monte_carlo(&ir, None, Some(&small), 100, 5);
        assert_eq!(
            sim.factors,
            vec!["P(cap.write) = 0.7500 (prior, n=2)".to_string()]
        );
    }

    #[test]
    fn mc_gate_is_integer_exact() {
        assert!(mc_gate(900, 1000));
        assert!(mc_gate(950, 1000));
        assert!(!mc_gate(899, 1000));
        assert!(!mc_gate(0, 1000));
        assert!(!mc_gate(1, 0)); // no trials → never passes
    }

    #[test]
    fn derive_seed_is_stable() {
        let a = derive_seed("00000000-0000-0000-0000-000000000001");
        let b = derive_seed("00000000-0000-0000-0000-000000000001");
        let c = derive_seed("00000000-0000-0000-0000-000000000002");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn shadow_events_do_not_transition_the_fold() {
        let folded = fold_events(&[
            V0Event {
                kind: "IntentCompiled".into(),
                payload: serde_json::json!({}),
                at: None,
            },
            V0Event {
                kind: "IntentSimulated".into(),
                payload: serde_json::json!({}),
                at: None,
            },
            V0Event {
                kind: "IntentApproved".into(),
                payload: serde_json::json!({}),
                at: None,
            },
            V0Event {
                kind: "TaskStarted".into(),
                payload: serde_json::json!({"shadow": true}),
                at: None,
            },
            V0Event {
                kind: "TaskSucceeded".into(),
                payload: serde_json::json!({"shadow": true, "output": "shadow"}),
                at: None,
            },
            V0Event {
                kind: "ShadowVerified".into(),
                payload: serde_json::json!({"match": true}),
                at: None,
            },
        ]);
        assert_eq!(folded.status, Some(IntentStatus::Approved));
        assert_eq!(folded.output, None);
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
        let seen = HashSet::new();
        let revoked = HashSet::new();
        let grant = Permissions::empty();
        let ctx = |now_ms: i64| VerifyContext {
            secret,
            now_ms,
            seen_nonces: &seen,
            revoked_ids: &revoked,
            grant: &grant,
            expected_subject: None,
            expected_intent: None,
        };
        assert_eq!(verify_token(&token, &ctx(now)), VerifyReason::Ok);
        let mut bad = token.clone();
        bad.signature.clear();
        assert_eq!(verify_token(&bad, &ctx(now)), VerifyReason::EmptySignature);
        assert_eq!(
            verify_token(&token, &ctx(now + 120_000)),
            VerifyReason::Expired
        );
        let mut net = token.clone();
        net.permissions.net.push("evil".into());
        net.signature = sign_token(&net, secret);
        assert_eq!(verify_token(&net, &ctx(now)), VerifyReason::NetworkDenied);
    }

    #[test]
    fn fold_happy_path() {
        let folded = fold_events(&[
            V0Event {
                kind: "IntentCompiled".into(),
                payload: serde_json::json!({}),
                at: None,
            },
            V0Event {
                kind: "IntentSimulated".into(),
                payload: serde_json::json!({}),
                at: None,
            },
            V0Event {
                kind: "IntentApproved".into(),
                payload: serde_json::json!({}),
                at: None,
            },
            V0Event {
                kind: "TaskSucceeded".into(),
                payload: serde_json::json!({"output": "ping"}),
                at: None,
            },
        ]);
        assert_eq!(folded.status, Some(IntentStatus::Executed));
        assert_eq!(folded.output.as_deref(), Some("ping"));
    }
}
