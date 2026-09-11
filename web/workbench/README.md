# IMPERIUM browser workbench

Zero-install static UI for the v0 intent loop.

```bash
cd web/workbench
python3 -m http.server 8080
# open http://127.0.0.1:8080
```

Flow: **compile → propose → simulate → approve → execute → replay**

- HMAC capability tokens (Web Crypto)
- WASM guest for `cap.echo` / `cap.write`
- Scratch files in memory for the session
- Event log fold must match the intent snapshot

Kernel source of truth remains `web/v0` and the Rust CLI (`just v0`).
