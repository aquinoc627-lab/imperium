# IMPERIUM v0 (working slice)

This directory is the **only implemented product**. Rust crates and `frontend/workbench` remain scaffolding.

## What works

```
NL → rules (or local propose) → IR → simulate → HMAC token → WASM guest → event fold
```

| Capability | Effect | Rights |
|------------|--------|--------|
| `cap.echo` | return the text | no FS, no net |
| `cap.write` | write under `scratch/` | FS prefix `scratch/` only |

Execute without a verified token is denied. Network is deny-all. `../` and absolute paths fail at compile.

A model may **propose** echo/write. The rules compiler still **decides**.

## Test (no install)

Node 22+:

```bash
cd web/v0
npm test
```

## Canonical forms

```
Echo this message: <text>
Write file <path> with contents <text>
```

Loose forms (`say hello`, `save notes.txt with hi`) go through `localPropose` first.

## What this is not

- Not the Rust WASM/WASI host
- Not air-gap / SLSA / TPM
- Not the mock `frontend/workbench` pages
