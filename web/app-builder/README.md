# App Builder workbench route (reference)

This is the fixed TanStack Start home route from the Grok App Builder
session (phases 0–6 UI). It is **not** a standalone app — it depends on
App Builder auth, `@/lib/imperium/actions`, and UI components that lived
in that sandbox.

The executable product remains:

- `web/v0` — TypeScript kernel + tests
- `crates/imperium-cli` — local CLI (`just v0`)

Drop this file in as `src/routes/index.tsx` in an App Builder project that
already has the IMPERIUM server functions and types.
