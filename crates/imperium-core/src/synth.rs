//! Capability synthesis — Phase 11 (partial spec 02).
//!
//! Compiles an OpenAPI 3 document into a `CapabilityManifest`: the declared
//! network hosts, the operation surface, and generated WIT for the
//! capability interface. The manifest is a *record*, not a grant — it never
//! enables anything by itself. Execution stays gated by policy (default-deny
//! `fetch`) + grants + explicit approval.
//!
//! Codegen is deterministic: the same spec + name always produces the same
//! manifest bytes. Property tests (seeded, no network) pin the invariants.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthOperation {
    pub method: String,
    pub path: String,
    pub operation_id: String,
}

impl CapabilityManifest {
    /// Human-facing operation summary, e.g. `3 GET` or `2 GET, 1 POST`.
    pub fn ops_summary(&self) -> String {
        let mut by_method: BTreeMap<&str, u64> = BTreeMap::new();
        for op in &self.operations {
            *by_method.entry(op.method.as_str()).or_insert(0) += 1;
        }
        by_method
            .iter()
            .map(|(m, n)| format!("{n} {m}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub name: String,
    pub hosts: Vec<String>,
    pub operations: Vec<SynthOperation>,
    pub wit: String,
    #[serde(default)]
    pub approved: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
    /// BLAKE3 of the source spec text.
    pub source_hash: String,
}

/// Extract a lowercase hostname from an https server URL; invalid entries
/// are skipped (a spec with no valid https servers cannot be synthesized).
fn server_host(server_url: &str) -> Option<String> {
    let lower = server_url.trim().to_ascii_lowercase();
    let rest = lower.strip_prefix("https://")?;
    let host: &str = rest.split(['/', ':', '?', '#']).next()?;
    if host.is_empty() || !host.contains('.') {
        return None;
    }
    if host == "localhost" || host.ends_with(".local") || host.ends_with(".internal") {
        return None;
    }
    Some(host.to_string())
}

const METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];

fn sanitize_op_id(method: &str, path: &str) -> String {
    let cleaned: String = path
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    format!("{method}_{trimmed}")
}

fn generate_wit(name: &str, ops: &[SynthOperation]) -> String {
    let mut wit = String::new();
    wit.push_str("package imperium:cap;\n\n");
    wit.push_str(&format!("interface {name} {{\n"));
    wit.push_str("    // GET-only in v0: responses are text bodies.\n");
    for op in ops {
        wit.push_str(&format!(
            "    // {} {}\n    {}-{}: func() -> string;\n",
            op.method.to_uppercase(),
            op.path,
            op.method,
            op.operation_id
                .trim_start_matches(&format!("{}_", op.method))
        ));
    }
    wit.push_str("}\n");
    wit
}

/// Compile an OpenAPI 3 JSON document into a capability manifest.
pub fn synthesize(name: &str, openapi_json: &str) -> Result<CapabilityManifest, String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err("capability name must be kebab/snake case lowercase".into());
    }
    let spec: serde_json::Value =
        serde_json::from_str(openapi_json).map_err(|e| format!("spec is not valid JSON: {e}"))?;
    if spec.get("openapi").is_none() && spec.get("swagger").is_none() {
        return Err("spec is not an OpenAPI/Swagger document".into());
    }

    let mut hosts: Vec<String> = vec![];
    if let Some(servers) = spec.get("servers").and_then(|s| s.as_array()) {
        for server in servers {
            if let Some(url) = server.get("url").and_then(|u| u.as_str()) {
                if let Some(host) = server_host(url) {
                    if !hosts.contains(&host) {
                        hosts.push(host);
                    }
                }
            }
        }
    }
    if hosts.is_empty() {
        return Err("spec has no usable https server host".into());
    }

    let mut operations: Vec<SynthOperation> = vec![];
    if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
        for (path, item) in paths {
            if !path.starts_with('/') {
                return Err(format!("operation path must start with '/': {path}"));
            }
            for method in METHODS {
                if let Some(op) = item.get(method) {
                    let operation_id = op
                        .get("operationId")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                        .unwrap_or_else(|| sanitize_op_id(method, path));
                    operations.push(SynthOperation {
                        method: method.to_uppercase(),
                        path: path.to_string(),
                        operation_id,
                    });
                }
            }
        }
    }
    // v0 synthesizes a GET surface; every non-GET op is recorded as callable
    // metadata but the manifest only enables GET through cap.http.
    operations.sort_by(|a, b| a.operation_id.cmp(&b.operation_id));

    let source_hash = {
        let mut hasher = blake3::Hasher::new();
        hasher.update(openapi_json.as_bytes());
        let bytes = hasher.finalize();
        hex::encode(&bytes.as_bytes()[..16])
    };

    let manifest = CapabilityManifest {
        name: name.to_string(),
        hosts,
        operations,
        wit: String::new(),
        approved: false,
        approved_at: None,
        source_hash,
    };
    let wit = generate_wit(&manifest.name, &manifest.operations);
    Ok(CapabilityManifest { wit, ..manifest })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic LCG for reproducible property tests (no deps, no net).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 33
        }
        fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
            &items[(self.next() % items.len() as u64) as usize]
        }
    }

    fn sample_spec() -> String {
        serde_json::json!({
            "openapi": "3.0.0",
            "servers": [{"url": "https://api.github.com"}],
            "paths": {
                "/repos/{owner}/{repo}": {"get": {"operationId": "get_repo"}},
                "/repos/{owner}/{repo}/issues": {"get": {"operationId": "list_issues"},
                                                  "post": {"operationId": "create_issue"}}
            }
        })
        .to_string()
    }

    #[test]
    fn synthesizes_manifest_with_wit() {
        let m = synthesize("github_rest", &sample_spec()).unwrap();
        assert_eq!(m.hosts, vec!["api.github.com"]);
        assert_eq!(m.operations.len(), 3);
        assert!(m
            .operations
            .iter()
            .all(|o| o.method == "GET" || o.method == "POST"));
        assert!(!m.approved);
        assert!(m.wit.starts_with("package imperium:cap;"));
        assert!(m.wit.contains("interface github_rest"));
        assert!(m.wit.contains("get_repo"));
        // Deterministic: same input → same manifest (wit + hash).
        let m2 = synthesize("github_rest", &sample_spec()).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn rejects_invalid_specs() {
        assert!(synthesize("x", "not json").is_err());
        assert!(synthesize("x", "{}").is_err());
        assert!(synthesize("x", r#"{"openapi": "3.0.0"}"#).is_err());
        assert!(synthesize("Bad Name", &sample_spec()).is_err());
        let http_only = serde_json::json!({
            "openapi": "3.0.0",
            "servers": [{"url": "http://api.github.com"}],
            "paths": {"/x": {"get": {}}}
        })
        .to_string();
        assert!(synthesize("x", &http_only).is_err());
    }

    #[test]
    fn property_generated_specs_satisfy_invariants() {
        let mut rng = Lcg(0x1BADB002);
        let hostnames = ["api.a.com", "api.b.org", "api.c.dev"];
        let paths = ["/x", "/y/{id}", "/z/{id}/sub"];
        let methods = ["get", "post", "put", "delete"];
        for _case in 0..25 {
            let host = rng.pick(&hostnames);
            let mut paths_obj = serde_json::Map::new();
            let n_paths = 1 + (rng.next() % 4) as usize;
            let mut expected_ops: Vec<String> = vec![];
            for i in 0..n_paths {
                let path = rng.pick(&paths);
                let mut path_item = serde_json::Map::new();
                let n_methods = 1 + (rng.next() % 3) as usize;
                for j in 0..n_methods {
                    let method = rng.pick(&methods);
                    path_item.insert(
                        method.to_string(),
                        serde_json::json!({"operationId": format!("op{i}_{j}")}),
                    );
                    expected_ops.push(format!("op{i}_{j}"));
                }
                paths_obj.insert(path.to_string(), serde_json::Value::Object(path_item));
            }
            let spec = serde_json::json!({
                "openapi": "3.0.0",
                "servers": [{"url": format!("https://{host}")}],
                "paths": paths_obj,
            })
            .to_string();
            let name = format!("prop_{:04}", rng.next() % 10_000);
            let m = synthesize(&name, &spec).expect("generated spec synthesizes");
            // Invariant: hosts come only from the spec's https servers.
            assert_eq!(m.hosts, vec![*host]);
            // Invariant: every recorded operation is from the spec.
            for op in &m.operations {
                assert!(
                    expected_ops.contains(&op.operation_id),
                    "unexpected op {op:?}"
                );
                assert!(op.path.starts_with('/'));
                assert!(METHODS.contains(&op.method.to_lowercase().as_str()));
            }
            // Invariant: WIT names the interface and every operation id.
            assert!(m.wit.contains(&format!("interface {name}")));
            for op in &m.operations {
                assert!(m.wit.contains(&op.operation_id));
            }
            // Invariant: manifest round-trips through JSON.
            let round: CapabilityManifest =
                serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
            assert_eq!(round, m);
        }
    }
}
