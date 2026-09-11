import { createFileRoute, Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { toast, Toaster } from "sonner";
import {
  ChevronRight,
  CircleAlert,
  Play,
  Shield,
  Terminal,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { UserButton } from "@/lib/auth/gates";
import { useCurrentUserState } from "@/lib/auth/use-current-user";
import {
  approveIntent,
  compileIntent,
  executeIntent,
  listEvents,
  listIntents,
  listScratchFiles,
  proposeIntent,
  replayIntent,
  revokeToken,
  simulateIntent,
} from "@/lib/imperium/actions";
import type { EventRecord, IntentRecord } from "@/lib/imperium/types";
import { cn } from "@/lib/utils";

export const Route = createFileRoute("/")({ component: Home });

function Home() {
  const { user, isPending } = useCurrentUserState();
  if (user) return <Workbench />;
  return <Landing loading={isPending} />;
}

function Landing({ loading }: { loading: boolean }) {
  return (
    <main className="min-h-screen bg-bg px-5 py-16 text-fg">
      <div className="mx-auto max-w-xl space-y-6">
        <p className="font-mono text-xs tracking-[0.22em] text-subtle uppercase">
          Phase 6 — event store is truth
        </p>
        <h1 className="text-4xl font-medium tracking-tight">IMPERIUM</h1>
        <p className="text-muted">
          Replay folds the event log. If it matches the snapshot, the store is
          consistent. Simulation notes come from that log, not decoration.
        </p>
        {loading ? (
          <p className="text-sm text-subtle">Checking session…</p>
        ) : (
          <Button asChild>
            <Link to="/login">Sign in</Link>
          </Button>
        )}
      </div>
    </main>
  );
}

const STEPS = ["compile", "simulate", "approve", "execute"] as const;

function Workbench() {
  const [nl, setNl] = useState("Echo this message: ping");
  const [intents, setIntents] = useState<IntentRecord[]>([]);
  const [selected, setSelected] = useState<IntentRecord | null>(null);
  const [events, setEvents] = useState<EventRecord[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [folded, setFolded] = useState<{
    status: string;
    output: string | null;
    proposer: string | null;
    canonical: string | null;
    fail_reason: string | null;
    matches: boolean;
    events: number;
  } | null>(null);
  const [scratch, setScratch] = useState<
    Array<{ path: string; contents: string; updated_at: string }>
  >([]);

  async function refreshList() {
    const rows = await listIntents();
    setIntents(rows);
    const files = await listScratchFiles();
    setScratch(files);
    return rows;
  }

  async function select(intent: IntentRecord) {
    setSelected(intent);
    setFolded(null);
    const ev = await listEvents({ data: intent.id });
    setEvents(ev);
  }

  useEffect(() => {
    refreshList().catch(() => setIntents([]));
  }, []);

  async function run(label: string, fn: () => Promise<void>) {
    setBusy(label);
    try {
      await fn();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Request failed";
      if (message === "Unauthorized") {
        toast.error("Sign in required");
      } else {
        toast.error(message);
      }
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="min-h-screen bg-bg text-fg">
      <Toaster theme="dark" position="bottom-right" />
      <header className="border-b border-border">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-5 py-4">
          <div>
            <p className="font-mono text-[11px] tracking-[0.22em] text-subtle uppercase">
              IMPERIUM
            </p>
            <p className="text-sm text-muted">
              Intent runtime · event-sourced replay
            </p>
          </div>
          <UserButton />
        </div>
      </header>

      <main className="mx-auto grid max-w-6xl gap-6 px-5 py-6 lg:grid-cols-[minmax(0,1fr)_280px]">
        <div className="space-y-5">
          <Card className="space-y-4">
            <div className="flex items-center justify-between gap-3">
              <h1 className="text-lg font-medium">Compose</h1>
              <span className="font-mono text-xs text-subtle">
                propose · rules · wasm
              </span>
            </div>
            <label className="block space-y-2">
              <span className="text-xs font-medium text-muted">
                Natural language
              </span>
              <textarea
                value={nl}
                onChange={(e) => setNl(e.target.value)}
                rows={3}
                className="w-full min-h-24 resize-y rounded-[var(--radius-md)] border border-border bg-elevated px-3 py-2 font-mono text-sm text-fg outline-none focus:ring-2 focus:ring-accent/40"
              />
            </label>
            <p className="text-xs text-subtle">
              Strict: Echo this message: {"<text>"} · Write file {"<path>"} with
              contents {"<text>"}
              <br />
              Propose also accepts looser phrasing like “say hello” or “save
              notes.txt with hi”.
            </p>
            <div className="flex flex-wrap gap-2">
              <Button
                disabled={busy !== null}
                onClick={() =>
                  run("compile", async () => {
                    const res = await compileIntent({ data: nl });
                    if (!res.ok) {
                      toast.error(res.error);
                      return;
                    }
                    toast.success("Compiled");
                    await refreshList();
                    if (res.intent) await select(res.intent);
                  })
                }
              >
                <Terminal className="size-4" />
                Compile
              </Button>
              <Button
                variant="secondary"
                disabled={busy !== null}
                onClick={() =>
                  run("propose", async () => {
                    const res = await proposeIntent({ data: nl });
                    if (!res.ok) {
                      toast.error(res.error);
                      return;
                    }
                    toast.success(`Proposed via ${res.proposer}`);
                    setNl(res.canonical);
                    await refreshList();
                    if (res.intent) await select(res.intent);
                  })
                }
              >
                Propose
              </Button>
            </div>
          </Card>

          {selected ? (
            <Card className="space-y-5">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <h2 className="text-lg font-medium">{selected.name}</h2>
                  <p className="font-mono text-xs text-subtle">{selected.id}</p>
                </div>
                <StatusChip status={selected.status} />
              </div>

              <StepRail status={selected.status} />

              <div className="flex flex-wrap gap-2">
                <Button
                  variant="secondary"
                  disabled={busy !== null}
                  onClick={() =>
                    run("simulate", async () => {
                      const res = await simulateIntent({ data: selected.id });
                      if (!res.ok) {
                        toast.error(res.error);
                        return;
                      }
                      toast.success("Simulated");
                      await refreshList();
                      if (res.intent) await select(res.intent);
                    })
                  }
                >
                  Simulate
                </Button>
                <Button
                  variant="secondary"
                  disabled={busy !== null}
                  onClick={() =>
                    run("approve", async () => {
                      const res = await approveIntent({ data: selected.id });
                      if (!res.ok) {
                        toast.error(res.error);
                        return;
                      }
                      toast.success("Approved");
                      await refreshList();
                      if (res.intent) await select(res.intent);
                    })
                  }
                >
                  <Shield className="size-4" />
                  Approve
                </Button>
                <Button
                  disabled={busy !== null}
                  onClick={() =>
                    run("execute", async () => {
                      const res = await executeIntent({ data: selected.id });
                      if (!res.ok) {
                        toast.error(res.error);
                        return;
                      }
                      toast.success("Executed");
                      await refreshList();
                      if (res.intent) await select(res.intent);
                    })
                  }
                >
                  <Play className="size-4" />
                  Execute
                </Button>
                <Button
                  variant="ghost"
                  disabled={
                    busy !== null || !selected.token || selected.token.revoked
                  }
                  onClick={() =>
                    run("revoke", async () => {
                      const res = await revokeToken({ data: selected.id });
                      if (!res.ok) {
                        toast.error(res.error);
                        return;
                      }
                      toast.success("Token revoked");
                      await refreshList();
                      if (res.intent) await select(res.intent);
                    })
                  }
                >
                  Revoke token
                </Button>
                <Button
                  variant="ghost"
                  disabled={busy !== null}
                  onClick={() =>
                    run("replay", async () => {
                      const res = await replayIntent({ data: selected.id });
                      if (!res.ok) {
                        toast.error(res.error);
                        return;
                      }
                      setFolded({
                        status: res.folded.status,
                        output: res.folded.output,
                        proposer: res.folded.proposer,
                        canonical: res.folded.canonical,
                        fail_reason: res.folded.fail_reason,
                        matches: res.matches_store,
                        events: res.event_count,
                      });
                      const ev = await listEvents({ data: selected.id });
                      setEvents(ev);
                    })
                  }
                >
                  Replay log
                </Button>
              </div>

              {selected.simulation ? (
                <div className="space-y-3">
                  <dl className="grid grid-cols-2 gap-3 text-sm md:grid-cols-4">
                    <Stat
                      label="Success"
                      value={`${Math.round(selected.simulation.success_probability * 100)}%`}
                    />
                    <Stat
                      label="Risk"
                      value={String(selected.simulation.risk)}
                    />
                    <Stat
                      label="Duration"
                      value={`${selected.simulation.duration_ms} ms`}
                    />
                    <Stat
                      label="Capability"
                      value={selected.ir.tasks[0]?.capabilities[0] ?? "—"}
                    />
                  </dl>
                  {selected.simulation.notes.length > 0 ? (
                    <ul className="space-y-1 text-sm text-muted">
                      {selected.simulation.notes.map((note) => (
                        <li key={note}>{note}</li>
                      ))}
                    </ul>
                  ) : null}
                </div>
              ) : (
                <p className="text-sm text-muted">
                  No simulation yet. Run Simulate before Approve.
                </p>
              )}

              {selected.token ? (
                <div className="space-y-2 rounded-[var(--radius-md)] border border-border bg-elevated px-3 py-3">
                  <p className="text-xs text-subtle">Capability token</p>
                  <dl className="grid grid-cols-2 gap-2 text-sm">
                    <Stat
                      label="Fingerprint"
                      value={selected.token.fingerprint}
                    />
                    <Stat
                      label="State"
                      value={
                        selected.token.revoked
                          ? "revoked"
                          : selected.token.used
                            ? "spent"
                            : "live"
                      }
                    />
                    <Stat
                      label="Expires"
                      value={new Date(
                        selected.token.expires_at,
                      ).toLocaleTimeString()}
                    />
                    <Stat
                      label="Net / FS / Env"
                      value={`${selected.token.permissions.net.length}/${selected.token.permissions.fs.length}/${selected.token.permissions.env.length}`}
                    />
                  </dl>
                  <p className="text-xs text-subtle">
                    Fail-closed: empty signature, expiry, nonce reuse, revoke,
                    unknown capability, any network host.
                  </p>
                </div>
              ) : (
                <p className="text-sm text-muted">
                  Approve issues a one-shot HMAC token. Execute verifies it.
                </p>
              )}

              {selected.output != null ? (
                <div className="rounded-[var(--radius-md)] border border-border bg-elevated px-3 py-3">
                  <p className="text-xs text-subtle">Capability output</p>
                  <p className="font-mono text-base">{selected.output}</p>
                </div>
              ) : null}

              <div className="space-y-1 rounded-[var(--radius-md)] border border-border bg-elevated px-3 py-3">
                <p className="text-xs text-subtle">Source</p>
                <p className="font-mono text-sm">{selected.nl_source}</p>
                <p className="text-xs text-subtle">
                  {selected.ir.compiler_version}
                </p>
              </div>

              {folded ? (
                <div className="space-y-2 rounded-[var(--radius-md)] border border-border bg-elevated px-3 py-3">
                  <p className="text-xs text-subtle">Folded event log</p>
                  <dl className="grid grid-cols-2 gap-2 text-sm">
                    <Stat label="Folded status" value={folded.status} />
                    <Stat
                      label="Match"
                      value={folded.matches ? "yes" : "diverged"}
                    />
                    <Stat label="Events" value={String(folded.events)} />
                    <Stat label="Proposer" value={folded.proposer || "rules"} />
                  </dl>
                  {folded.output ? (
                    <p className="font-mono text-sm">output: {folded.output}</p>
                  ) : null}
                  {folded.fail_reason ? (
                    <p className="text-sm text-bad">fail: {folded.fail_reason}</p>
                  ) : null}
                  {folded.canonical ? (
                    <p className="font-mono text-xs text-muted">
                      {folded.canonical}
                    </p>
                  ) : null}
                </div>
              ) : null}

              <details className="rounded-[var(--radius-md)] border border-border bg-elevated px-3 py-2">
                <summary className="cursor-pointer text-sm text-muted">
                  Intent IR
                </summary>
                <pre className="mt-2 overflow-x-auto font-mono text-xs text-fg">
                  {JSON.stringify(selected.ir, null, 2)}
                </pre>
              </details>

              <div>
                <h3 className="mb-2 text-sm font-medium">Event log</h3>
                {events.length === 0 ? (
                  <p className="text-sm text-muted">No events.</p>
                ) : (
                  <ol className="space-y-2">
                    {events.map((ev) => (
                      <li
                        key={ev.id}
                        className="flex flex-col gap-0.5 font-mono text-xs text-muted"
                      >
                        <span className="flex items-start gap-2">
                          <ChevronRight className="mt-0.5 size-3 shrink-0" />
                          <span className="text-fg">{ev.kind}</span>
                          <span className="text-subtle">
                            {new Date(ev.created_at).toLocaleTimeString()}
                          </span>
                        </span>
                        {Object.keys(ev.payload).length > 0 ? (
                          <span className="pl-5 text-subtle">
                            {Object.entries(ev.payload)
                              .slice(0, 4)
                              .map(([k, v]) => `${k}=${String(v)}`)
                              .join(" · ")}
                          </span>
                        ) : null}
                      </li>
                    ))}
                  </ol>
                )}
              </div>
            </Card>
          ) : (
            <Card className="flex items-start gap-3 text-sm text-muted">
              <CircleAlert className="mt-0.5 size-4 shrink-0" />
              Compile an echo or write intent. Execute is rejected until Approve.
            </Card>
          )}
        </div>

        <aside className="space-y-6">
          <div className="space-y-3">
            <h2 className="text-sm font-medium text-muted">Intents</h2>
            {intents.length === 0 ? (
              <p className="text-sm text-subtle">None yet.</p>
            ) : (
              <ul className="space-y-2">
                {intents.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      onClick={() => select(item)}
                      className={cn(
                        "min-h-11 w-full rounded-[var(--radius-md)] border px-3 py-3 text-left transition-colors",
                        selected?.id === item.id
                          ? "border-accent/40 bg-elevated"
                          : "border-border bg-surface hover:bg-elevated",
                      )}
                    >
                      <p className="truncate text-sm">{item.name}</p>
                      <p className="font-mono text-[11px] text-subtle">
                        {item.status}
                      </p>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
          <div className="space-y-3">
            <h2 className="text-sm font-medium text-muted">Scratch</h2>
            {scratch.length === 0 ? (
              <p className="text-sm text-subtle">
                Empty. Write file hello.txt with contents hi
              </p>
            ) : (
              <ul className="space-y-2">
                {scratch.map((file) => (
                  <li
                    key={file.path}
                    className="rounded-[var(--radius-md)] border border-border bg-surface px-3 py-3"
                  >
                    <p className="truncate font-mono text-xs text-fg">
                      {file.path}
                    </p>
                    <p className="mt-1 truncate text-sm text-muted">
                      {file.contents}
                    </p>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </aside>
      </main>
    </div>
  );
}

function StatusChip({ status }: { status: IntentRecord["status"] }) {
  return (
    <span className="rounded-full border border-border px-2.5 py-1 font-mono text-[11px] tracking-wide text-muted uppercase">
      {status}
    </span>
  );
}

function StepRail({ status }: { status: IntentRecord["status"] }) {
  const order = ["compiled", "simulated", "approved", "executed"] as const;
  const idx = Math.max(
    0,
    order.indexOf(
      status === "failed" ? "compiled" : (status as (typeof order)[number]),
    ),
  );
  return (
    <ol className="grid grid-cols-4 gap-2">
      {STEPS.map((step, i) => (
        <li
          key={step}
          className={cn(
            "rounded-[var(--radius-sm)] border px-2 py-2 text-center font-mono text-[10px] tracking-wide uppercase",
            i <= idx
              ? "border-accent/30 bg-elevated text-fg"
              : "border-border text-subtle",
          )}
        >
          {step}
        </li>
      ))}
    </ol>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-[var(--radius-md)] border border-border bg-elevated px-3 py-2">
      <dt className="text-xs text-subtle">{label}</dt>
      <dd className="font-mono text-sm tabular-nums">{value}</dd>
    </div>
  );
}
