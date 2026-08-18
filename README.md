# IMPERIUM

> **The Self-Synthesizing Intent Runtime**
>
> Local. Sovereign. Absolute.

IMPERIUM is an operating system for intent. You express what you want in natural language. IMPERIUM compiles it to an executable specification, simulates thousands of outcomes, shows you the future before it happens, and executes with full auditability — all locally, air-gapped, and sovereign.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            IMPERIUM RUNTIME                                  │
├──────────────────┬──────────────────┬──────────────────┬────────────────────┤
│  INTENT          │  CAPABILITY      │  SIMULATION      │  EVOLUTION         │
│  COMPILER        │  SYNTHESIZER     │  ENGINE          │  LOOP              │
├──────────────────┼──────────────────┼──────────────────┼────────────────────┤
│  • NL → IR       │  • Registry      │  • Causal World  │  • Friction        │
│  • Goal          │    lookup        │    Model         │    Detection       │
│  Decomposition   │  • API           │  • Monte Carlo   │  • Autonomous      │
│  • Dependency    │    Discovery     │    Rollouts      │    Patch Gen       │
│  Graph           │  • Code Gen      │  • Counterfactual│  • Shadow Deploy   │
│  • Policy Check  │  • Sandbox       │    Queries       │  • A/B Validation  │
│  • Risk Score    │  • Verification  │  • Diff Preview  │  • Auto-Promote    │
└──────────────────┴──────────────────┴──────────────────┴────────────────────┘
```

## Quick Start

### Prerequisites

- [Nix](https://nixos.org/download.html) with flakes enabled
- Or manually: Rust 1.78+, Python 3.12+, Node 20+, pnpm 9+

### Development

```bash
# Clone and enter
git clone https://github.com/yourorg/imperium
cd imperium

# Enter hermetic dev shell (all tools provided)
nix develop

# Or use just directly
just check          # fmt + lint + test (CI gate)
just build-all      # Build all workspaces
just test-all       # Run all tests
just run-daemon     # Start daemon
just run-cli        # Run CLI
```

### First Run

```bash
# Initialize (downloads models, sets up vault, calibrates voice)
imperium init

# Try an intent
imperium intent new "Create a REST API for user management with PostgreSQL" --interactive

# Simulate before executing
imperium intent simulate <intent-id> --rollouts=10000

# Execute with approval
imperium intent approve <intent-id>
imperium intent execute <intent-id>
```

## Workspaces

| Workspace | Language | Purpose |
|-----------|----------|---------|
| `crates/imperium-core` | Rust | Core types: Intent IR, Events, Capabilities, Policy, Crypto |
| `crates/imperium-runtime` | Rust | WASM host, capability manager, sandbox |
| `crates/imperium-store` | Rust | SQLite event store, projections, snapshots |
| `crates/imperium-sync` | Rust | libp2p/WebRTC sync, CRDT integration |
| `crates/imperium-policy` | Rust | OPA/Rego embedding, policy evaluation |
| `crates/imperium-cli` | Rust | CLI binary |
| `crates/imperium-daemon` | Rust | Background daemon (gRPC + HTTP) |
| `crates/imperium-voice` | Rust | Porcupine/Kokoro WASM bridge |
| `crates/imperium-crypto` | Rust | Signing, encryption, key management |
| `crates/imperium-ffi` | Rust | C/FFI interface for Python/Node |
| `python/imperium_intent` | Python | Intent compiler (NL → IR) |
| `python/imperium_simulation` | Python | Monte Carlo, causal world model |
| `python/imperium_synthesis` | Python | Capability synthesizer (OpenAPI → WASM) |
| `python/imperium_evolution` | Python | Friction detection, patch gen, shadow deploy |
| `python/imperium_api` | Python | FastAPI server, OpenAPI spec |
| `frontend/workbench` | TypeScript | React workbench (Intent Composer, Simulation Preview, World Model) |

## Key Concepts

### Intent IR
The executable intermediate representation. Compiled from natural language, validated, versioned, signed.

```json
{
  "id": "01HXK3JQ9V...",
  "name": "Migrate auth to passkeys",
  "goal": { "description": "Migrate auth to passkeys, zero downtime" },
  "constraints": [{ "kind": "ZeroDowntime", "severity": "Hard" }],
  "success_criteria": [{ "metric": "TestPassRate", "threshold": { "operator": "GreaterThanOrEqual", "value": 0.99 } }],
  "tasks": [...],
  "risk_score": 0.7,
  "requires_approval": true
}
```

### Capability
A WASM component with declared permissions. Synthesized on-demand from API specs.

```json
{
  "name": "github-integration",
  "capabilities": {
    "network": ["api.github.com:443"],
    "vault": ["read:notes", "write:notes:project-*"],
    "shell": ["git", "gh"],
    "secrets": ["GITHUB_TOKEN"]
  }
}
```

### Simulation
Monte Carlo rollouts on a causal world model. Answers "what if" before you commit.

```
✅ Success: 94.2%
⚠️  Risk: 5.8% chance of 5-min DB lock
💰 Cost: $147/mo
⏱️  Timeline: 4.2 hrs median
🔄 Rollback: <30s (tested in 99.8% of sims)
```

### Evolution Loop
The system improves itself while you sleep. Detects friction → generates patches → shadow deploys → validates → promotes.

## Security

- **Air-gapped by default**: `ALLOW_CLOUD_ROUTING=false` enforced in code
- **Capability-based security**: WASM components with unforgeable tokens
- **Supply chain**: Sigstore signing, Rekor transparency log, SLSA Level 3 builds
- **Attestation**: TPM-backed measured boot, IMA/EVM runtime integrity
- **Privacy**: Zero-trust PII redaction proxy, local-first everything

## License

Business Source License 1.1 (BSL-1.1) — converts to Apache-2.0 after 4 years.

See [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

**IMPERIUM** — Intent. Made Law.