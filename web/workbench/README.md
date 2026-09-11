# IMPERIUM browser workbench

Zero-install projector for the TypeScript reference kernel in `web/v0`.

```bash
cd web/workbench
python3 -m http.server 8080
# open http://127.0.0.1:8080
```

`app.js` imports `kernel.js`, which is generated from `web/v0/src/browser-api.ts`.
Do not add compiler, token, or WASM logic to `app.js`.

```bash
# regenerate kernel.js after changing web/v0 (requires esbuild)
node scripts/emit-workbench-kernel.mjs
```

## Voice input (Phase 19)

**Hold to talk** uses the browser SpeechRecognition API to fill the sentence
box. You still press Compile or Propose. Voice never executes intents.

Honesty: browser STT may send audio to the browser vendor. This is not
air-gapped speech.

## What this screen runs

- Compile / propose / simulate / approve / execute / replay / revoke
- Verbs the kernel already has: echo, write, read, append, list
- Fetch compiles and dry-run default-denies. The workbench never performs network I/O.
- Session `.imp` policy (lint + evaluate at simulate)
- HMAC tokens via Web Crypto. Secret is random per page load and never stored.
- Scratch is an in-memory `Map` for the tab
- Shadow execute runs the guest without spending the token or advancing status
- Low-risk verbs may auto-approve on execute (`IntentApproved {auto: true}`)

## Not this screen

Ledger SQLite, schedules, keychain bind, MCP stdio, OpenAPI registry files.
Those stay in `imperium-cli`.

Gate for kernel behavior remains `just v0` (`web/v0/src/workbench-bind.test.ts`
locks the exports this UI calls).
