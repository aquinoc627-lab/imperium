# Capability Synthesis Specification

## Overview

Capabilities are the atomic units of execution in IMPERIUM. Each capability is a WebAssembly component with a declared manifest specifying its permissions. Capabilities are synthesized on-demand from API specifications.

## Capability Lifecycle

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CAPABILITY SYNTHESIS FLOW                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  NEED: "Query Jira for blocked tickets"                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ 1. REGISTRY LOOKUP → No Jira capability registered                  │    │
│  │ 2. API DISCOVERY → Fetch OpenAPI spec from atlassian.com            │    │
│  │ 3. CODE GENERATION → Generate WASM component + WIT interface        │    │
│  │ 4. SANDBOX TEST → Property-based tests against Jira sandbox         │    │
│  │ 5. VERIFICATION → Sign, attest, register                            │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                    │                                         │
│                                    ▼                                         │
│  RESULT: `jira.query(blocked=true)` now exists — forever                   │
│                                                                              │
│  NEXT TIME: Instant registry lookup (cached, versioned, signed)            │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Capability Manifest

```json
{
  "id": "CapabilityId",
  "name": "string",
  "version": "semver",
  "description": "string",
  "author": "string",
  "repository": "string?",
  "license": "string",
  "capabilities": "DeclaredCapabilities",
  "wasm_hash": "Hash",
  "wasm_size": "u64",
  "entry_point": "string",
  "wit": "string (WIT format)",
  "dependencies": ["CapabilityDependency"],
  "min_protocol_version": "u32",
  "tags": ["string"],
  "created_at": "DateTime<Utc>",
  "signature": "string?"
}
```

### DeclaredCapabilities

```json
{
  "network": [
    {
      "host_pattern": "api.github.com",
      "port": 443,
      "protocol": "HTTPS",
      "tls_required": true,
      "description": "GitHub REST API"
    }
  ],
  "vault": [
    {
      "path_pattern": "projects/*/issues/*",
      "permissions": ["Read", "Write"],
      "description": "Project issue notes"
    }
  ],
  "shell": [
    {
      "command": "git",
      "args_pattern": ["commit", "-m", "*"],
      "working_dir": null,
      "timeout_ms": 30000,
      "description": "Git commits"
    }
  ],
  "secrets": [
    {
      "name": "GITHUB_TOKEN",
      "description": "GitHub personal access token",
      "required": true
    }
  ],
  "resources": {
    "max_cpu_ms_per_invocation": 5000,
    "max_memory_bytes": 134217728,
    "max_execution_time_ms": 30000,
    "max_concurrent_invocations": 10,
    "max_network_bytes_per_sec": 10485760,
    "max_filesystem_bytes_per_sec": 52428800
  },
  "filesystem": [
    {
      "path": "/tmp/imperium-*",
      "permissions": ["Read", "Write", "Create"],
      "description": "Temporary workspace"
    }
  ],
  "custom": {}
}
```

## WASM Component Model

### WIT Interface Example

```wit
package imperium:github-integration@1.0.0;

interface github {
    // Query operations
    query-issues: func(input: query-input) -> result<issue-list, error>;
    query-prs: func(input: query-input) -> result<pr-list, error>;
    query-repos: func(input: query-input) -> result<repo-list, error>;

    // Mutation operations
    create-issue: func(input: create-issue-input) -> result<issue, error>;
    update-issue: func(input: update-issue-input) -> result<issue, error>;
    close-issue: func(input: close-issue-input) -> result<(), error>;

    // PR operations
    create-pr: func(input: create-pr-input) -> result<pr, error>;
    merge-pr: func(input: merge-pr-input) -> result<(), error>;
    review-pr: func(input: review-pr-input) -> result<(), error>;
}

// Types
type query-input = record {
    repo: string,
    labels: list<string>,
    state: option<string>,
    assignee: option<string>,
    limit: option<u32>,
};

type issue = record {
    number: u32,
    title: string,
    body: string,
    state: string,
    labels: list<string>,
    assignee: option<string>,
    created-at: string,
    updated-at: string,
    url: string,
};

type issue-list = list<issue>;

type create-issue-input = record {
    repo: string,
    title: string,
    body: string,
    labels: list<string>,
    assignees: list<string>,
};

type error = variant {
    not-found: string,
    unauthorized: string,
    rate-limited: record { retry-after: u64 },
    validation: string,
    internal: string,
};

type result<T, E> = variant { ok: T, err: E };
```

### Generated Rust Component

```rust
use wit_bindgen::generate;

generate!({
    world: "github-integration",
    path: "wit",
});

use crate::exports::imperium::github_integration::github::{Guest, QueryInput, Issue, IssueList, Error, Result};

struct GitHubIntegration;

impl Guest for GitHubIntegration {
    fn query_issues(input: QueryInput) -> Result<IssueList, Error> {
        // HTTP client with capability token
        let client = HttpClient::with_capability_token();
        
        // Build request
        let url = format!("https://api.github.com/repos/{}/issues", input.repo);
        let mut req = client.get(&url);
        
        if let Some(labels) = input.labels {
            req = req.query(&[("labels", labels.join(","))]);
        }
        if let Some(state) = input.state {
            req = req.query(&[("state", state)]);
        }
        
        // Execute with timeout
        let response = req.send().await?;
        
        // Parse and return
        let issues: Vec<Issue> = response.json().await?;
        Ok(issues)
    }
    // ... other methods
}

export!(GitHubIntegration);
```

## Synthesis Pipeline

### 1. Registry Lookup
```python
async def find_capability(name: str) -> Optional[CapabilityManifest]:
    """Check local registry first."""
    return await registry.get(name)
```

### 2. API Discovery
```python
async def discover_api(service: str) -> OpenAPISpec:
    """Fetch and parse API specification."""
    # Known providers
    providers = {
        "github": "https://api.github.com/openapi.json",
        "jira": "https://developer.atlassian.com/cloud/jira/platform/openapi/",
        "slack": "https://api.slack.com/openapi.json",
        # ...
    }
    
    # Custom discovery
    if service in providers:
        return await fetch_openapi(providers[service])
    
    # Generic: try common paths
    for path in ["/openapi.json", "/swagger.json", "/api-docs"]:
        try:
            return await fetch_openapi(f"https://{service}{path}")
        except:
            continue
    
    raise CapabilityNotFound(f"Could not discover API for {service}")
```

### 3. Code Generation
```python
async def generate_capability(spec: OpenAPISpec, requirements: CapabilityRequirements) -> CapabilityArtifact:
    """Generate WASM component from OpenAPI spec."""
    
    # 1. Filter operations by requirements
    ops = filter_operations(spec, requirements)
    
    # 2. Generate WIT interface
    wit = generate_wit(ops, requirements.namespace)
    
    # 3. Generate Rust code
    rust_code = generate_rust(ops, wit)
    
    # 4. Generate Cargo.toml
    cargo_toml = generate_cargo(ops)
    
    # 5. Compile to WASM
    wasm_bytes = await compile_wasm(rust_code, cargo_toml)
    
    # 6. Generate manifest
    manifest = generate_manifest(ops, requirements, wasm_bytes)
    
    return CapabilityArtifact(manifest, wasm_bytes, wit)
```

### 4. Sandbox Testing
```python
async def test_capability(artifact: CapabilityArtifact) -> TestResult:
    """Run property-based tests in sandbox."""
    
    # Start sandbox
    sandbox = await Sandbox.create(
        wasm=artifact.wasm_bytes,
        manifest=artifact.manifest,
        network_allowlist=artifact.manifest.capabilities.network,
    )
    
    try:
        # Property tests
        results = []
        
        # Test: idempotency
        for op in artifact.manifest.capabilities.operations:
            if op.idempotent:
                result = await sandbox.invoke(op.name, op.sample_input)
                result2 = await sandbox.invoke(op.name, op.sample_input)
                assert result == result2, f"{op.name} not idempotent"
                results.append(TestPass("idempotency", op.name))
        
        # Test: rate limit handling
        for _ in range(100):
            await sandbox.invoke("query", {"page": 1})
        # Should handle 429 gracefully
        
        # Test: auth failure
        sandbox.revoke_secret("GITHUB_TOKEN")
        result = await sandbox.invoke("query", {...})
        assert isinstance(result, Error) and result.kind == "unauthorized"
        
        # Test: network isolation
        try:
            await sandbox.invoke("query", {...})  # Should fail if host not in allowlist
        except NetworkDenied:
            pass
        
        return TestResult(passed=True, details=results)
    
    finally:
        await sandbox.destroy()
```

### 5. Verification & Registration
```python
async def register_capability(artifact: CapabilityArtifact) -> CapabilityId:
    """Sign, attest, and register capability."""
    
    # Sign with sigstore
    signature = await cosign_sign(artifact.wasm_bytes)
    
    # Submit to Rekor
    rekor_entry = await rekor_submit(signature, artifact.manifest)
    
    # Verify build provenance
    await verify_slsa_provenance(artifact)
    
    # Register in local registry
    capability_id = await registry.register(
        manifest=artifact.manifest,
        wasm_bytes=artifact.wasm_bytes,
        signature=signature,
        rekor_entry=rekor_entry,
    )
    
    return capability_id
```

## Capability Token (Runtime)

```json
{
  "id": "TokenId",
  "capability_id": "CapabilityId",
  "permissions": "GrantedPermissions",
  "subject": "ActorId",
  "issuer": "ActorId",
  "issued_at": "DateTime<Utc>",
  "expires_at": "DateTime<Utc>",
  "nonce": "[u8; 32]",
  "signature": "Vec<u8>"
}
```

### Token Validation
1. Verify signature (ed25519)
2. Check expiration
3. Verify nonce not reused
4. Check subject matches caller
5. Verify issuer is policy engine
6. Validate permissions ⊆ manifest capabilities

## Host Functions (WASM Imports)

```rust
// Network
fn http_request(method: u32, url_ptr: *const u8, url_len: u32, 
                headers_ptr: *const u8, headers_len: u32,
                body_ptr: *const u8, body_len: u32) -> u32;

// Vault
fn vault_read(path_ptr: *const u8, path_len: u32) -> u32;
fn vault_write(path_ptr: *const u8, path_len: u32, 
               data_ptr: *const u8, data_len: u32) -> u32;
fn vault_delete(path_ptr: *const u8, path_len: u32) -> u32;
fn vault_list(path_ptr: *const u8, path_len: u32) -> u32;
fn vault_search(query_ptr: *const u8, query_len: u32) -> u32;

// Shell
fn shell_exec(cmd_ptr: *const u8, cmd_len: u32,
              args_ptr: *const u8, args_len: u32,
              timeout_ms: u64) -> u32;

// Secrets
fn secret_get(name_ptr: *const u8, name_len: u32) -> u32;

// Resources
fn resource_check(cpu_ms: u64, memory_bytes: u64) -> u32;

// Crypto
fn crypto_sign(key_id: u32, data_ptr: *const u8, data_len: u32) -> u32;
fn crypto_verify(key_id: u32, data_ptr: *const u8, data_len: u32, 
                 sig_ptr: *const u8, sig_len: u32) -> u32;
```

## Capability Marketplace

### Publishing
```bash
imperium capability publish github-integration@1.2.0 \
  --manifest manifest.json \
  --wasm component.wasm \
  --wit github.wit \
  --changelog CHANGELOG.md
```

### Installation
```bash
# One-click install
imperium plugin install github-integration@latest

# With custom config
imperium plugin install github-integration@1.2.0 \
  --config '{"default_repo": "myorg/*"}'
```

### Discovery
```bash
imperium plugin search "github"
imperium plugin info github-integration@1.2.0
```

## Security Considerations

1. **Least Privilege**: Manifest declares minimum required permissions
2. **Token Expiry**: Short-lived tokens (default 5 min)
3. **Audit Logging**: Every capability invocation logged
4. **Sandbox**: WASM + capability-based enforcement
4. **Supply Chain**: Sigstore + Rekor + SLSA
5. **Revocation**: Capability can be quarantined instantly
6. **Resource Limits**: Enforced at host level

---

*This specification defines the capability synthesis pipeline. Implementation details in `python/imperium_synthesis/` and `crates/imperium-runtime/`.*