import {
  compileRules,
  localPropose,
  simulateStatic,
  issueToken,
  verifyToken,
  fingerprint,
  grantForCapability,
  foldEvents,
  parsePolicyLenient,
  lintPolicy,
  runGuest,
  rightsFromToken,
  extractEchoText,
  extractWrite,
  HTTP_CAPABILITY,
} from "./kernel.js";

const SUBJECT = "workbench-user";

const intents = new Map();
const scratch = new Map();
const seenNonces = new Set();
const revokedIds = new Set();

const SECRET = (() => {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
})();

function nowIso() {
  return new Date().toISOString();
}

function uid() {
  return crypto.randomUUID();
}

function toShadowPath(path) {
  if (!path.startsWith("scratch/")) return path;
  return `scratch/shadow/${path.slice("scratch/".length)}`;
}

function pushEvent(id, kind, payload = {}) {
  const state = intents.get(id);
  if (!state) return;
  state.events.push({ id: uid(), intent_id: id, kind, payload, created_at: nowIso() });
  state.intent.updated_at = nowIso();
}

function listIntents() {
  return [...intents.values()]
    .map((s) => s.intent)
    .sort((a, b) => b.created_at.localeCompare(a.created_at));
}

function listEvents(id) {
  return intents.get(id)?.events ?? [];
}

function listScratch() {
  return [...scratch.values()].sort((a, b) => a.path.localeCompare(b.path));
}

function compile(nl, proposer) {
  const result = compileRules(nl);
  if (!result.ok) return result;
  const ir = result.ir;
  const intent = {
    id: ir.id,
    name: ir.name,
    nl_source: ir.nl_source,
    status: "compiled",
    ir,
    simulation: null,
    output: null,
    created_at: nowIso(),
    updated_at: nowIso(),
    token: null,
  };
  intents.set(ir.id, { intent, events: [], rawToken: null });
  if (proposer) pushEvent(ir.id, "IntentProposed", { proposer, canonical: nl });
  pushEvent(ir.id, "IntentCompiled", {
    name: ir.name,
    capability: ir.tasks[0]?.capabilities[0] ?? "",
    proposer: proposer ?? null,
  });
  return { ok: true, intent };
}

function propose(nl) {
  const p = localPropose(nl);
  if (!p.ok) return p;
  const c = compile(p.canonical, p.proposer);
  if (!c.ok) return c;
  return { ok: true, intent: c.intent, canonical: p.canonical, proposer: p.proposer };
}

function parsePolicyFromUi() {
  return parsePolicyLenient($("policy").value);
}

function hasDeniedPreview(simulation) {
  return (simulation?.effects_preview ?? []).some((effect) => effect.kind === "denied");
}

function simulate(id) {
  const state = intents.get(id);
  if (!state) return { ok: false, error: "Intent not found." };
  const parsed = parsePolicyFromUi();
  const sim = simulateStatic(state.intent.ir, parsed.policy);
  state.intent.simulation = sim;
  state.intent.status = "simulated";
  pushEvent(id, "IntentSimulated", {
    success_probability: sim.success_probability,
    risk: sim.risk,
    duration_ms: sim.duration_ms,
    notes: sim.notes,
    effects_preview: sim.effects_preview ?? [],
  });
  return { ok: true, intent: state.intent };
}

async function approve(id, options = {}) {
  const state = intents.get(id);
  if (!state) return { ok: false, error: "Intent not found." };
  if (state.intent.status !== "simulated" && state.intent.status !== "approved") {
    return { ok: false, error: "Simulate before Approve." };
  }
  const simulation = state.intent.simulation;
  if (!simulation) return { ok: false, error: "Simulate before Approve." };
  if (hasDeniedPreview(simulation)) {
    return { ok: false, error: "Approve refused: dry-run contains denied effects." };
  }
  const capability = state.intent.ir.tasks[0]?.capabilities[0] ?? "";
  const grant = grantForCapability(capability);
  const token = await issueToken(
    {
      capability,
      subject: SUBJECT,
      intent_id: id,
      permissions: grant,
      expires_at: Date.now() + 15 * 60 * 1000,
    },
    SECRET,
  );
  const checked = await verifyToken(token, {
    secret: SECRET,
    expectedSubject: SUBJECT,
    expectedIntentId: id,
    grant,
    revokedIds,
    seenNonces,
  });
  if (!checked.ok) return { ok: false, error: `Token verify failed: ${checked.reason}` };

  state.rawToken = token;
  state.intent.token = {
    id: token.id,
    capability,
    fingerprint: fingerprint(token.signature),
    expires_at: new Date(token.expires_at).toISOString(),
    revoked: false,
    used: false,
    permissions: token.permissions,
  };
  state.intent.status = "approved";
  const payload = options.auto ? { auto: true, risk: state.intent.ir.risk_score } : {};
  pushEvent(id, "IntentApproved", payload);
  pushEvent(id, "TokenIssued", {
    token_id: token.id,
    fingerprint: state.intent.token.fingerprint,
    capability,
  });
  return { ok: true, intent: state.intent };
}

function memFs() {
  const dirs = new Set();
  const dirOf = (path) => {
    const idx = path.lastIndexOf("/");
    if (idx > 0) dirs.add(path.slice(0, idx));
  };
  return {
    write(path, contents) {
      scratch.set(path, { path, contents, updated_at: nowIso() });
      dirOf(path);
      return contents.length;
    },
    read(path) {
      return scratch.get(path)?.contents ?? null;
    },
    append(path, contents) {
      const current = scratch.get(path)?.contents ?? "";
      scratch.set(path, { path, contents: `${current}${contents}`, updated_at: nowIso() });
      dirOf(path);
      return contents.length;
    },
    list(path) {
      const out = [];
      for (const [p] of scratch) {
        if (p.startsWith(`${path}/`)) out.push(p.slice(path.length + 1));
      }
      for (const d of dirs) {
        if (d.startsWith(`${path}/`)) out.push(`${d.slice(path.length + 1)}/`);
      }
      return out;
    },
  };
}

function guestOp(intent, capability, shadow) {
  const task = intent.ir.tasks[0];
  const write = extractWrite(intent.ir);
  const writePath = write?.path ? (shadow ? toShadowPath(write.path) : write.path) : null;
  if (capability === "cap.echo") {
    return { kind: "echo", text: extractEchoText(intent.ir) };
  }
  if (capability === "cap.write") {
    return writePath && write ? { kind: "write", path: writePath, contents: write.contents } : null;
  }
  if (capability === "cap.read") {
    return task.target_path ? { kind: "read", path: task.target_path } : null;
  }
  if (capability === "cap.append") {
    const path = task.target_path;
    if (!path) return null;
    return { kind: "append", path: shadow ? toShadowPath(path) : path, contents: task.description };
  }
  if (capability === "cap.list") {
    return task.target_path ? { kind: "list", path: task.target_path } : null;
  }
  return null;
}

async function execute(id, shadow = false) {
  const state = intents.get(id);
  if (!state) return { ok: false, error: "Intent not found." };

  if (!state.rawToken || !state.intent.token) {
    if (!state.intent.ir.requires_approval) {
      if (!state.intent.simulation) {
        const sim = simulate(id);
        if (!sim.ok) return sim;
      }
      const approved = await approve(id, { auto: true });
      if (!approved.ok) return approved;
    } else {
      return { ok: false, error: "Approve first to issue a token." };
    }
  }

  if (!state.rawToken || !state.intent.token) {
    return { ok: false, error: "Approve first to issue a token." };
  }
  if (state.intent.token.revoked) return { ok: false, error: "Token revoked." };
  if (!shadow && state.intent.token.used) return { ok: false, error: "Token already spent." };

  if (state.rawToken.capability === HTTP_CAPABILITY) {
    return { ok: false, error: "cap.http execution is disabled in workbench." };
  }

  const grant = grantForCapability(state.rawToken.capability);
  const checked = await verifyToken(state.rawToken, {
    secret: SECRET,
    revokedIds,
    seenNonces: shadow ? undefined : seenNonces,
    expectedSubject: SUBJECT,
    expectedIntentId: id,
    grant,
  });
  if (!checked.ok) {
    if (shadow) {
      pushEvent(id, "TaskFailed", { shadow: true, reason: checked.reason });
      return { ok: false, error: `Token verify failed: ${checked.reason}` };
    }
    state.intent.status = "failed";
    pushEvent(id, "TaskFailed", { reason: checked.reason });
    return { ok: false, error: `Token verify failed: ${checked.reason}` };
  }

  if (!shadow) seenNonces.add(state.rawToken.nonce);
  pushEvent(id, "TaskStarted", { capability: state.rawToken.capability, shadow });

  const op = guestOp(state.intent, state.rawToken.capability, shadow);
  if (!op) {
    const reason = "Unsupported or malformed task for capability.";
    if (shadow) {
      pushEvent(id, "TaskFailed", { shadow: true, reason });
      return { ok: false, error: reason };
    }
    state.intent.status = "failed";
    pushEvent(id, "TaskFailed", { reason });
    return { ok: false, error: reason };
  }

  try {
    const output = await runGuest(
      op,
      rightsFromToken(state.rawToken.capability, state.rawToken.permissions.fs),
      memFs(),
    );
    if (shadow) {
      pushEvent(id, "TaskSucceeded", { shadow: true, output });
      return { ok: true, intent: state.intent };
    }
    state.intent.output = output;
    state.intent.status = "executed";
    state.intent.token = { ...state.intent.token, used: true };
    pushEvent(id, "TaskSucceeded", { output });
    return { ok: true, intent: state.intent };
  } catch (err) {
    const reason = err instanceof Error ? err.message : "execute failed";
    if (shadow) {
      pushEvent(id, "TaskFailed", { shadow: true, reason });
      return { ok: false, error: reason };
    }
    state.intent.status = "failed";
    pushEvent(id, "TaskFailed", { reason });
    return { ok: false, error: reason };
  }
}

function revoke(id) {
  const state = intents.get(id);
  if (!state?.rawToken || !state.intent.token) return { ok: false, error: "No token to revoke." };
  revokedIds.add(state.rawToken.id);
  state.intent.token = { ...state.intent.token, revoked: true };
  pushEvent(id, "TokenRevoked", { token_id: state.rawToken.id });
  return { ok: true, intent: state.intent };
}

function replay(id) {
  const state = intents.get(id);
  if (!state) return { ok: false, error: "Intent not found." };
  const folded = foldEvents(state.events);
  const matches = folded.status === state.intent.status && folded.output === state.intent.output;
  pushEvent(id, "IntentReplayed", { matches, folded_status: folded.status });
  return { ok: true, folded, matches_store: matches, event_count: state.events.length };
}

let selected = null;
let folded = null;

const $ = (id) => document.getElementById(id);

function toast(msg) {
  const t = $("toast");
  t.hidden = false;
  t.textContent = msg;
  setTimeout(() => {
    t.hidden = true;
  }, 2200);
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => {
    if (c === "&") return "&amp;";
    if (c === "<") return "&lt;";
    if (c === ">") return "&gt;";
    if (c === '"') return "&quot;";
    return "&#39;";
  });
}

function renderPolicyLint() {
  const issues = lintPolicy($("policy").value);
  $("policy-lint").textContent =
    issues.length === 0
      ? "policy ok"
      : issues.map((i) => `L${i.line} ${i.severity}: ${i.message}`).join(" · ");
}

function renderMeter() {
  const nodes = [...$("meter").querySelectorAll("i")];
  if (!selected) {
    nodes.forEach((n) => n.classList.remove("on"));
    return;
  }
  const order = ["compiled", "simulated", "approved", "executed"];
  const state = selected.status === "failed" ? "compiled" : selected.status;
  const idx = Math.max(0, order.indexOf(state));
  nodes.forEach((n, i) => n.classList.toggle("on", i <= idx));
}

function renderList() {
  const ul = $("intent-list");
  ul.innerHTML = "";
  for (const item of listIntents()) {
    const li = document.createElement("li");
    const b = document.createElement("button");
    b.className = selected?.id === item.id ? "active" : "";
    b.innerHTML = `<span>${escapeHtml(item.name)}</span><span>${escapeHtml(item.status)}</span>`;
    b.onclick = () => {
      selected = item;
      folded = null;
      render();
    };
    li.appendChild(b);
    ul.appendChild(li);
  }

  const panel = $("scratch-list");
  panel.innerHTML = "";
  const files = listScratch();
  if (files.length === 0) {
    panel.innerHTML = `<p class="empty">Empty. Write file notes.txt with contents hello</p>`;
    return;
  }
  for (const file of files) {
    const div = document.createElement("div");
    div.className = "file";
    div.innerHTML = `<div>${escapeHtml(file.path)}</div><pre>${escapeHtml(file.contents)}</pre>`;
    panel.appendChild(div);
  }
}

function renderDetail() {
  const el = $("detail");
  if (!selected) {
    el.innerHTML = `<p class="empty">No intent. Compile a canonical sentence.</p>`;
    return;
  }

  const sim = selected.simulation;
  const tokenState = selected.token
    ? selected.token.revoked
      ? "revoked"
      : selected.token.used
        ? "spent"
        : "live"
    : "—";
  const preview = (sim?.effects_preview ?? [])
    .map((e) => {
      if (e.kind === "denied") {
        return `<div class="deny">DENY ${escapeHtml(e.capability)} ${escapeHtml(e.path)} · ${escapeHtml(e.reason)}</div>`;
      }
      if (e.kind === "echo") {
        return `<div class="preview">echo ${escapeHtml(e.text)}</div>`;
      }
      if (e.kind === "fetch") {
        return `<div class="preview">fetch ${escapeHtml(e.url)}</div>`;
      }
      if (e.kind === "read" || e.kind === "list") {
        return `<div class="preview">${escapeHtml(e.kind)} ${escapeHtml(e.path)}</div>`;
      }
      return `<div class="preview">${escapeHtml(e.kind)} ${escapeHtml(e.path)} (${e.bytes} bytes)</div>`;
    })
    .join("");
  const events = listEvents(selected.id)
    .map(
      (ev) =>
        `<li><span>${escapeHtml(ev.kind)}</span><span>${new Date(ev.created_at).toLocaleTimeString()}</span></li>`,
    )
    .join("");

  el.innerHTML = `
    <div class="row" style="justify-content:space-between;align-items:center">
      <strong>${escapeHtml(selected.name)}</strong>
      <span>${escapeHtml(selected.status)}</span>
    </div>
    <dl class="kv">
      <dt>intent</dt><dd>${escapeHtml(selected.id)}</dd>
      <dt>capability</dt><dd>${escapeHtml(selected.ir.tasks[0]?.capabilities[0] ?? "")}</dd>
      <dt>requires approval</dt><dd>${selected.ir.requires_approval ? "yes" : "no"}</dd>
      <dt>token</dt><dd>${escapeHtml(tokenState)}</dd>
    </dl>
    <div class="row">
      <button id="a-sim">Simulate</button>
      <button id="a-appr">Approve</button>
      <button class="primary" id="a-exec">Execute</button>
      <button id="a-shadow">Shadow execute</button>
      <button id="a-rev">Revoke</button>
      <button id="a-rep">Replay log</button>
    </div>
    ${sim ? `<div class="preview">risk ${sim.risk} · success ${Math.round(sim.success_probability * 100)}% · ${sim.duration_ms}ms</div>` : "<p class=\"empty\">No simulation yet.</p>"}
    ${preview}
    ${selected.output != null ? `<div class="preview">output: ${escapeHtml(selected.output)}</div>` : ""}
    ${folded ? `<div class="preview">folded status=${escapeHtml(folded.status)} match=${folded.matches ? "yes" : "no"} events=${folded.events}</div>` : ""}
    <details><summary>Intent IR</summary><pre>${escapeHtml(JSON.stringify(selected.ir, null, 2))}</pre></details>
    <h2>Event log</h2>
    <ul class="events">${events || "<li><span>none</span><span></span></li>"}</ul>
  `;

  $("a-sim").onclick = () => {
    const r = simulate(selected.id);
    if (!r.ok) return toast(r.error);
    selected = r.intent;
    toast("Simulated");
    render();
  };

  $("a-appr").onclick = async () => {
    const r = await approve(selected.id);
    if (!r.ok) return toast(r.error);
    selected = r.intent;
    toast("Approved");
    render();
  };

  $("a-exec").onclick = async () => {
    const r = await execute(selected.id, false);
    if (!r.ok) return toast(r.error);
    selected = r.intent;
    toast("Executed");
    render();
  };

  $("a-shadow").onclick = async () => {
    const before = selected.status;
    const usedBefore = selected.token?.used;
    const r = await execute(selected.id, true);
    if (!r.ok) return toast(r.error);
    selected = r.intent;
    if (selected.status !== before || selected.token?.used !== usedBefore) {
      toast("Shadow run changed state unexpectedly");
    } else {
      toast("Shadow executed");
    }
    render();
  };

  $("a-rev").onclick = () => {
    const r = revoke(selected.id);
    if (!r.ok) return toast(r.error);
    selected = r.intent;
    toast("Token revoked");
    render();
  };

  $("a-rep").onclick = () => {
    const r = replay(selected.id);
    if (!r.ok) return toast(r.error);
    folded = {
      status: r.folded.status,
      matches: r.matches_store,
      events: r.event_count,
    };
    renderDetail();
    toast(r.matches_store ? "Replay matches store" : "Replay diverged");
  };
}

function render() {
  if (selected) {
    selected = listIntents().find((i) => i.id === selected.id) ?? selected;
  }
  renderPolicyLint();
  renderMeter();
  renderList();
  renderDetail();
}

$("policy").addEventListener("input", renderPolicyLint);

$("btn-compile").onclick = () => {
  const result = compile($("nl").value);
  if (!result.ok) return toast(result.error);
  selected = result.intent;
  folded = null;
  toast("Compiled");
  render();
};

$("btn-propose").onclick = () => {
  const result = propose($("nl").value);
  if (!result.ok) return toast(result.error);
  $("nl").value = result.canonical;
  selected = result.intent;
  folded = null;
  toast(`Proposed via ${result.proposer}`);
  render();
};

render();
