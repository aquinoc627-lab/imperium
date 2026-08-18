# IMPERIUM Architecture Specification

## Overview

IMPERIUM is a self-synthesizing intent runtime. It takes natural language intent, compiles it to an executable Intermediate Representation (IR), simulates outcomes using a causal world model, and executes with full auditability — all locally and air-gapped.

## Core Principles

1. **Local-First**: No external dependencies. Runs entirely on user hardware.
2. **Air-Gap Enforced**: `ALLOW_CLOUD_ROUTING=false` in code, not config.
3. **Intent-Driven**: NL → IR → Simulation → Execution. No manual scripting.
4. **Self-Synthesizing**: Generates capabilities (WASM components) on-demand from API specs.
5. **Evolutionary**: Detects friction, generates patches, shadow deploys, validates, promotes.
6. **Sovereign**: Capability-based security, supply chain attestation, TPM-backed boot.

## System Architecture

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

## Component Responsibilities

### Intent Compiler (Python)
- **Input**: Natural language + context (codebase, infra, team, history)
- **Process**: LLM-based compilation → IR validation → risk scoring
- **Output**: `IntentIR` (executable, versioned, signed)

### Capability Synthesizer (Python + Rust)
- **Input**: Capability name + intent requirements
- **Process**: 
  1. Registry lookup
  2. API discovery (OpenAPI, GraphQL, gRPC reflection)
  3. Code generation (WASM component + WIT interface)
  4. Sandbox testing (property-based, fuzzing)
  5. Registration with manifest
- **Output**: `CapabilityManifest` + WASM component

### Simulation Engine (Python)
- **Input**: `IntentIR` + World Model snapshot
- **Process**:
  1. Initialize causal world model
  2. Run Monte Carlo rollouts (10k+ parallel)
  3. Inject chaos (latency, failures, load spikes)
  4. Compute success probability, cost, timeline, risk factors
- **Output**: `SimulationResult` with interactive preview

### Evolution Loop (Python)
- **Input**: Execution traces, friction patterns, user feedback
- **Process**:
  1. Detect friction (repeated manual steps, errors, latency)
  2. Generate patches (code + tests + docs + rollback)
  3. Shadow deploy alongside production
  4. A/B validate with statistical significance
  5. Auto-promote or discard
- **Output**: Improved capabilities, updated world model

### World Model (Rust + Python)
- **Representation**: Causal graph (code, infra, data, team, traffic)
- **Updates**: Event-driven from execution traces
- **Queries**: Counterfactual ("what if traffic 3x?"), path analysis
- **Storage**: SQLite + projections + CRDT sync

### Runtime (Rust)
- **WASM Host**: wasmtime with component model
- **Capability Manager**: Token issuance, enforcement, audit
- **Policy Engine**: Embedded OPA/Rego
- **Event Store**: Append-only, encrypted, projected
- **Sync**: libp2p + WebRTC, CRDT-based

## Data Flow

```
User Input (NL/Voice)
       │
       ▼
┌──────────────────┐
│ Intent Compiler  │──▶ IntentIR (validated, signed)
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Policy Check     │──▶ Allow/Deny + Reason
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Simulation       │──▶ SimulationResult (probabilistic)
└──────────────────┘
       │
       ▼
   [User Approval] ◀── Interactive Preview (what-if, diff, timeline)
       │
       ▼
┌──────────────────┐
│ Execution        │──▶ Event Stream (durable, replayable)
│ (DAG scheduler)  │
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Evolution Loop   │──▶ Patches → Shadow → Promote
└──────────────────┘
```

## Security Model

### Capability-Based Security
- Every capability = WASM component + manifest
- Manifest declares: network, vault, shell, secrets, filesystem, resources
- Runtime issues unforgeable `CapabilityToken` per invocation
- Host functions enforce permissions at WASM boundary

### Supply Chain
- Reproducible builds (Nix flakes)
- Sigstore keyless signing (cosign + fulcio)
- Rekor transparency log
- SLSA Level 3 provenance
- SBOM (SPDX/CycloneDX) per build

### Runtime Integrity
- TPM 2.0 measured boot
- IMA/EVM file integrity
- Remote attestation for plugins/models
- Encrypted event store at rest

### Privacy
- Zero-trust PII redaction proxy
- All cloud calls opt-in, explicit approval
- Local embeddings, local inference
- No telemetry without consent

## Deployment Targets

| Target | Description |
|--------|-------------|
| **Laptop** | 16GB RAM, CPU-only, full suite |
| **Workstation** | 64GB+, GPU, model zoo |
| **Server** | Headless, multi-user, team sync |
| **Edge** | ARM64, limited resources, air-gapped |
| **Container** | Distroless, non-root, signed |

## Interfaces

| Interface | Protocol | Purpose |
|-----------|----------|---------|
| CLI | JSON/Postcard | User interaction |
| Daemon API | gRPC + HTTP/REST | Programmatic control |
| Frontend | WebSocket + SSE | Real-time UI |
| Sync | libp2p/WebRTC | P2P state sync |
| Plugins | WASM Component Model | Capability execution |
| Models | Ollama-compatible | Local inference |

## Observability

- **Traces**: W3C TraceContext (intent → simulate → execute)
- **Metrics**: Prometheus format (success rate, latency, cost, resources)
- **Logs**: Structured JSON, correlated with traces
- **Profiles**: Continuous (py-spy, cargo-profiler)
- **Dashboards**: Embedded Grafana, exportable

## Evolution

The system evolves via:
1. **Friction Detection**: ML on execution traces + user behavior
2. **Patch Generation**: LLM + symbolic reasoning
3. **Shadow Deployment**: Production traffic mirroring
4. **Statistical Validation**: Bayesian A/B testing
5. **Auto-Promotion**: Policy-gated, human-in-loop for high-risk

---

*This document is a living specification. Update with each architectural decision.*